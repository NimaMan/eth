# Investigations

Investigations are the Risk Atlas evidence layer. They include concrete token or
pool cases, the parity method used to validate facts, and the shared catalog of
behavior/mechanism labels learned from those cases.

Use a stable folder name:

```text
cases/<token_symbol_or_name>_<short_context>_<start_block>
```

Examples:

```text
cases/sss_space_services_25041048
cases/biba_primary_v2_pool_25039257
```

Investigations are not just notes. A complete investigation `README.md` should
explain:

- what the range builder reported;
- what the chain actually did;
- whether the simulator can reproduce it;
- whether a detector or guardrail should exist before trading;
- what code changed because of the investigation.

Investigation folders should normally contain only:

- `README.md`: the single narrative markdown file;
- `investigation.toml`: machine-readable metadata;
- `artifacts/README.md`: artifact folder instructions.

Generated artifacts belong under `artifacts/` and are ignored by default.

## Layout

| Path | Purpose |
| --- | --- |
| `cases/` | Concrete token/pool investigations with narrative, metadata, and artifacts. |
| `parity/` | Chain truth vs token-builder/source observation vs simulator vs Alpha route methodology. |
| `behavior_catalog/` | Shared catalog of suspicious behavior patterns and scam mechanism families. |

Investigation scripts belong in the module that owns the capability. For
example, token tracking audits belong under `eth_token`, route replays belong
under `tx_simulator` or `tx_processor`, and strategy analysis belongs under
`alpha/lab`.

## Current Issue Ledger

Keep this ledger categorized. The first category is always the parity gate:
chain truth, token-builder/read-model rows, source observations, and simulator
behavior must agree before an investigation can support policy or performance
claims.

## Investigation Categories

| Category | Primary Question | Blocks Promotion? |
| --- | --- | --- |
| `source_simulator_parity` | Do chain truth, token-builder/source observations, and simulator route probes agree? | Yes. This blocks strategy evidence and model data for the affected protocol/cohort. |
| `backtest_result_validity` | Is reported PnL/accounting/lifecycle state correct under the declared execution model? | Yes for the affected run/policy. |
| `mechanism_classification` | What behavior or scam mechanism happened, and are labels precise enough? | Blocks labels/features when unresolved. |
| `alpha_strategy_input` | Does a fact belong in Alpha lab for strategy/cohort analysis? | No; Risk Atlas records the fact and Alpha lab owns the policy decision. |
| `display_read_model` | How should Risk Atlas/Asena surface the behavior without misleading operators? | Blocks UI trust, not necessarily execution. |
| `network_actor_context` | Do wallet/fund-flow relationships add explanatory or predictive signal? | Research only until promoted into features. |
| `ops_freshness_recheck` | Old evidence may be stale; does it still reproduce on current code? | Blocks only if reproduced. |

## P0 Parity Gate

Resolve these before treating related backtests, model rows, or protocol
coverage as production evidence.

Current open P0 parity blockers: none. Keep newly discovered source/simulator
disagreements here before they are allowed to become policy or performance
evidence.

## Ranked Promotion Queue

Work this table in order. Chain/source/simulator parity stays above strategy
policy, display work, and operator UX because it determines whether the
evidence base is trustworthy.

| Order | Priority | Category | Status | Issue | Why It Ranks Here | Next Check |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | P1 | `source_simulator_parity` | confirmed | [helper-route V2 sell parity](cases/compass_v2_vault_helper_route_chain_parity_25122982/) | `Compass` has meaningful WETH reserves and non-dust same-block helper-route sells, while actual pre-block holders still fail through Alpha's classic V2 route. The deployed V2 vault also uses the classic router sell path, and exact historical replay needs a vault deploy/code overlay because the vault did not exist at the affected blocks. | Add tx-index-aware observed-route replay/classification, add historical V2 vault rehearsal, and do not treat observed helper-route sells as Alpha-executable sellability until the exact executable route passes. |
| 2 | P1 | `mechanism_classification` | confirmed | [SSS creator transferFrom pair drain](cases/sss_creator_transfer_from_pair_drain_25041123/) | Exact chain truth shows creator `transferFrom(pair, drain_wallet, ...)` at tx `197`, creator `sync()` at tx `198`, and WETH drain sell at tx `200`; Alpha's current deployed V2 vault route fails even before tx `197`, so this is avoid-only evidence for our route. | Decide whether the live detector should key on mempool calldata, token transfer logs, or both; then run a corpus false-positive pass before promotion. |

## Fresh Parity Recheck Backlog

Current open fresh recheck backlog: none. Re-add a row here only when old
evidence has token/pool coordinates or reproduces on current code.

## Categorized Investigation Index

| Category | Status | Investigation | Current Disposition |
| --- | --- | --- | --- |
| `source_simulator_parity` | confirmed | `compass_v2_vault_helper_route_chain_parity_25122982` | Focused chain-parity note: the deployed V2 vault's classic-router sell path is not helper-route equivalent; exact parity needs historical vault rehearsal and tx-index-aware route replay. |
| `source_simulator_parity` | confirmed | `external_router_classic_v2_scope_25122982` | `Compass` helper routes can sell from liquid pools while classic V2 route fails; `COMPASS AI` is a separate stateful fresh-buyer restriction case. |
| `source_simulator_parity` | fixed | `v4_observed_flow_only_eligibility_25065694` | V4 source-observation leakage is fixed; exact-window reruns are evidence hygiene, not an active P0 blocker. |
| `mechanism_classification` | confirmed | `sss_creator_transfer_from_pair_drain_25041123` | Focused SSS mechanism note: creator/control `transferFrom(pair, ...)` is the first state-changing warning, `sync()` publishes the manipulated balance, the WETH drain follows in the same block, and Alpha's current deployed V2 vault route cannot sell even before the sequence. |
| `display_read_model` | fixed | `sss_space_services_25041048` | Chain/token-builder parity is reproduced; display price/init is suppressed for token-reserve-dust pools, and durable observation/model fields are in place. |
| `display_read_model` | fixed | `mothman_live_retention_liquidity_removal_25063339` | Live retention now preserves evidence-bearing depleted liquidity-removal pools. |
| `backtest_result_validity` | fixed | `sell_proceeds_recipient_net_25065694` | Sell proceeds now prefer strategy-recipient net ETH/WETH before gross pool output. |
| `backtest_result_validity` | explained | `top5_winner_liquidity_after_exit_25074419` | Winners sold before later collapse; realized PnL is valid for max-hold semantics. |
| `backtest_result_validity` | explained | `dbb_top_winner_scam_after_exit_25077629` | DBB exited before scam observation; PnL valid under non-mempool historical semantics. |
| `backtest_result_validity` | explained | `worst_loser_position_checks_25073543` | Representative losses match dead-pool and full-size failed-exit behavior. |
| `mechanism_classification` | explained | `vyp_burned_lp_reserve_drain_25077324` | LP was locked; WETH was drained by backdoored pair-balance transfer plus sell, not normal LP removal. |
| `mechanism_classification` | explained | `transfer_from_failed_exit_classification_25065694` | Failed exits are bucketed into address restriction, chunking, no observed sell, Pancake V2 unsellable, and V3 drained liquidity. |
| `alpha_strategy_input` | fixed | `stable_denom_execution_scope_25065694` | Stable-denom pools stay in research, but are excluded from ETH-denominated executable baseline until quote conversion exists. |
| `alpha_strategy_input` | explained | `weth_buy_size_sensitivity_25065694` | Fixed `0.01 ETH` entries fail while smaller orders execute; adaptive sizing is a separate Alpha lab question. |

## Resolved Parity/Infrastructure Notes

These are not active investigation blockers, but they explain why current
parity assumptions changed.

| Status | Issue | Evidence | Fix |
| --- | --- | --- | --- |
| fixed | V4 observed-flow-only entries polluted strategy eligibility | Old 15k source replay selected six V4 pools from observed third-party flow even though same-route Universal Router probes failed at `block-1` and entry block. | Current V4 Risk Atlas rows persist effective trading flags, current strategy observations persist runtime flags, and Alpha's `PoolWire` conversion treats V4 rows with missing runtime flags as non-tradeable. Fresh DB check found zero persisted V4 buy-failed execution reports. |
| fixed | V3 pool identity parity recheck | Prior live evidence mentioned `uniswap_v3_pool_identity_mismatch` / `uniswap_v3_pool_missing_at_block`, but the current ops API reports zero grouped issues, current `eth_chain_server` pipeline logs have no V3 identity/missing issues, and `eth_token` V3 discovery tests pass for Uniswap, SushiSwap, and PancakeSwap factories. | Factory-aware V3 discovery and simulation routing now persist/use `factory_address` and `KnownV3Protocol`; no focused investigation is opened unless the issue reproduces on current code. |
| fixed | amount-quality / dust sell parity recheck | The old Xtrim note did not include reproducible token/pool coordinates, current Risk Atlas DB has no `INSUFFICIENT_INPUT_AMOUNT` observations, and the pool buy/sell simulator now uses denom-aware buy/sell route dispatchers with V2/Sushi fee-on-transfer supporting token-to-token swaps. | Keep the detector family for future rows, but do not keep this as an active blocker without a current reproduction. Fresh candidates must record simulated buy output, sell `amountIn`, pool liquidity level, and calldata path before promotion. |
| fixed | SushiSwap V3 pools misclassified as Uniswap V3 | token `0x2b9d...5512` pool `0x1137...a160` was created by factory `0xBAcE...9C4F`; Uniswap V3 factory returned zero, so post-block sim emitted `No Uniswap V3 pool found` | `d8dbbae8` records V3 `factory_address`, adds `KnownV3Protocol`, routes Sushi V3 through Sushi factory/router, and updates schema metadata |
| fixed | latest token-server failure noise | restarted run `run-20260512-185408Z-pid-2545165` has zero `pipeline_issues`, zero `pool_buy_sell_sim_failures`, and no `token_sim_session_profile` rows in normal mode | session-profile rows are now gated behind `TOKEN_SIM_SESSION_PROFILE=1`; successful pool sim completions and handled disk-cache miss details are debug-only |
| fixed | sell-only balance-layout parity for the 15k baseline | STRX, ROME, and PERP were sellable on chain but failed fresh-fork valuation with `unsupported_balance_storage_layout`; rerun `hist-snipe-all-poolonly-15k-balance-overlay-20260513-120114` has zero failures in that bucket | sell setup now derives recipient storage from a proven token transfer diff with deterministic tax gross-up; STRX probe sells 100% at block `25,073,894` |
| fixed | Pancake V2 protocol string was treated as unsupported | stored source observations used `pancakeswap-v2`, while the engine parser only accepted `pancake-v2`; latest run `hist-snipe-all-poolonly-15k-pancake-protocol-20260513-124112` has zero unsupported-protocol failures | `parse_protocol` now accepts `pancakeswap-v2` / `pancakeswap_v2`; the affected Pancake pool is now correctly exposed as a confirmed buy with failed sells |
| fixed | V3 Universal Router sell route parity | BFB observed sell `0xd629...04b68f` used Universal Router `0x4C82...2cCA` with V3 input encoded as six params plus `UNWRAP_WETH`; same-route strategy probes now succeed at observed block `25,075,896` and fail at strategy exit block `25,076,700` only after pool liquidity is zero | V3 sell builder now emits `V3_SWAP_EXACT_IN + UNWRAP_WETH`, routes WETH output to `address(2)`, includes the router variant's trailing empty bytes field, and decodes `V3InvalidSwap` / `SliceOutOfBounds` |

Status values:

- `new`: captured from the token range builder, UI, or logs.
- `investigating`: chain-truth or simulator parity work has started.
- `confirmed`: the behavior is real and needs a detector, guardrail, or display rule.
- `explained`: the behavior is real but expected, or the display should clarify it.
- `fixed`: code or display logic has been changed and verified.
- `needs_recheck`: old evidence exists, but the issue must be reproduced on the
  current code before it should be treated as active.
- `ignored`: not useful after review.
