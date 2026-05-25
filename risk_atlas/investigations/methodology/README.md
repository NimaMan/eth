# Investigation Methodology

This folder owns the reusable investigation method for Risk Atlas. It defines
how evidence is promoted from a concrete token/pool case into a detector,
display rule, model feature, or Alpha input.

## Evidence Contract

A complete token/pool case in `../../token_lab/cases/` should explain:

- what the range builder or live system reported;
- what the chain actually did;
- whether the simulator can reproduce the executable route;
- whether a detector or guardrail should exist before trading;
- what code, display, or policy changed because of the investigation.

Case folders normally contain:

- `README.md`: narrative, chain truth, simulator parity, conclusion, and
  follow-up actions;
- `investigation.toml`: machine-readable coordinates and category metadata when
  useful;
- `artifacts/README.md`: instructions for generated artifacts.

Generated artifacts belong under each case's `artifacts/` folder and should not
become frontend, model, or strategy inputs directly. Promote durable facts into
the owning production module first.

## Investigation Categories

| Category | Primary Question | Blocks Promotion? |
| --- | --- | --- |
| `source_simulator_parity` | Do chain truth, token-builder/source observations, simulator route probes, and Alpha executable routes agree? | Yes. This blocks strategy evidence and model data for the affected protocol/cohort. |
| `backtest_result_validity` | Is reported PnL/accounting/lifecycle state correct under the declared execution model? | Yes for the affected run/policy. |
| `mechanism_classification` | What behavior or scam mechanism happened, and are labels precise enough? | Blocks labels/features when unresolved. |
| `display_read_model` | How should Risk Atlas/Asena surface behavior without misleading operators? | Blocks UI trust, not necessarily execution. |
| `execution_viability` | What route, denomination, or size constraint is part of the token/pool execution fact? | Blocks execution claims until the fact is clear. Strategy-policy interpretation belongs in Alpha lab. |
| `network_actor_context` | Do wallet/fund-flow relationships add explanatory or predictive signal? | Research only until promoted into features. |
| `ops_freshness_recheck` | Old evidence may be stale; does it still reproduce on current code? | Blocks only if reproduced. |

## Status Terms

- `new`: captured from the token range builder, UI, or logs.
- `investigating`: chain-truth or simulator parity work has started.
- `confirmed`: the behavior is real and needs a detector, guardrail, or display rule.
- `explained`: the behavior is real but expected, or the display should clarify it.
- `fixed`: code or display logic has been changed and verified.
- `needs_recheck`: old evidence exists, but the issue must be reproduced on the current code before it should be treated as active.
- `ignored`: not useful after review.

## Promotion Rule

Do not use a case as strategy evidence, model data, or operator-facing truth
until its source/simulator/chain parity status is clear. If parity is unclear,
the case belongs in `../promotion_queue/` before it is allowed to influence
policy.

Validated facts that imply a strategy decision should be indexed in
`../../../alpha/lab/strategy_analysis/risk_atlas_inputs/`.
