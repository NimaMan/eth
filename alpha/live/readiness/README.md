# Live Readiness

`alpha/live/readiness/` contains the repeatable gates that must pass before a
live strategy can use public real capital. These checks are operational and
evidence-driven, so they live outside individual crate READMEs.

Backtests and live backtests can prove strategy logic, simulation assumptions,
and accounting consistency. They cannot prove signing, broadcast, mining,
receipt reconciliation, nonce behavior, or deployed-vault event handling. Those
chain-facing assumptions need explicit readiness gates.

## Rule

No live strategy should be promoted from dry-run or chain-sim validation to
public real capital unless it has:

1. a strategy-specific readiness file under `strategies/`;
2. a short-hold, one-entry mined-validation run or a documented reason why that
   gate does not apply;
3. archived evidence for every pass or failure; and
4. reviewed follow-on risks before increasing bankroll or concurrent positions.

## Layout

| Path | Purpose |
| --- | --- |
| `gates/pre_live_mined_validation/` | Reusable one-position mined-validation gate. |
| `gates/production_gas_rank/` | Gas-rank, fee-cap, and production transaction-cost gate. |
| `strategies/alpha11/` | Alpha11-specific application of the readiness gates. |

Gate definitions describe what must be proven for any strategy. Strategy files
bind those gates to concrete strategy names, caps, vaults, and current evidence.
