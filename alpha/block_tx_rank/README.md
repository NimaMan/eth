# block_tx_rank

Crate: `eth_block_tx_rank`

`block_tx_rank` is the mined-block evidence layer for Alpha gas-rank selection.
It answers one question before Alpha builds a real transaction:

```text
Given this route gas estimate and next-block base-fee forecast, what priority
fee candidates would likely land early enough based on recent mined blocks?
```

The crate does not choose whether a strategy should trade. It produces rank
evidence that Alpha can combine with exact route simulation, strategy urgency,
and value caps before submitting through Kartal.

## Bribe Definition

For Alpha direct-raw execution, `bribe` means the EIP-1559 priority fee paid
through `max_priority_fee_per_gas`. The base fee is mandatory execution cost,
not bribe. Direct `block.coinbase` transfers, bundles, and private relay
payments are not part of the current `eth_direct_raw_v1` path.

Kartal and `tx_executor` do not discover the bribe. Alpha selects the priority
fee first, includes it in the transaction request, and also mirrors it in the
request `bribe` object for auditability.

## Owns

- Sampling recent mined transactions and their effective priority fees.
- Estimating candidate rank position and gas-before-us from recent blocks.
- Producing named rank candidates such as `normal`, `p50`, `p75`, `p85`, `p90`,
  and `p95`.
- Returning source-window evidence that can be persisted with the Alpha order
  decision.

## Does Not Own

- Pending mempool ingestion or pending transaction simulation.
- Dependency-relative `mempool_race` fee construction.
- AMM route construction, quotes, slippage, or pool policy.
- Strategy approval, capital sizing, or exit eligibility.
- Signing, nonce reservation, broadcast, or receipt reconciliation.

## Integration

The production live path consumes this evidence through the ETH chain server
gas-rank endpoint:

```text
Alpha live strategy decision
  -> exact deployed V2 vault simulation
  -> alpha/live/trading ChainServerGasRankProvider
  -> /api/v1/eth/alpha/gas-rank/estimate
  -> eth_block_tx_rank mined-block rank evidence
  -> alpha/live/trading tx_prep value-cap and source gate
  -> Kartal direct-raw request
  -> tx_executor validation/sign/dry-run-or-broadcast
```

`tx_executor` remains a submission boundary. It receives only a final prepared
transaction with explicit `max_fee_per_gas`, `max_priority_fee_per_gas`, and
audit metadata.

Current live Alpha config requires candidates from source
`eth_chain_server_gas_rank`. The live gas-rank lookback is `100` blocks and is
clamped to the supported `1..100` range by the planner. The service adds
`ALPHA_GAS_RANK_PRIORITY_TIE_BREAKER_GWEI` to raw sampled values so Alpha does
not submit at common whole-gwei priority buckets.

## Candidate Shape

Each ranked candidate should carry enough evidence to explain both selected and
rejected gas decisions:

```text
label
priority_fee_gwei
max_fee_per_gas_gwei
rank_position_p50
gas_before_p50
likely_fits_at_p50
source
source window / sample metadata
```

`P50`, `P75`, `P85`, `P90`, and `P95` are mined priority-fee percentile bands.
They are not transaction positions. The separate `normal` profile is a low-cost
rank-target fallback, currently sampled from the priority fee needed to beat
roughly transaction position 50.

## Current Live Ladders

Alpha chooses a ladder from strategy context before applying the value cap:

| Transaction kind | Gas-rank ladder |
| --- | --- |
| Entry buy | `p85 -> p75 -> p50 -> normal` |
| Routine strategy exit | `p85 -> p75 -> p50 -> normal` |
| Mined LP approval exit | `p90 -> p75 -> p50 -> normal` |
| Buy-confirm-block LP approval exit | `p90 -> p75 -> p50 -> normal` |
| Mempool LP approval or liquidity-removal race | `mempool_race` |

`mempool_race` is intentionally not produced by `block_tx_rank`. It is built in
`alpha/live/trading` from the triggering pending transaction:

```text
dependency_effective_priority_fee
  + deterministic per-tx buffer, currently 0.1..0.2 gwei
  + 1 wei
  = selected priority fee
```

That candidate uses source `eth_public_mempool_pending_tx_fee` and includes the
dependency tx hash, dependency gas fields, dependency effective priority fee,
selected priority fee, and predicted base fee in metadata.

## Value Cap

Rank evidence is advisory. `alpha/live/trading::tx_prep` remains the final
decider and rejects any candidate that does not fit the protected value:

```text
avoidable_loss = protected_exit_value
               - expected_late_recovery
               - safety_buffer

max_total_fee = min(operator_total_fee_cap, avoidable_loss)
estimated_base_fee_cost = predicted_base_fee_gwei * estimated_gas_used
max_priority_spend = max_total_fee - estimated_base_fee_cost
max_priority_fee_gwei = min(value_capped_priority_fee,
                            configured_max_priority_fee_gwei)
```

A candidate is eligible only if both its priority spend and total max-fee spend
fit the budget. Among eligible candidates, Alpha prefers the best expected rank,
then the higher priority fee when ranks tie. If no ranked candidate fits, Alpha
rejects with `gas_rank_exceeds_value_cap`. It must not synthesize an unranked
value-cap fallback.

The current live-real priority cap is `3.5 gwei`; entry and exit total gas fee
caps are owned by `alpha/live/trading` config and documented there.

## Source Gate

Production callers can require a gas-rank source. The current real runner
requires `eth_chain_server_gas_rank`, so fixed or test candidates cannot cross
the live execution boundary. The only exception is a mempool pre-mine LP/removal
race, where `mempool_race` candidates from
`eth_public_mempool_pending_tx_fee` are allowed because they are
dependency-relative rather than mined-block-rank candidates.

If no candidate survives the source gate, Alpha rejects with
`gas_rank_source_not_allowed`.

## Persistence Contract

Submitted requests carry the selected gas-rank decision in metadata:

- selected label and profile ladder;
- priority fee and max fee;
- estimated priority spend and max cost;
- rank position and gas-before estimate when available;
- candidate source and source metadata;
- predicted base fee, protected value, late recovery, safety buffer, and
  estimated gas used.

Reject outcomes carry the reject reason, required source, candidate list,
strategy gas-rank policy, and value-cap budget. Callers should persist that
metadata with the order or execution attempt so later analysis can compare
selected, rejected, and mined outcomes.

## Related Docs

- `../live/trading/README.md` documents how the live planner consumes gas-rank
  evidence, applies exact deployed V2 vault simulation, and submits through
  Kartal.
- `../live/readiness/gates/production_gas_rank/` tracks the production readiness
  gate for mined gas-rank evidence.

## Tests

```bash
cargo test -p eth_block_tx_rank
```
