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

### Active Investigation Queue

| Priority | Status | Area | Token / Pool | Evidence | Next Check | Investigation |
| --- | --- | --- | --- | --- | --- | --- |
| P0 | confirmed | source eligibility parity | V4 observed-flow-only pools | The 15k source replay selected six V4 pools from observed third-party flow. Same-route Universal Router probes fail at `block-1` and entry block for `0.01`, `0.001`, and `0.0001 ETH`; stored source rows lack `runtime_state.can_buy/can_sell`. | Rebuild the source observation range from current token-server output with runtime trading flags, rerun the 15k baseline, and close this if those entries become skipped eligibility decisions. | v4_observed_flow_only_eligibility_25065694 |
| P1 | investigating | reserve quality / display | SSS Space Services | Token-builder replay reproduces the creator-triggered `sync()` with no transfers, same-block WETH-drain sell, token-reserve dust, `can_buy=true`, `can_sell=false`, and a raw `2128397x` price ratio. | Promote reserve-quality display/detector rules and only run same-prestate synthetic sell if we need to prove `observed_sell_simulator_fail`. | sss_space_services_25041048 |

### Policy Backlog

These are classified behaviors. They should not stay in the active parity queue
unless a new run shows a mismatch against the stated policy.

| Status | Area | Evidence | Policy Work | Investigation |
| --- | --- | --- | --- | --- |
| explained | fixed-size entry sizing | Five WETH V2/V3 entries fail at the configured `0.01 ETH` order and succeed at `0.001 ETH` on the same route. | Keep fixed-size failures as failed entries, or design adaptive sizing / pre-entry size guards as a separate strategy. | weth_buy_size_sensitivity_25065694 |
| explained | failed exits and retryable exposure | The 15k run's open failed exits are bucketed into address-specific restrictions, chunkable max-transfer, no observed sell evidence, Pancake V2 unsellable, and V3 drained liquidity. | Choose retry cadence, max retries, chunked exits, address-specific exposure treatment, and no-observed-sell treatment. | transfer_from_failed_exit_classification_25065694 |
| explained | full-size loss treatment | Worst-loser samples match chain behavior: dead pools are zero-valued and 11AE full-size exit fails while smaller chunks work. | Keep zero-value treatment for current fixed-size baseline; partial exits belong to a different strategy. | worst_loser_position_checks_25073543 |

### Needs Fresh Evidence

These rows came from older live or range observations and do not currently have
complete investigation folders. Reproduce them on the current code before
treating them as active work.

| Status | Area | Old Evidence | Fresh Check |
| --- | --- | --- | --- |
| needs_recheck | live candidate selection | `run-20260512-150750Z-pid-1394158` had 9 slow live applies in the first 24 live blocks, with candidate selection around 4.3s; an earlier long run had 402 slow applies. | Run a current long live apply or replay with candidate-selection timing and only open an investigation if p95 is still above target. |
| needs_recheck | live state parity | `run-20260512-095105Z-pid-326613` had five mined token update txs reported as lack-of-funds. | Reproduce on current live-aware replay; if still present, create `live_token_update_sender_funding_<block>` with tx hashes and trace evidence. |
| needs_recheck | processed block replay parity | Live warmup once rejected block `25078746` for lack-of-funds while canonical full traces succeeded. | Re-run strict processed-block load on `25078746` and `25033700`; keep only if the current MDBX/cache path still diverges from full trace. |
| needs_recheck | V3 pool identity parity | Prior live run reported `uniswap_v3_pool_identity_mismatch` / `uniswap_v3_pool_missing_at_block`, while the latest ops API had zero grouped issues. | Recheck the three pools against factory `getPool` and current grouped issues before opening a parity investigation. |
| needs_recheck | route-equivalent V2 sell parity | WCT, SCREAM, and BAG had observed Universal Router, Permit2, or external-router sells while classic V2 simulation could report `can_sell=false`. | Replay the mined route and classify as route mismatch only if current route-aware simulation still cannot reproduce it. |
| needs_recheck | amount-quality / dust sell parity | Xtrim produced `UniswapV2Library: INSUFFICIENT_INPUT_AMOUNT` rather than transfer-from failure. | Inspect simulated buy output, token tax/burn, and balance extraction before treating the pool as buyable. |

### Recently Fixed Or Explained

| Status | Issue | Evidence | Fix |
| --- | --- | --- | --- |
| fixed | MOTH live retention dropped depleted liquidity-removal pool | token detail showed `PAIR_CREATION`, `risk=clear`, and `pools=0` while mempool signal `1724` showed a 98.45% WETH removal | live retention now keeps evidence-bearing liquidity-removal pools for 15,000 blocks, refreshes lifecycle after pruning, and includes pool liquidity-removal in token risk |
| fixed | SushiSwap V3 pools misclassified as Uniswap V3 | token `0x2b9d...5512` pool `0x1137...a160` was created by factory `0xBAcE...9C4F`; Uniswap V3 factory returned zero, so post-block sim emitted `No Uniswap V3 pool found` | `d8dbbae8` records V3 `factory_address`, adds `KnownV3Protocol`, routes Sushi V3 through Sushi factory/router, and bumps processed-block cache schemas |
| fixed | latest token-server failure noise | restarted run `run-20260512-185408Z-pid-2545165` has zero `pipeline_issues`, zero `pool_buy_sell_sim_failures`, and no `token_sim_session_profile` rows in normal mode | session-profile rows are now gated behind `TOKEN_SIM_SESSION_PROFILE=1`; successful pool sim completions and handled disk-cache miss details are debug-only |
| fixed | sell-only balance-layout parity for the 15k baseline | STRX, ROME, and PERP were sellable on chain but failed fresh-fork valuation with `unsupported_balance_storage_layout`; rerun `hist-snipe-all-poolonly-15k-balance-overlay-20260513-120114` has zero failures in that bucket | sell setup now derives recipient storage from a proven token transfer diff with deterministic tax gross-up; STRX probe sells 100% at block `25,073,894` |
| fixed | sell proceeds must be recipient-net before gross fallback | the gross-output run fixed zero-fill sells but overcounted third-party token auto-swap WETH for 8 confirmed sells; corrected run `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954` moved total PnL from the invalid `+15.137896975732239922 ETH` to `+9.693440469949935980 ETH`, and moved top token `0x12a776...2DA1e5` from `+9.020898526198050656 ETH` PnL to `+3.752855185725959110 ETH` | sell simulation now prefers the strategy recipient's net ETH/WETH balance increase, with gross pool-output fallback only when recipient net proceeds are unavailable |
| explained | VYP was a backdoored pair-balance drain, not hidden mint/direct LP removal | total supply stayed `1,000,000,000` VYP; LP was transferred to `0xdead` at block `25,077,327`; at block `25,077,595` an actor moved `588075766.703258098` VYP from the pair despite zero allowance, then sold those tokens through the router | use `vyp_burned_lp_reserve_drain_25077324` as the canonical `pair_balance_backdoor_drain` case |
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
- `needs_recheck`: old evidence exists, but the issue must be reproduced on the
  current code before it should be treated as active.
- `ignored`: not useful after review.
