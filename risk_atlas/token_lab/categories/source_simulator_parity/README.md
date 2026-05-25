# Source Simulator Parity

Cases where chain truth, source observations, simulator route probes, or Alpha
executable routes disagree or need proof before the result can support trading
or model evidence.

## Questions

Ask these before a case can be used as strategy evidence or model data:

| Question | Why It Matters |
| --- | --- |
| What is the exact chain coordinate: block, transaction index, pool, token, sender, and route? | Parity failures are often coordinate mistakes. |
| What did chain truth show at that coordinate? | Establishes the reference state. |
| Did token-builder/source observations record the same reserves, flags, protocol, pool identity, and risk labels? | Finds read-model or observation leakage. |
| Did the simulator replay the same pre-state and prior block-prefix transactions? | Catches missing setup transactions and stale state. |
| Did the simulated buy/sell use the same route shape and sender class as the chain or Alpha route being evaluated? | Distinguishes observed helper routes from Alpha-executable routes. |
| If observed chain sells succeeded but Alpha route fails, what is different: router, sender, allowance, transfer restriction, route encoding, or timing? | Prevents observed sellability from being treated as our sellability. |
| Does the failure block promotion, or is it resolved by current code/tests? | Decides whether the case belongs in the promotion queue. |

## Cases

| Status | Case | Current Disposition |
| --- | --- | --- |
| confirmed | [compass_v2_vault_helper_route_chain_parity_25122982](../../cases/compass_v2_vault_helper_route_chain_parity_25122982/) | The deployed V2 vault's classic-router sell path is not helper-route equivalent; exact parity needs historical vault rehearsal and tx-index-aware route replay. |
| confirmed | [external_router_classic_v2_scope_25122982](../../cases/external_router_classic_v2_scope_25122982/) | Helper routes can sell from liquid pools while classic V2 route fails; `COMPASS AI` is a separate stateful fresh-buyer restriction case. |
| fixed | [v4_observed_flow_only_eligibility_25065694](../../cases/v4_observed_flow_only_eligibility_25065694/) | V4 source-observation leakage is fixed; exact-window reruns are evidence hygiene, not an active P0 blocker. |

Related cross-case rules live in `../../../investigations/parity/`.
