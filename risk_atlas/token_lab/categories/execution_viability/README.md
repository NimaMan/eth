# Execution Viability

Cases where the token/pool fact is about executable trade scope, route support,
denomination support, or size sensitivity. Token Lab owns the factual evidence;
Alpha lab owns the strategy-policy interpretation.

## Cases

| Status | Case | Current Fact |
| --- | --- | --- |
| fixed | [stable_denom_execution_scope_25065694](../../cases/stable_denom_execution_scope_25065694/) | Stable-denom pools require quote-aware execution and should not be treated as WETH-equivalent by the executable baseline. |
| explained | [weth_buy_size_sensitivity_25065694](../../cases/weth_buy_size_sensitivity_25065694/) | Fixed `0.01 ETH` entries fail while smaller orders execute; this is an execution-size fact before it becomes an Alpha sizing policy. |

Strategy-facing follow-up lives in
`../../../../alpha/lab/strategy_analysis/risk_atlas_inputs/`.
