# Investigations

Each investigation is a concrete token or pool review. Investigations should be
small enough to reproduce and specific enough to become regression tests.

Use a stable folder name:

```text
<token_symbol_or_name>_<short_context>_<start_block>
```

Examples:

```text
sss_space_services_25041048
biba_primary_v2_pool_25039257
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

## Current Issue Ledger

Keep this ledger short. It should contain the latest issues we intend to
investigate or implement next. Fixed rows should move to focused investigation
folders, regression tests, or commit history after the fix lands.

### Active

| Priority | Status | Area | Token / Pool | Evidence | Next Check | Investigation |
| --- | --- | --- | --- | --- | --- | --- |
| P0 | investigating | live candidate selection | V3/V4 LP positions | latest confirmed slow run `run-20260512-150750Z-pid-1394158`: 9 slow live applies in first 24 live blocks, with ~4.3s in candidate selection; prior long run had 402 slow applies | implement indexed LP-position candidate lookup keyed by position manager/token id/owner/operator; target candidate-selection p95 <100ms | live_position_candidate_index_25079830 |
| P0 | investigating | live state parity | mined token update txs | prior long run `run-20260512-095105Z-pid-326613` had five `token_transaction_update_failed` lack-of-funds reports for txs mined successfully on chain | add validation-only sender funding or use a live-aware state context that includes sender balances for mined token updates | live-token-update-sender-funding |
| P0 | investigating | processed block parity | blocks `25078746`, `25033700` | live warmup failed on block `25078746` when local DB replay rejected mined tx `0x806ed236...c7c5fa` as lack-of-funds; canonical traces show earlier same-block credits and current full-trace block processing succeeds | keep block processing strict; reproduce the temporary MDBX/readiness divergence and add regression checks that require full traces | processed_block_replay_parity_25078746 |
| P0 | confirmed | source eligibility parity | V4 observed-flow-only pools | 6 V4 Universal Router buy failures came from pools with top-level observed `can_buy=true/can_sell=true`, but same-route probes failed at `block-1` and entry block for `0.01`, `0.001`, and `0.0001 ETH`; stored source observations lack `runtime_state.can_buy/can_sell` | rebuild the source observation range from current token-server output, verify the replay rows preserve runtime trading flags, and confirm these entries disappear | v4_observed_flow_only_eligibility_25065694 |
| P1 | confirmed | entry sizing policy | 5 WETH-denominated V2/V3 buy failures | latest 15k WETH/native-only run has 3 V2 `TRANSFER_FAILED` and 2 V3 `TF` buy failures; representative probes fail at `0.01 ETH` and succeed at `0.001 ETH` on the same route at `block-1` and entry block | decide fixed-size failure vs adaptive sizing vs pre-entry size guard before strategy tuning | weth_buy_size_sensitivity_25065694 |
| P1 | confirmed | sell restriction classification | 19 V2 `TRANSFER_FROM_FAILED` reports + 1 V3 `V3InvalidSwap` | latest 15k run `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954` has 20 failed sell reports. BCB2 later sells on retry; the 8 still-open failed exits are zero-valued open exposure. DF1A/BE74/2834 are address-specific, 11AE is chunkable, Pancake V2 is unsellable for this strategy route/address, two V2 tokens have no observed sell evidence, and the V3 case is drained at the exit block. | choose strategy policies for retries, chunked exits, no-observed-sell exposure, and address-specific restrictions | transfer_from_failed_exit_classification_25065694 |
| P1 | investigating | V3 pool parity | pools `0x067e...fb99`, `0xe377...0eed`, `0x528c...88f` | live run reported pool-local `uniswap_v3_pool_identity_mismatch` / `uniswap_v3_pool_missing_at_block`; token-server latest ops API currently has zero grouped issues, so recheck on next long run | verify each pool against factory `getPool` and actual emitting factory; if chain agrees with warning, keep pool-local issue only; otherwise fix discovery/config selection | live-v3-pool-identity-parity |
| P1 | investigating | route-equivalent V2 sell parity | WCT, SCREAM, BAG | chain shows buys/sells through Universal Router, Permit2, or external routers while classic V2 simulator can report `can_sell=false` | replay actual mined route before labeling cannot-sell; decide whether to add route-aware simulator support or downgrade classic-route failures to route mismatch | wct_universal_router_sell_parity_24991564 / scream_route_dependent_sell_parity_24995988 / bag_external_buy_sell_parity_24987948 |
| P2 | investigating | amount-quality / dust sell parity | Xtrim `0x16a8...2274` pool `0x4b2f...7cc` | simulator reports `can_buy=true`, `can_sell=false` with `UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT` rather than transfer-from failure | inspect simulated buy output, token transfer tax/burn, and balance extraction before treating pool as buyable | xtrim_dust_sell_input_parity_24987207 |

### Recently Fixed

| Status | Issue | Evidence | Fix |
| --- | --- | --- | --- |
| fixed | SushiSwap V3 pools misclassified as Uniswap V3 | token `0x2b9d...5512` pool `0x1137...a160` was created by factory `0xBAcE...9C4F`; Uniswap V3 factory returned zero, so post-block sim emitted `No Uniswap V3 pool found` | `d8dbbae8` records V3 `factory_address`, adds `KnownV3Protocol`, routes Sushi V3 through Sushi factory/router, and bumps processed-block cache schemas |
| fixed | latest token-server failure noise | restarted run `run-20260512-185408Z-pid-2545165` has zero `pipeline_issues`, zero `pool_buy_sell_sim_failures`, and no `token_sim_session_profile` rows in normal mode | session-profile rows are now gated behind `TOKEN_SIM_SESSION_PROFILE=1`; successful pool sim completions and handled disk-cache miss details are debug-only |
| fixed | sell-only balance-layout parity for the 15k baseline | STRX, ROME, and PERP were sellable on chain but failed fresh-fork valuation with `unsupported_balance_storage_layout`; rerun `hist-snipe-all-poolonly-15k-balance-overlay-20260513-120114` has zero failures in that bucket | sell setup now derives recipient storage from a proven token transfer diff with deterministic tax gross-up; STRX probe sells 100% at block `25,073,894` |
| fixed | sell proceeds must be recipient-net before gross fallback | the gross-output run fixed zero-fill sells but overcounted third-party token auto-swap WETH for 8 confirmed sells; corrected run `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954` moved total PnL from the invalid `+15.137896975732239922 ETH` to `+9.693440469949935980 ETH`, and moved top token `0x12a776...2DA1e5` from `+9.020898526198050656 ETH` PnL to `+3.752855185725959110 ETH` | sell simulation now prefers the strategy recipient's net ETH/WETH balance increase, with gross pool-output fallback only when recipient net proceeds are unavailable |
| explained | DBB top winner was realized before scam | rank-3 winner `0xDBb776...878B20` sold at block `25,077,831` for `2.006753067216354612 ETH`; source observations first mark the pool scammed at block `25,077,998` after WETH reserve collapsed from `18.27684282640402` to `0.0010138925720517` | keep PnL as valid under non-mempool historical semantics; use `dbb_top_winner_scam_after_exit_25077629` as a max-hold/rug timing regression case |
| explained | top-five low-liquidity winners were sold before collapse | ranks 2 through 5 now have low WETH reserves, but each backtest sell happened before the first bad observation; same-amount probes at the bad block fail down to 1%, while sell-block probes match recorded proceeds | keep realized PnL as valid for the max-hold policy; use `top5_winner_liquidity_after_exit_25074419` to distinguish max-hold results from never-sell results |
| explained | representative worst losers are real strategy outcomes | sampled V4 dead-pool, V2 rug-before-exit, and sell-failed full-size restriction cases all match chain-sim probes: dead pools are zero-valued and 11AE full-size sell fails while 5% chunks succeed | keep zero-value treatment for current baseline; partial/chunked exits require a separate strategy policy | worst_loser_position_checks_25073543 |
| fixed | Pancake V2 protocol string was treated as unsupported | stored source observations used `pancakeswap-v2`, while the engine parser only accepted `pancake-v2`; latest run `hist-snipe-all-poolonly-15k-pancake-protocol-20260513-124112` has zero unsupported-protocol failures | `parse_protocol` now accepts `pancakeswap-v2` / `pancakeswap_v2`; the affected Pancake pool is now correctly exposed as a confirmed buy with failed sells |
| fixed | V3 Universal Router sell route parity | BFB observed sell `0xd629...04b68f` used Universal Router `0x4C82...2cCA` with V3 input encoded as six params plus `UNWRAP_WETH`; same-route strategy probes now succeed at observed block `25,075,896` and fail at strategy exit block `25,076,700` only after pool liquidity is zero | V3 sell builder now emits `V3_SWAP_EXACT_IN + UNWRAP_WETH`, routes WETH output to `address(2)`, includes the router variant's trailing empty bytes field, and decodes `V3InvalidSwap` / `SliceOutOfBounds` |
| fixed | stable-denom pools polluted the ETH-denominated baseline | previous 15k run had 2 USDC/USDT empty-router buy failures and 1 confirmed USDT-denom position; denom-aware probes showed the pools were tradeable with 6-decimal quote sizes but `1e16` raw USDC/USDT was nonsensical for a `0.01 ETH` budget | shared entry eligibility now rejects non-WETH/non-native quote pools with `unsupported_execution_denom` until quote conversion and ETH-equivalent PnL exist; rerun `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954` has no confirmed non-WETH positions and no empty-router buy failures |

Status values:

- `new`: captured from the token range builder, UI, or logs.
- `investigating`: chain-truth or simulator parity work has started.
- `confirmed`: the behavior is real and needs a detector, guardrail, or display rule.
- `explained`: the behavior is real but expected, or the display should clarify it.
- `fixed`: code or display logic has been changed and verified.
- `ignored`: not useful after review.
