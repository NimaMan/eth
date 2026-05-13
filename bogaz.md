# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only active bottlenecks here:
what is limiting the pipeline, the evidence, the owning subsystem, and the next
action. Completed work and long investigation notes belong in focused docs or
commit history, not in this file.

## Operating Rule

A run is useful only if it tells us which limit dominates: strategy quality,
chain-sim fill realism, decision auditing, backtest parity, mempool signal
recall, live-feed reliability, or execution readiness.

The live mempool and token-tracking architecture is documented in the related
subsystem READMEs, primarily `mempool_processor/README.md` and
`eth_chain_server/README.md`.

## Active Bottlenecks

| Order | Bottleneck | Owner | Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Redis-mediated live block/state handoff** | `eth_token_server` -> `eth_chain_server`, `tx_processor`, `eth_live_feed`, `tx_simulator` | The live token tracker currently tails `eth/live/blocks` through Redis and loads processed blocks through `LiveProcessedBlockProvider`. The live block processor also publishes `eth/live/block/<n>/chain_state_snapshot` values around 335 MB each. Redis was using about 12.66 GB during the investigation, and V2 metadata lookups timed out while the local Reth historical view was behind the live block. The same pool/block metadata probe took 6524 ms while state was not ready and about 15 ms after catch-up, so reducing the timeout to 250 ms only limits damage; it does not remove the hot-path handoff bottleneck. | Rename the production service to `eth_chain_server` and merge live block processing and live token tracking into one Redis-free `LiveChainRuntime`. Process blocks, keep the block-scoped state/session in memory, apply token tracking directly from each `LiveBlockUpdate`, and serve recent processed blocks from an in-process ring buffer/API instead of Redis. |
| 2 | **Live token candidate-selection latency** | `eth_token`, `eth_token_server` -> `eth_chain_server` | Current token-server run `run-20260512-150750Z-pid-1394158` is live past block `25,079,830`. In its first 24 live blocks, 9 were already slow and live profile averages showed 4.4s block processor time with 4.3s in candidate selection. The prior longer run `run-20260512-095105Z-pid-326613` showed the same shape: 402 slow live applies, 226 above 5s, max 20.88s. Code currently scans all tracked V3/V4 pools for any ERC721 transfer/approval. | Replace full registry scans in token candidate selection with an indexed LP-position lookup keyed by position manager plus token id, and owner/operator approval lookup where needed. Candidate selection must be proportional to events in the tx, not tracked token count. |
| 3 | **V3 pool identity and missing-pool classification** | `eth_token`, `tx_simulator` | SushiSwap V3 misclassification was fixed in `d8dbbae8`. The latest token-server ops API currently reports zero grouped issues, but the prior live run still had pool-local `uniswap_v3_pool_identity_mismatch` / `uniswap_v3_pool_missing_at_block` warnings for three V3 pools. | Recheck these pools on the next long run. If warnings recur, verify each pool against factory `getPool` and the actual emitting factory; if chain agrees, keep it as a pool-local warning, otherwise fix V3 discovery/config selection. |
| 4 | **Strict processed-block replay parity** | `tx_processor`, `tx_simulator`, `reth_chain_query` | Live token warmup failed at block `25,078,746` because local DB-backed replay rejected mined tx `0x806ed236...c7c5fa` with `lack of funds`. Canonical block traces show txs `6` and `7` credit `0xC0ffee...9671` before tx `8`; the same block now processes with full traces (`236` txs, `1,764` trace nodes), so the failure was replay/state-readiness divergence. No receipt/log-only fallback is allowed in block processing. | Keep block processing strict. Diagnose why the MDBX historical context was temporarily non-canonical, add readiness/parity diagnostics around replay startup, and add regression checks for blocks `25,078,746` and `25,033,700`. |
| 5 | **Live token transaction validation funding gaps** | `eth_token`, `tx_processor`, `tx_simulator` | The prior longer run had five `token_transaction_update_failed` issues with `transaction validation error: lack of funds` on mined transactions, meaning sparse live state can still make successful chain txs look locally invalid. The current run has not yet advanced far enough to re-hit those blocks after live transition. | Apply the same validation-only sender funding policy used for prior replay/metadata paths to live token transaction update paths, or route mined token updates through a state context that cannot fail before execution due only to missing sender balance. |
| 6 | **Strategy policy quality** | `alpha/strategies`, `alpha/engine` | `Snipe All v1` is intentionally broad. Latest clean 15k WETH/native-only run PnL is concentrated: corrected total PnL is `+9.693440469949935980 ETH`, excluding the top five positions is `-0.219822746621256746 ETH`, and excluding the top ten is `-2.606516897707689237 ETH`, so the current baseline is measuring infrastructure and tail winners more than durable edge. | Replace broad entry with selective pool/token scoring, risk-aware exits, position sizing, and clear skip reasons. Measure taken/skipped ratio, top-N concentration, and PnL per pool family. |
| 7 | **Sell restriction policy** | `alpha/engine`, `tx_processor`, `tx_simulator` | Latest 15k run `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954` has 20 failed sell reports but 8 sell-failed positions. Chain classification is complete: BCB2 later confirms on retry, DF1A/BE74/2834 are address-specific because observed chain sellers simulate successfully while the strategy seller fails, 11AE sells in 5%/2%/1% chunks, Pancake V2 fails down to 1% with no token-to-pool sell logs through current head, two V2 tokens have no observed sell evidence, and the V3 case exits into a drained zero-liquidity pool. The 8 still-open failed exits have zero-value snapshots. | Decide strategy policies for retry cadence, chunked exits, address-specific restrictions, and no-observed-sell exposure. |
| 8 | **Chain-sim fill realism** | `alpha/engine`, `tx_processor`, `tx_simulator` | Latest clean 15k-block run `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954` is fully chain-sim reported: 554 reports, 523 confirmed, 31 failed, no missing `position_id` or `order_side`. Current Alpha Lab summary shows 449 positions, 353 open positions, 19 failed positions, 11 buy-failed positions, and 8 sell-failed positions. Failure buckets are 6 V4 observed-flow-only buy reverts, 19 sell `TRANSFER_FROM_FAILED`, 3 V2 buy `TRANSFER_FAILED`, 2 V3 buy `TF`, and 1 V3 `V3InvalidSwap`. The previous 2 stable-denom empty-router reverts and 1 confirmed USDT-denom position are gone after the executable baseline was restricted to native ETH/WETH quote pools. The 5 remaining V2/V3 WETH buy failures are size-sensitive: same-route probes fail at `0.01 ETH` but succeed at `0.001 ETH`. | Rebuild the source observation range with runtime-state trading flags preserved, rerun the 15k backtest, then separate real locked/anti-whale exits from true unsupported routes. |
| 9 | **Trader decision ledger completeness** | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | Historical execution reports now persist `position_id`, `order_side`, and failed `block_number`, so retries no longer hide earlier failed exits. The broader strategy decision ledger still lacks every skip/rule input needed for explainable strategy pages. | Persist a complete decision ledger keyed by `run_id`, `strategy_id`, token, `TokenPoolId`, rule id, input snapshot, action, execution report, exit reason, and PnL snapshot. |
| 10 | **Backtest and live replay parity** | `alpha/backtest`, `alpha/engine`, live feed crates | Backtests must use the same strategy state machine and chain-sim fill adapter as live no-capital runs, otherwise results cannot be trusted. | Run historical replay with the same adapter/contracts as live. Compare replay lower-bound PnL to live chain-sim PnL by pool and by signal type. |
| 11 | **Execution handoff readiness** | `alpha/engine`, `tx_executor` | Real capital should not start until no-capital chain-sim PnL, decision auditing, and persisted signal flow are stable. | Keep real trade paths separated from backtest/live chain-sim. Add real execution only after the no-capital system is profitable and fully auditable. |

## Current Evidence

- Latest issue-log check comes from restarted run
  `logs/eth_token_server/run-20260512-185408Z-pid-2545165`: `pipeline_issues`
  has zero rows, `pool_buy_sell_sim_failures` has zero rows, and
  `token_pipeline_profile` contains only `live_token_apply_profile` and
  `token_block_processor_profile` rows in normal mode.
- Latest live-state handoff evidence comes from the `2026-05-13` token-server
  investigation. V2 metadata lookup timeout was reduced from 2500 ms to 250 ms
  in `eth_token/src/tracking/token_update_router/mod.rs`, and the restarted
  service logged `timeout_ms=250`. The timeout reduction is only a guardrail:
  the root issue is that metadata/simulation code can still wait on a live
  state view that is being reconstructed from Redis/Reth instead of sharing the
  block-scoped state already built by the live block processor.
- Redis live-state snapshots are too large for a hot-path transport layer.
  Observed keys under `eth/live/block/<n>/chain_state_snapshot` were about
  335 MB each, and Redis memory was about 12.66 GB during the bottleneck check.
  This makes the live tracker pay serialization, network, deserialization, and
  cache churn costs for state that already exists in the process that just
  processed the block.
- The current live token tracker does not receive processed blocks directly
  from the live block processor. It tails the Redis stream, then loads the
  block through `LiveProcessedBlockProvider`, which tries Redis first and falls
  back to the regular processed-block provider. That architecture explains why
  cheap V2 metadata reads can become slow when the Redis/Reth state view lags
  the live block.
- First migration slice landed on `2026-05-13`: the crate/package/binary/log
  namespace was renamed to `eth_chain_server`, chain-server owned config keys
  now use `CHAIN_SERVER_*`, `/eth/tokens/api/live/processed-blocks?limit=5`
  exposes an in-process recent live block ring, and gas-rank reads that ring
  instead of the Redis live-block stream. Verification passed with
  `cargo check -p eth_chain_server` and `cargo check -p eth_chain_server
  --examples`.
- Latest token-server latency evidence comes from
  `logs/eth_token_server/run-20260512-150750Z-pid-1394158`. The process was live
  past block `25,079,862`, and the latest observed state had 826 tracked tokens,
  502 tracked pools, and 5 accumulated pool-local transaction failures.
- The dominant token-server bottleneck is not simulation cost. In the current
  run's first live blocks, average router time is 4.4s and average
  candidate-selection time is 4.3s, while V2/V3/V4 simulation averages are much
  smaller.
- Mempool detector persistence is no longer the top Bogaz item for the latest
  run: the restarted detector initialized DB writers and had zero actionable
  simulation errors in the first monitored intervals. It still needs normal
  service supervision and persisted-signal UI verification, but token latency is
  the current blocking issue for live-token freshness.
- Pool-local simulator warning families are now structured telemetry and should
  not stop the live tracker. They still need token-lab parity checks when they
  can change trading status.
- The previous historical backtest limiter was sell-only valuation state
  parity. It is cleared for the 15k baseline by recipient-transfer storage-diff
  balance setup with deterministic tax gross-up: STRX, ROME, and PERP no longer
  fail with `unsupported_balance_storage_layout`, and the STRX probe sells the
  recorded 100% entry amount at block `25,073,894`.
- Sell proceeds extraction now prefers the strategy recipient's net ETH/WETH
  balance increase and only falls back to gross denomination-token pool output
  when recipient net proceeds are unavailable. The gross-output baseline fixed
  zero-fill sells but overcounted token auto-swap proceeds paid to third parties:
  the corrected 15k PnL is `+9.693440469949935980 ETH` instead of the invalid
  gross-output `+15.137896975732239922 ETH`.
- Pancake V2 protocol strings are now parsed from `pancakeswap-v2` /
  `pancakeswap_v2`. The previous unsupported-protocol row is now a confirmed
  Pancake buy followed by failed sells, which is the correct worse outcome to
  classify.
- Processed-block replay is intentionally strict. A mined transaction validation
  failure during block replay is a chain-parity/readiness failure, not something
  the block processor should paper over with receipt-only data.
- Latest clean historical baseline is
  `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`, range
  `25,065,694..25,080,693`, source run `snipe-all-v1-chain-sim-live-v4`.
  It processed 22,584 events, has 449 total positions, 353 open positions,
  19 failed positions, generated 554 execution reports, and ended at
  `+9.693440469949935980 ETH` total PnL. All execution reports have
  `position_id`, `order_side`, and failed reports have a block.
- The prior WETH/native-only run
  `hist-snipe-all-poolonly-15k-weth-only-20260513-134623` is superseded because
  its gross-output sell accounting counted third-party auto-swap WETH as
  strategy proceeds for eight confirmed sells. The largest affected case was
  `0x12a776...2DA1e5`, where reported proceeds moved from
  `9.030898526198050656 ETH` to the seller's actual
  `3.762855185725959110 ETH`.
- The clean run remains strategy-concentration heavy: excluding the top five
  positions turns PnL from `+9.693440469949935980 ETH` to
  `-0.219822746621256746 ETH`; excluding the top ten gives
  `-2.606516897707689237 ETH`.
- Stable-denom entries are now explicitly outside the executable historical
  baseline until quote conversion and ETH-equivalent PnL are implemented. The
  previous 15k run had two stable-denom empty-router buy failures and one
  confirmed USDT-denom position that was routed/accounted incorrectly. The
  WETH/native-only rerun has no confirmed non-WETH positions and no empty-router
  buy failures.
- The remaining 5 V2/V3 WETH buy failures are fixed-size strategy failures, not
  setup failures. Representative V2 and V3 probes fail at `0.01 ETH` on both
  `block-1` and the entry block, then buy and sell successfully at `0.001 ETH`.
- The generic buy-failure bucket was removed from the latest run by executing
  V2/V3 buys through an explicit funded fork with configured gas and decoded
  reverts. Remaining buy failures are now concrete router/token errors instead
  of `"Buy transaction failed"`.
- The 6 V4 Universal Router buy failures in the clean run are not size or
  post-block timing effects. Each failed at the previous block and the decision
  block with 0.01, 0.001, and 0.0001 ETH. The source observations used the
  token-server effective `can_buy` field, which can be true from observed flow.
  Current token-server live API responses include full
  `runtime_state.can_buy/can_sell`, but the stored 15k replay rows do not; the
  source observation range must be rebuilt before this fix can affect the
  historical backtest.
- The remaining sell failures were probed at 100%, 100%-1, 99.9%, 99%, 50%,
  25%, 20%, 10%, 5%, 2%, and 1% of the recorded entry amount. BCB2 fails at
  block `25,074,899` but later sells successfully at `25,074,918`. DF1A, BE74,
  and 2834 observed chain sellers simulate successfully with nonzero proceeds,
  while the strategy seller fails at every tested size, so those are
  address-specific restriction/whitelist cases rather than router-selector
  failures. One V2 token failed from 10% upward but succeeded at 5%, 2%, and 1%,
  so that case is max-transfer/anti-whale style. The Pancake V2 token fails
  every tested size down to 1% and has zero token-to-pool sell logs through
  current head. The V3 sell route is now Universal Router parity-compatible:
  the observed chain seller succeeds at block `25,075,896`, while the strategy's
  max-hold exit at block `25,076,700` fails with `V3InvalidSwap` because the
  pool has `liquidity() = 0` and only dust token/WETH balances after the
  liquidity-removal tx in that block.

## Proposed Goal: Redis-Free Joined Live Runtime

Objective: rename `eth_token_server` to `eth_chain_server` and run it as the
single live-chain runtime. It should subscribe to new heads, process blocks,
keep the latest block-scoped state in memory, apply token tracking from the
same `LiveBlockUpdate`, and serve recent processed-block APIs without Redis as
a live block or live state transport.

Recommended structure:

1. Rename the crate/binary/service/log namespace from `eth_token_server` to
   `eth_chain_server`. `eth_token` stays the token-domain crate;
   `eth_chain_server` is the broader runtime/API service that owns live and
   historical chain views, processed blocks, tokens, recent block read models,
   health, and HTTP APIs.
2. Add `LiveChainRuntime` as the chain-server owned orchestrator. It owns the
   `LiveBlockLoop`, `LiveTokenRuntime`, `RecentLiveBlocks`, `LiveStateView`,
   health reporters, and shutdown path.
3. Keep `LiveBlockLoop` close to the existing `LiveBlockProcessor`: subscribe
   to new heads, fetch RPC traces/state diffs, build the strict
   `ProcessedBlock`, and emit one in-memory `LiveBlockUpdate` per canonical
   block.
4. Define `LiveBlockUpdate` as the handoff object: block number/hash/timestamp,
   processed block, state diffs, processing timings, and the block-scoped
   simulation/metadata context needed by token tracking.
5. Refactor `LiveTokenRuntime` so warmup can still load historical blocks from
   disk/Reth, but the live tail consumes `LiveBlockUpdate` directly. The live
   tail must not use `RedisBlockStream` or `LiveProcessedBlockProvider`.
6. Move V2/V3/V4 metadata and token transaction simulation paths to use the
   current `LiveStateView` or the update's block-scoped context before falling
   back to slower historical providers.
7. Add `RecentLiveBlocks` as an in-memory ring buffer for recent processed
   blocks and compact summaries. Use it for APIs such as
   `GET /eth/tokens/api/live/processed-blocks?limit=5` and for read models that
   need recent blocks.
8. Migrate gas-rank and any tx-ranking read model away from the Redis
   `eth/live/blocks` stream. They should query `RecentLiveBlocks` for the latest
   processed block window and use the existing Reth/RPC providers only for
   sample details that are not already in memory.
9. Remove normal-runtime publication and reads of
   `eth/live/block/<n>/chain_state_snapshot`. Remove the standalone
   Redis-publisher live block processor from production service wiring; any
   remaining tests or local tools must not be a fallback transport for token
   server live mode.

Finished means:

1. The production crate/binary/service/log namespace is `eth_chain_server`.
   `eth_token_server` is removed from production wiring and docs, except for any
   explicit one-time migration note.
2. `eth_chain_server` starts one `LiveChainRuntime` for live mode; no separate
   Redis-mediated live block processor is required for token tracking.
3. Live token application after warmup is driven by `LiveBlockUpdate`, not by
   `RedisBlockStream`, Redis block keys, or `LiveProcessedBlockProvider`.
4. Normal chain-server runtime does not write or read
   `eth/live/block/<n>/chain_state_snapshot`, and Redis memory/key count for
   those snapshots does not increase while the service runs.
5. Gas-rank and tx-ranking consumers use `RecentLiveBlocks` or an HTTP/internal
   API for the recent processed block window instead of the Redis live-block
   stream.
6. The service config no longer requires `redis_url` or `live_block_stream` for
   live token tracking, live metadata, gas rank, or recent processed-block APIs.
   There is no Redis fallback in the chain-server live path.
7. `cargo check -p eth_chain_server`,
   `cargo test -p eth_live_feed -p tx_processor -p eth_chain_server`, and any
   focused live-runtime unit tests pass.
8. Runtime verification passes after a rebuild/restart: the service reaches
   live mode, advances at least 50 live blocks, `/live/status` and `/ops/health`
   remain healthy, `pipeline_health.jsonl` heartbeats continue, and structured
   WARN/ERROR logs do not show Redis-state metadata stalls.
9. Performance verification over that same live window shows V2 metadata lookup
   p95 below 100 ms and live token block apply p95 below 3 seconds, excluding
   clearly identified upstream RPC/Reth unavailability events.

Verification status on `2026-05-13`:

- Renamed the production crate/binary/service/log namespace to
  `eth_chain_server`.
- Added `LiveChainRuntime`: the chain server now owns the new-head loop,
  gap-fill processing, replay-store writes, direct `LiveBlockUpdate` handoff,
  and shutdown path.
- Removed Redis live block/state transport from chain-server live mode and
  removed the standalone Redis live-block processor from production systemd
  install wiring.
- Added the recent processed block API:
  `GET /eth/tokens/api/live/processed-blocks?limit=5`.
- Moved gas-rank recent block reads to the in-process `RecentLiveBlocks` ring.
- Added direct block-state sessions from live prestate diffs so current-block
  token simulations no longer read `eth/live/block/<n>/chain_state_snapshot`.
- Fixed the remaining live warning classes found during verification:
  direct live sessions now use fresh historical provider views on cache misses,
  carry recent live block hashes, rebuild from current prestate diffs across
  reorg parent mismatches, and skip speculative live V2 metadata reads for
  swap/sync-only unknown pools under the 250 ms budget.
- `verify9` on port `8766` advanced past 50 direct live blocks with
  `last_block_source="live_block_update"`, `tx_failures=0`, healthy
  `/ops/health`, no Redis state fallback strings, no direct-session warnings,
  no live V2 metadata timeout warnings, no pool simulation failure logs, no
  panic logs, and live token apply p95 `123 ms` over 52 live apply samples.
- The only WARN entries in `verify9` were startup-only address-index writer lock
  warnings caused by the existing local MDBX environment being unavailable; live
  block processing itself had zero structured pipeline issues.

## Next Actions

1. Implement indexed V3/V4 LP-position candidate lookup so ERC721 transfers and
   approvals no longer scan every tracked token/pool.
2. Re-run a longer chain-server soak after installing the service under the new
   `eth-chain-server.service`; acceptance target is zero block-local
   live-header/Reth-timeout token update warnings and live token block apply p95
   below 3 seconds.
3. Diagnose strict processed-block replay readiness failures without adding
   fallback processing; add regression checks for blocks `25,078,746` and
   `25,033,700`.
4. Fix live token transaction update funding gaps for mined txs with missing
   sparse-state sender balances.
5. Choose policy rules for the 8 classified failed sell exits and add
   regression artifacts for STRX/ROME/PERP balance-overlay valuation plus
   recipient-net sell-proceeds accounting.
6. Decide the entry-size policy for thin WETH pools: fixed-size failure,
   adaptive retry down to a minimum, or pre-entry size guard.
7. Rebuild the source observation/backtest path from current chain-server output
   that includes `runtime_state.can_buy/can_sell`, then confirm the six V4
   observed-flow-only entries disappear.
8. Keep sell-only balance setup strict: if recipient-transfer diff cannot prove
   `balanceOf(seller) >= recorded_entry_amount`, the simulator must fail loudly.
9. Recheck V3 pool mismatch/missing-pool warnings on the next long run now that
   SushiSwap V3 classification is fixed and the latest ops API is clean.
10. Re-run the chain-server profile summary after the candidate index fix; the
   acceptance target is candidate-selection p95 below 100ms on live blocks.
