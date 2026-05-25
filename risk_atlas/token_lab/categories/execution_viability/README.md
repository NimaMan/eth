# Execution Viability

Cases where the token/pool fact is about executable trade scope, route support,
denomination support, or size sensitivity. Token Lab owns the factual evidence;
Alpha lab owns the strategy-policy interpretation.

## Questions

Ask these before turning a pool fact into an executable strategy assumption:

| Question | Why It Matters |
| --- | --- |
| Which route is being tested: protocol, router/vault, sender, recipient, denomination, and amount? | Execution facts are route-specific. |
| Is the quote denomination supported by the route and accounting layer? | Prevents stable/other-denom pools from being treated as WETH-equivalent. |
| Does the intended size execute, and what smaller/larger sizes change the result? | Identifies fixed-size failures and adaptive-sizing candidates. |
| Does buy succeed, sell succeed, or only one side succeed? | Separates entry eligibility from exit/recovery assumptions. |
| Does execution depend on address, route, timing, or state after another transaction? | Determines whether the fact is generally tradeable or route-specific. |
| Is this a Risk Atlas fact or an Alpha policy decision? | Risk Atlas records viability; Alpha chooses thresholds and strategy variants. |

## Cases

| Status | Case | Current Fact |
| --- | --- | --- |
| fixed | [stable_denom_execution_scope_25065694](../../cases/stable_denom_execution_scope_25065694/) | Stable-denom pools require quote-aware execution and should not be treated as WETH-equivalent by the executable baseline. |
| explained | [weth_buy_size_sensitivity_25065694](../../cases/weth_buy_size_sensitivity_25065694/) | Fixed `0.01 ETH` entries fail while smaller orders execute; this is an execution-size fact before it becomes an Alpha sizing policy. |

Strategy-facing follow-up lives in
`../../../../alpha/lab/strategy_analysis/risk_atlas_inputs/`.
