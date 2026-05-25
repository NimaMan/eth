# Promotion Queue

This queue tracks cross-case questions that block promotion into strategy
policy, modeling, detectors, or operator-facing Risk Atlas pages.

## P0 Parity Gate

Resolve these before treating related backtests, model rows, or protocol
coverage as production evidence.

Current open P0 parity blockers: none. Newly discovered source/simulator
disagreements should be added here before they are allowed to become policy or
performance evidence.

## Ranked Queue

Work this table in order. Chain/source/simulator parity stays above strategy
policy, display work, and operator UX because it determines whether the
evidence base is trustworthy.

| Order | Priority | Category | Status | Issue | Why It Ranks Here | Next Check |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | P1 | `source_simulator_parity` | confirmed | [helper-route V2 sell parity](../../token_lab/cases/compass_v2_vault_helper_route_chain_parity_25122982/) | `Compass` has meaningful WETH reserves and non-dust same-block helper-route sells, while actual pre-block holders still fail through Alpha's classic V2 route. The deployed V2 vault also uses the classic router sell path, and exact historical replay needs a vault deploy/code overlay because the vault did not exist at the affected blocks. | Add tx-index-aware observed-route replay/classification, add historical V2 vault rehearsal, and do not treat observed helper-route sells as Alpha-executable sellability until the exact executable route passes. |
| 2 | P1 | `mechanism_classification` | confirmed | [SSS creator transferFrom pair drain](../../token_lab/cases/sss_creator_transfer_from_pair_drain_25041123/) | Exact chain truth shows creator `transferFrom(pair, drain_wallet, ...)` at tx `197`, creator `sync()` at tx `198`, and WETH drain sell at tx `200`; Alpha's current deployed V2 vault route fails even before tx `197`, so this is avoid-only evidence for our route. | Decide whether the live detector should key on mempool calldata, token transfer logs, or both; then run a corpus false-positive pass before promotion. |

## Fresh Parity Recheck Backlog

Current open fresh recheck backlog: none. Re-add a row here only when old
evidence has token/pool coordinates or reproduces on current code.

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
