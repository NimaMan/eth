# Source Simulator Parity

Cases where chain truth, source observations, simulator route probes, or Alpha
executable routes disagree or need proof before the result can support trading
or model evidence.

## Cases

| Status | Case | Current Disposition |
| --- | --- | --- |
| confirmed | [compass_v2_vault_helper_route_chain_parity_25122982](../../cases/compass_v2_vault_helper_route_chain_parity_25122982/) | The deployed V2 vault's classic-router sell path is not helper-route equivalent; exact parity needs historical vault rehearsal and tx-index-aware route replay. |
| confirmed | [external_router_classic_v2_scope_25122982](../../cases/external_router_classic_v2_scope_25122982/) | Helper routes can sell from liquid pools while classic V2 route fails; `COMPASS AI` is a separate stateful fresh-buyer restriction case. |
| fixed | [v4_observed_flow_only_eligibility_25065694](../../cases/v4_observed_flow_only_eligibility_25065694/) | V4 source-observation leakage is fixed; exact-window reruns are evidence hygiene, not an active P0 blocker. |

Related cross-case rules live in `../../../investigations/parity/`.
