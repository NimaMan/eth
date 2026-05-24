# Mempool Overlay Entry

## Objective

Use mempool simulation as the source of projected entry facts for a live buy:

1. receive a `trading_enabled` mempool signal;
2. consume the projected pool/entry facts already computed by
   `mempool_processor`;
3. run the strategy entry gates against that projected evidence package;
4. require exact deployed V2 vault buy simulation evidence from the same
   mempool overlay state;
5. submit with a gas policy that intentionally trails the dependency tx.

This is different from the current mined-state path. The current path treats the
mempool signal as early evidence, then waits until the local Reth state already
contains the pool/trading-enabled state. If the pair or can-buy state only
exists inside the pending tx, the current path correctly defers instead of
submitting.

## Current Data Path

`mempool_processor` receives full pending tx JSON from local Reth IPC:

```text
Reth IPC newPendingTransactions(true)
  -> MempoolTransaction { hash, data, from, to, input, value, gas, fees, nonce }
  -> simulation manager
  -> signal detector
  -> live_trading.signal_events
  -> eth_chain_server mempool signal API
  -> eth_alpha_trader
```

The detector already uses pending transaction replay internally:

- `PendingSequences` keeps a short in-memory `(creator, token)` helper
  sequence, limited to six transactions and two seconds.
- nonce/funding dependencies are replayed before the target tx when needed.
- pool buy/approve/sell probes receive `prior_txs` from the replay sequence.
- if the probe succeeds, a `trading_enabled` semantic signal is emitted.

The persistence/API boundary is the gap. `live_trading.signal_events` currently
stores the semantic signal plus `pending_tx_hash`; it does not persist the
projected pool facts, exact vault-buy simulation proof, dependency fee metadata,
or the dependency sequence audit trail that made the signal true.

## Required Implementation

### 1. Persist projected entry evidence with the signal

Add a public signal attachment table, for example:

```sql
CREATE TABLE live_trading.signal_entry_evidence (
    signal_id BIGINT NOT NULL REFERENCES live_trading.signal_events(signal_id) ON DELETE CASCADE,
    evidence_version TEXT NOT NULL,
    base_block BIGINT NOT NULL,
    simulated_at TIMESTAMPTZ NOT NULL,
    dependency_tx_hashes TEXT[] NOT NULL,
    dependency_fee_metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    projected_pool JSONB NOT NULL,
    viability JSONB NOT NULL,
    vault_buy_simulation JSONB NOT NULL,
    strategy_neutral_flags JSONB NOT NULL DEFAULT '{}'::jsonb,
    audit JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

For trading-enabled signals this row should contain the neutral facts alpha
needs for entry:

- projected reserves/liquidity and price-to-initial ratio;
- buy/approve/sell viability and tax values;
- exact configured V2 vault-buy calldata simulation output, gas used, and
  revert status;
- dependency tx hashes and fee metadata used by the tail gas policy;
- base block, simulation time, and freshness metadata.

The raw pending tx sequence does not need to be sent to alpha in the normal
decision path. It should remain available as an internal audit/replay artifact,
keyed by `signal_id`, so we can reproduce disagreements after the fact without
making alpha replay the sequence on every poll.

### 2. Expose entry evidence through chain-server

Extend the mempool signal API response with:

```json
{
  "entry_evidence": {
    "evidence_version": "mempool_entry_evidence_v1",
    "base_block": 25163949,
    "dependency_tx_hashes": ["0x..."],
    "projected_pool": {
      "liquidity_eth": "1.23",
      "price_to_initial": "1.08"
    },
    "viability": {
      "can_buy": true,
      "can_approve": true,
      "can_sell": true,
      "buy_tax_percent": 0.0,
      "sell_tax_percent": 0.0
    },
    "vault_buy_simulation": {
      "route": "uniswap_v2_trading_vault",
      "would_revert": false,
      "gas_used": 176000,
      "eth_spent_wei": "10000000000000000",
      "tokens_received_raw": "..."
    },
    "dependency_fee_metadata": {
      "tail_after_tx_hash": "0x...",
      "dependency_priority_fee_wei": "..."
    }
  }
}
```

`eth_alpha_trader` should reject the mempool-overlay entry path if entry
evidence is missing, stale, from the wrong vault, or from a different strategy
execution route. Falling back to mined-state entry is allowed, but it must
remain a separate decision and should keep using `buy_deferred` while confirmed
state is unavailable.

### 3. Add an entry-evidence consumer in alpha live trading

Add a module under `alpha/live/trading/src/planner/` that:

- validates the evidence version, vault address, base block freshness, buy size,
  and route;
- converts projected facts into the strategy entry context needed by Alpha11;
- applies the same strategy entry rules used for mined-state entry;
- builds the Kartal request from the exact vault calldata and simulation result
  in the evidence package;
- records dependency hashes, projected pool facts, gas used, and freshness in
  the order metadata.

The projected pool snapshot must be explicitly tagged so we do not confuse it
with confirmed token-server state.

### 4. Add a tail-entry gas policy

Map `entry.tail_after_enabling_tx` to a new execution category, for example
`tail_entry_buy`.

For public mempool mode, this policy should target placement after the
dependency transaction:

- read the dependency tx effective priority fee from its tx JSON or RPC;
- choose our priority fee slightly below the dependency priority fee;
- reject if the dependency priority is too low to trail safely without missing
  the block;
- record `tail_after_tx_hash`, target priority, chosen priority, and the
  expected ordering basis in the order metadata.

This is probabilistic in public mempool. Deterministic same-block ordering
requires private relay or bundle support. Public mode must therefore keep the
exact overlay simulation evidence and receipt reconciliation strict.

## Example Signals

Two recent `trading_enabled` signals reproduced through the current detector
replay example:

| Signal | Tx | Block Used | Token | Pool | Result |
| --- | --- | ---: | --- | --- | --- |
| `2004` | `0x6e771f8376501abca08f53120335ef3f798cb690e32ae9ccbfd42c3c47cbee2c` | `25163949` | `0x27a251c9a2669De495399764aD1e13c530405150` | `0x520a40B187BAa890fbdCF44091D530caAB4813F0` | buy/sell true, 0%/0% tax |
| `2001` | `0xf813f4a3e419afd49df209c55907217d2c094625d7d3a84a90cfe9ca3841f8ab` | `25163941` | `0xb94aEa2e3729c03AcF7Af859d96A6b8487d49191` | `0x62B00495457A7FA8a19Fe058F409506371E04e99` | buy/sell true, 0%/0% tax |

These examples prove the detector side can replay the enabling tx and emit the
semantic trading signal. They do not yet prove the live trader can submit a
vault buy after that tx; that requires the dependency persistence/API and alpha
overlay simulator above.
