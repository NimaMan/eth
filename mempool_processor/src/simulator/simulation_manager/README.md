# Simulation Manager

The simulation manager is the bridge between **canonical context** (blocks that
have already been mined and processed by the live token pipeline) and **live
mempool activity** (creator transactions that might toggle trading or taxes
before anyone else notices). Its only job is to take the contextual data
token-server shares with us, re-simulate the mempool transactions against the
latest canonical snapshot, and emit structured `SimulationResult`s that the
signal detectors can reason about.

## Data Sources

| Source | What it contributes | Where it is consumed |
| --- | --- | --- |
| `eth_chain_server` live context | Latest pool inventory, per-token metadata, liquidity snapshots derived from mined blocks, V3 fee tiers, V4 pool display keys | `TokenTrackingCache` (injected into `SimulationManager`) |
| Canonical head listener | `SealedHeader` + DB view for the most recent block | `MempoolSimulator`, `LiquidityRemovalSimulator` |
| Mempool fetcher | Raw transactions plus routing metadata (function detection, category, priority) | `RequestQueue` / flow modules |
| Unresolved intent store | Critical pending txs that arrived before token/pool mapping was available | Retried by the binary and submitted only after mapping exists |

By the time a transaction enters the simulation path we should already know
**which token/pool it touches** (from the cache) and **why we care** (router
category). If the mapping is not available yet, the transaction belongs in the
unresolved-intent lane and should not be submitted as a normal simulation
failure. The manager’s responsibility is to enrich mapped transactions with a
replay sequence, run the appropriate simulator(s), and update the caches when
the result materially changes the token’s state.

## Directory Layout

```
simulation_manager/
├── block_pruner.rs          # Background task that evicts stale replay sequences
├── contract_creation_flow.rs# Handles deployments + metadata lookups
├── creator_buy_sell_flow.rs # Orchestrates creator tx replay, pool viability probes, signals
├── liquidity_removal_flow.rs# Dedicated path for LP decrease/remove functions
├── logging.rs               # Formatting helpers shared across flows
├── manager.rs               # Wiring: owns simulators, cache handles, queue, signal manager
├── mod.rs                   # Module exports
├── pending_sequences.rs     # Stores helper chains keyed by (creator, token)
├── pool_buy_sell_flow.rs    # Token/pool resolution + buy/sell simulator glue
├── request_queue.rs         # Async queue + statistics
├── types.rs                 # Simulation job/result types shared with the rest of the crate
```

## Core Responsibilities

1. **Keep helper sequences fresh**
   `pending_sequences` records the processed transactions that ran just before a
   creator tx. This is what allows buy/sell probes to re-use already-seen state
   (approvals, routing setup, etc.) even before token-server publishes the next
   mined block snapshot. This is intentionally a same-mempool-window cache:
   entries expire after roughly two seconds rather than waiting for future
   blocks.

2. **Preserve sender nonce order**
   `pending_nonce_dependencies` records raw mempool transactions by
   `(sender, nonce)` for a bounded dependency window. When Reth state says the
   target tx nonce is too high, the manager replays the contiguous pending
   prefix first, then the target tx, and only then runs pool probes. This is not
   a signal retry lane; it is execution context needed to mimic chain ordering
   when users submit several transactions before the next block.

3. **Preserve fresh-wallet funding order**
   `pending_funding_dependencies` records visible inbound ETH transfers keyed
   by recipient. When a fresh contract creation fails with `lack of funds` at
   the selected base block, the manager replays the matching funding txs first
   and then the deployment. If the funding tx was private or not visible in our
   public mempool feed, the result is classified as a funding dependency gap
   rather than an actionable simulation error.

4. **Simulate creator activity per pool**
   Every creator transaction is run through the mempool simulator and then
   replayed through `pool_buy_sell_flow` to test each relevant pool reported by
   the token tracker (WETH/token, USDC/token, …). This is how we observe the
   *effect* that an in-flight transaction would have on the pools we plan to
   trade against. V2/Sushi and V3 pools are probed when the cache has complete
   metadata. V4 buy/sell probes remain disabled until the cache exposes the full
   V4 pool-key config required by `tx_processor`.

5. **Track new deployments until token-server catches up**
   `contract_creation_flow` processes deployment transactions, tries to resolve
   the contract address, and records the processed tx under `(creator, token)`.
   When token-server later exposes definitive pool information for that token we
   already have the creator context on our side.

6. **Keep cache waits out of simulation errors**
   LP approvals, liquidity removals, creator-control calls, and V4
   modify-liquidity txs can arrive before token-server has published the mapped
   token/pool. Those txs are retried outside the manager only inside the short
   unresolved-intent window. Inside simulation flows, an
   `unresolved_cache_context` result means "wait for
   context", not "the pool failed".

7. **Surface liquidity threats immediately**
   Liquidity removal signals are handled in their own flow so they can run even
   when buy/sell probes are skipped. V2/Sushi removals use reserve deltas when
   available. V3 removals map processed pool burn/decrease events back to the
   tracked pool. V4 removals map negative `ModifyLiquidity` events back to the
   `pool_manager#pool_id` display key. If reserve impact is not measurable
   before mining, the signal is still emitted as unknown severity instead of
   being mislabeled as low risk.

8. **Feed downstream detectors**
   Every flow ultimately builds a `SimulationResult` and passes it to
   `SignalManager`. The detectors compare the result with the cached “last known”
   token state to decide whether to emit `TradingEnabled`, `HighTax`, or
   `LiquidityRemoval` signals. This is where the objective of “buy immediately
   on trading-enabled, sell immediately on scammy behaviour” becomes actionable.

## Control Flow Overview

```
TxSimulationJob
   │
   ├─ request_queue.submit()                 (enforces back-pressure / stats)
   └─ process_queue() ─┐
                       ▼
                 simulate_request()
                       │
       ┌───────────────┴────────────────────┐
       │                                    │
ContractCreation flow               CreatorTransaction flow
       │                                    │
 record pending sequences         run liquidity removal flow (if relevant)
 metadata lookup via Reth         build replay chain + buy/sell tests
 emit audit debug info            send per-pool SimulationResults to SignalManager
```

Unresolved critical txs do not enter this diagram until token/pool context is
available:

```text
critical tx + cache miss
   -> UnresolvedIntentStore
   -> retry after token cache refresh
   -> TransactionRouter resolves mapped token/pool
   -> request_queue.submit()
```

## How This Supports Our Objective

1. **“Get the latest pool/token info from mined tx”**
   We deliberately treat token-server as the authoritative source for canonical
   pool inventory. The manager never tries to reconstruct pools from scratch; it
   only consumes the cache and keeps short-term helper sequences so we can act
   *before* the next confirmed snapshot arrives.

2. **“Watch new mempool tx and see their effect on the tokens we track”**
   Creator transactions go through `creator_buy_sell_flow`, guaranteeing both
   the base transaction and the derived buy/sell probes are simulated at the tip
   snapshot. That means we know whether trading is enabled (or taxes changed)
   without waiting for the blockchain to mine a block.

3. **“Buy on trading enabled, sell on scams”**
   The SimulationManager itself does not execute trades, but it produces the
   precise signals (`TradingEnabled`, `HighTax`, `LiquidityRemoval`) that the
   trading handlers listen to. Because each signal includes the relevant pool,
   token, and tax numbers, downstream strategies can execute the buy/sell
   playbooks immediately.

## Extending the Module

- **Adding another flow** (e.g., anti-bot protection) should follow the same
  pattern: create `xyz_flow.rs`, expose a method on `SimulationManager`, and
  call it from `simulate_request`.
- **Token cache integration** lives entirely in the manager. If token-server
  starts publishing more metadata (DEX version, fee tiers, etc.), extend the
  cache and flow structs but keep simulators ignorant—they should continue to
  accept resolved pool/token addresses.
- **Telemetry** hooks belong near `request_queue` (ingress stats) and inside the
  flow modules (per outcome). This keeps the manager focused on orchestration
  rather than logging minutiae.

Keeping these boundaries clear ensures the simulations remain aligned with our
trading goals: canonical token-server data defines *what exists*, mempool
simulations predict *what will change*, and signals tell the trading layer when
to act.
