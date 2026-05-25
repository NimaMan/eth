# Token Lab Categories

Token Lab categories classify concrete token/pool cases. They are scoped to
case evidence, not to reusable methodology. Cross-case rules belong in
`../../investigations/`.

Every case with `investigation.toml` should set one of these categories.

| Category | Folder | Case Count |
| --- | --- | --- |
| `source_simulator_parity` | `source_simulator_parity/` | 3 current cases |
| `mechanism_classification` | `mechanism_classification/` | 3 current cases |
| `display_read_model` | `display_read_model/` | 1 current case |
| `execution_viability` | `execution_viability/` | 2 current cases |

Cases without `investigation.toml` should still be placed in the closest
category index once their category is known.

Strategy-facing interpretation of validated facts belongs in
`../../../alpha/lab/strategy_analysis/risk_atlas_inputs/`, not as a Token Lab
category.

Position, PnL, and strategy-result validation belongs in
`../../../alpha/lab/strategy_analysis/backtest_validity/`.
