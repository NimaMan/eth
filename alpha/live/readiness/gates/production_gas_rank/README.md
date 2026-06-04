# Production Gas-Rank Readiness

This gate exists because the first public validation trade used a temporary
fixed gas envelope around a 40 gwei priority fee. That proved the pipeline, but
it is not acceptable for production trading.

## Required Rule

No real Alpha live transaction may be submitted unless all of these are true:

- Exact deployed-vault calldata simulation succeeds and reports `gas_used`.
- The route gas estimate is derived from the simulation result, not a fallback.
- Gas candidates come from `eth_chain_server_gas_rank`.
- Entry buys use the `p85 -> p75 -> p50 -> normal` profile ladder.
- Routine strategy exits such as max-hold use `p85 -> p75 -> p50 -> normal`.
- LP/risk race exits may request the `p95 -> p90 -> p75 -> p50 -> normal`
  ladder, but still must fit the
  production caps below.
- `p50`, `p75`, `p85`, `p90`, and `p95` are priority-fee percentile profiles from the
  mined sample. They are not transaction-position targets.
- `normal` is the separate rank-target fallback around transaction position 50.
- The gas-rank service adds `ALPHA_GAS_RANK_PRIORITY_TIE_BREAKER_GWEI` to each
  recommendation before it can become a submit candidate, so common values like
  exactly `2 gwei` become distinct ranked fees such as `2.1456 gwei`.
- Selected priority fee is `<= 3.5 gwei`.
- Entry estimated gas fee is `<= 0.0012 ETH`.
- Exit estimated gas fee is `<= 0.002 ETH`.
- The selected gas plan, source, profile, priority fee, max fee, simulation
  block, and gas estimate are persisted in execution metadata.

## Current Evidence

Checked against the live `eth_chain_server` gas-rank endpoint on
2026-05-22 using the mined hold3 validation trade gas shape:

| Case | Gas Limit | Estimated Gas Used | Selected Profile | Priority | Estimated Total Gas |
| --- | ---: | ---: | --- | ---: | ---: |
| Buy-sized route | 300000 | 221135 | P50 | 2.000000001 gwei | 0.0004681073 ETH |
| Sell-sized route | 300000 | 238622 | P50 | 2.000000001 gwei | 0.0005051178 ETH |

The same sample had `P95` at `5.000000001 gwei`, which exceeds the
production cap and must not be selected for the normal Alpha11 validation path.

## Simulated Gas Buffer Calibration

The first public hold3 validation run did not persist pre-submit simulation
metadata, but it did persist mined receipt gas:

| Side | Mined gas used | 50% buffered | 25% buffered |
| --- | ---: | ---: | ---: |
| Buy | 176908 | 265362 | 221135 |
| Sell | 190897 | 286346 | 238622 |

A later chain-sim run for the same token/pool persisted the gas-policy guard.
It recorded `chain_sim_estimated_gas_used=237212` for buys with simulated
receipt gas `158141`, and `chain_sim_estimated_gas_used=285914` for sells with
simulated receipt gas `190609`; both are exactly the old 50% buffer.

The configured buffer is now `2500 bps`. Against that same chain-sim evidence it
would estimate `197677` gas for the buy and `238262` gas for the sell. Those
values still cover the public mined receipts by `20769` gas on the buy and
`47365` gas on the sell, while cutting budget overstatement in half.

## Implementation State

- Real buy planning filters candidates by source, priority cap, and estimated
  gas-fee cap before building the ETH tx executor request.
- Real sell tx-prep can require a gas-rank source; the production live-real
  adapter sets this to `eth_chain_server_gas_rank`.
- Max-hold exits are classified as normal strategy exits, not mempool
  races.
- Fixed gas-rank providers remain available only for tests and calibration
  fixtures.

## Still Required Before More Public Capital

- Lower ETH tx executor and signer hard caps so the last line of defense is not much
  looser than the Alpha gas policy.
- Run one ETH tx executor dry-run using the production gas-rank path and confirm the
  metadata shows `source=eth_chain_server_gas_rank`, `label=p50`, and
  priority below the cap.
- Re-run the hold3 public validation only after the dry-run metadata check
  passes.
