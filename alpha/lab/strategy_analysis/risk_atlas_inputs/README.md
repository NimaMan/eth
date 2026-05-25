# Risk Atlas Inputs

Validated Risk Atlas token/pool facts that imply Alpha strategy-policy work
live here. Risk Atlas owns the chain evidence; Alpha lab owns the policy
decision, threshold calibration, and strategy variant design.

## Current Inputs

| Source Fact | Risk Atlas Case | Alpha Question |
| --- | --- | --- |
| Stable-denom pools require quote-aware execution and cannot be treated as WETH-equivalent. | `../../../../risk_atlas/token_lab/cases/stable_denom_execution_scope_25065694/` | Which stable-denom routes and liquidity thresholds should an executable strategy support? |
| Smaller WETH entries can execute where fixed `0.01 ETH` entries fail. | `../../../../risk_atlas/token_lab/cases/weth_buy_size_sensitivity_25065694/` | Should strategy sizing be adaptive by pool liquidity, simulator amount sweep, or initial quote depth? |

Do not duplicate chain evidence here. Link to Risk Atlas cases, then write the
strategy question, candidate policy, threshold source, and backtest plan.
