# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only active bottlenecks here:
what is limiting the pipeline, the evidence, the owning subsystem, and the next
action. Completed work belongs in focused docs or commit history.

## Current Runtime Snapshot

Snapshot time: `2026-05-14 10:10 Europe/Amsterdam`.

- Chain server user service: `eth-chain-server.service`, active PID `309313`.
- Chain server log run:
  `logs/eth_chain_server/run-20260514-064919Z-pid-309313`.
- Chain server live status: `live`, current block `25,092,085`, `832`
  tracked tokens, `499` tracked pools, `405` V2, `5` V3, `89` V4,
  `1` token tx failure, `last_block_source=live_block_update`.
- Chain server issue log:
  `run-20260514-064919Z-pid-309313/pipeline_issues.jsonl` has `1` row:
  a `token_transaction_update_failed` warning at block `25,091,919`, tx
  `0xd3612113d9f53737c1a6e2892456329d032c73f368e080ad28d9acc6b0067881`,
  because no Uniswap V3 pool was found for token
  `0x8Ef699477219710Ac4540919A374621f1f855510` against WETH at fee tier `100`.
- Mempool user service: `eth-mempool-signal-detector.service`, active PID
  `557236`.
- Mempool log run:
  `logs/mempool_processor/signal_detector_2026-05-14_10-08-44`.
- Mempool startup now logs:
  `Live data Redis: disabled; simulations use local Reth historical context`.
- Mempool token cache follows chain-server snapshots through HTTP. Latest
  observed fetch was `block=25092084, tokens=832, pools=499`; the external
  update log advanced through `@block 25092084`.
- Mempool simulation target now follows local Reth historical context. The new
  diagnostic line reports
  `Latest simulation block target: 25092083 source=LocalHistoricalContext reth_finished=25092083 historical_context=25092083 live_head=None tracked_state=None`.
- The old stale target `25091681` was caused by long-lived read-only
  `TxSimulator` instances not enabling Reth read-only sync, so their static-file
  header index stayed pinned to the block visible at process start.
- The repeated LP-position approval warning in the previous mempool run was for
  tx `0x229f8213791cc1f4b86711c174cca1bba28ca4e25ae1e733a670bd7cee3376db`,
  a Uniswap V4 PositionManager approval for token id `0x42422`. That token id
  was not present in the tracked position index, so no public LP-position
  approval signal could be enriched. The retry path is now kept out of WARN logs;
  `unresolved_intents.log` remains the record for these pending mappings.
- These are user systemd services. In non-login shells, use
  `XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user ...` to inspect them;
  plain system-level `systemctl` will not show these unit names.

## Active Bottlenecks

| Order | Bottleneck | Owner | Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Uniswap V3 pool identity miss in live token apply** | `eth_token`, `eth_chain_server` | The fresh chain-server run now has `tx_failures=1` and one `pipeline_issues.jsonl` row. At block `25,091,919`, token transaction apply failed because a Uniswap V3 pool for token `0x8Ef699477219710Ac4540919A374621f1f855510` / WETH / fee tier `100` was not present in the tracked registry when the token update needed it. | Reproduce that block from disk cache/Reth and inspect whether the V3 `PoolCreated` event was missed, filtered out by retention, mis-keyed by token orientation, or unavailable before the token tx. Decide whether this class should be strict failure or optional pending metadata. Chain-server soak should return to `tx_failures=0`. |
| 2 | **LP position approval mapping coverage** | `eth_token`, `eth_chain_server`, `mempool_processor` | The previous mempool run repeatedly retried a V4 PositionManager approval for token id `0x42422`, but the live pool cache had no tracked position context for that id. Current live pools expose 94 V3/V4 pools and only 13 pools with non-empty `liquidity_positions`, so many valid position approvals cannot be enriched into public LP-position approval signals. | Decide whether missing position mappings should stay as unresolved intents only, or whether chain-server should backfill position context on approval by querying the position manager for token id -> pool key/owner/liquidity. Keep public signals blocked unless a token/pool/share mapping is known. |
| 3 | **Residual Redis code outside the production chain-server path** | `tx_simulator`, `tx_processor`, `alpha/live/state`, docs | Production chain-server live token tracking no longer uses Redis and `TxSimulator::new()` no longer auto-attaches Redis. Mempool startup no longer accepts a Redis live cache. Legacy modules still exist: `tx_simulator::live_chain_cache`, `live_data_registry`, `tx_processor::live::{redis_block_publisher, block_notifier}`, `LiveProcessedBlockProvider`, and `alpha/live/state` Redis key contracts. | Decide which legacy Redis pieces are still needed for diagnostics or compatibility. Feature-gate or remove unused production exports after the mempool simulator freshness fix is in place. Update READMEs that still describe Redis as a normal runtime dependency. |
| 4 | **Mempool startup ordering** | `mempool_processor`, systemd units | On restart, mempool starts after the chain-server process, but before the HTTP listener is ready. It logs one `Connection refused` hydrate failure, then waits for token cache population and recovers. This is noisy and can delay startup diagnostics. | Add a readiness wait or health-check loop before first hydrate, or use a chain-server health endpoint in service startup. Keep the current retry path, but make the first connection-refused warning less alarming if startup is still inside the grace window. |
| 5 | **Strict processed-block replay readiness** | `tx_processor`, `tx_simulator`, `reth_chain_query` | Prior warmup/live replay hit mined transaction validation failures such as `lack of funds` when local state context lagged or was sparse. Current chain-server run has one V3 identity failure, but no stale-Reth validation failure. | Keep block processing strict. Add readiness/parity regression checks around historical context startup and known problematic blocks `25,078,746` and `25,033,700`. |
| 6 | **Token candidate-selection scaling** | `eth_token` | The current live run is mostly healthy, but the known algorithmic risk remains: V3/V4 ERC721 transfer/approval candidates can scan tracked registry state instead of using a position-manager/token-id index. This can reappear as tracked pool count grows. | Implement indexed V3/V4 LP-position and approval lookup keyed by position manager, token id, owner, and operator. Candidate selection should scale with events in the transaction, not total tracked tokens/pools. |
| 7 | **Strategy policy quality and concentration** | `alpha/strategies`, `alpha/engine` | Latest 15k baseline PnL is concentrated in a few winners. Excluding the top five positions previously moved PnL from positive to roughly flat/negative, so the broad baseline is still mostly measuring infrastructure and tail winners. | Continue the five-strategy 70k comparison work. For each strategy, inspect top 10 and worst 10 positions and preserve skip reasons, entry inputs, exit reasons, and PnL snapshots for frontend review. |
| 8 | **Sell restriction and exit policy** | `alpha/engine`, `tx_processor`, `tx_simulator` | Prior probes classified failed exits into retry-later, address-specific restriction, chunk-size/anti-whale, no observed sell evidence, and drained-liquidity V3 cases. Strategy-side position monitoring still needs continuous exit policy instead of one-shot max-hold behavior. | Keep monitoring continuous in strategy state. Add retry cadence, chunked exits, pool-approved exits, and no-observed-sell exposure limits as explicit strategies/policies. |
| 9 | **Decision ledger completeness** | `alpha/engine`, `alpha/store`, frontend | Execution reports now carry more fill context, but the strategy decision ledger still does not persist every rule input and skip/action reason needed for explainable frontend review. | Persist decisions keyed by run, strategy, token, pool, rule id, inputs, action, execution report, exit reason, and PnL snapshot. |
| 10 | **Execution handoff readiness** | `alpha/engine`, `tx_executor` | Real capital should remain blocked until live no-capital simulation freshness, strategy profitability, and decision auditing are stable. | Keep real execution separate. Only wire execution after live simulation freshness and strategy auditability are verified. |

## Redis Cleanup Status

Done:

- `eth_chain_server` owns the live block loop, direct `LiveBlockUpdate` handoff,
  live token runtime, recent processed-block ring, and live APIs.
- Chain-server live tail uses `last_block_source=live_block_update`; it does
  not tail `eth/live/blocks` or load live blocks through
  `LiveProcessedBlockProvider`.
- `TxSimulator::new()` and `TxSimulator::with_provider_factory()` no longer
  auto-attach Redis live state. Redis live cache is explicit opt-in via
  `with_default_live_chain_cache()` or `with_live_chain_cache(...)`.
- `mempool_signal_detector` no longer constructs a Redis `LiveChainCache` at
  startup and no longer has `simulation.live_data_redis_url` in its config.
- Long-lived `TxSimulator::new()` instances enable Reth read-only sync, so
  local historical simulation context can advance with the live Reth node
  instead of staying pinned to process-start static files.
- Redis-free optional metadata misses such as `live chain cache not configured`
  are classified as optional discovery misses, not live token transaction
  failures.
- Direct live pool trading simulation no longer falls back to the legacy live
  cache when a previous-block direct state session is unavailable; it skips that
  optional pool simulation and keeps the token block apply successful.

Still present by design or migration debt:

- `tx_simulator/src/block_context/live_chain_cache/` and
  `live_data_registry/` remain as legacy Redis snapshot helpers.
- `tx_processor/src/live/redis_block_publisher.rs`,
  `tx_processor/src/live/block_notifier.rs`, and
  `tx_processor/src/processed_tx_provider/block/live.rs` still implement the
  old Redis block transport.
- `alpha/live/state` still owns the old `eth/live/...` Redis key contract.
- Several READMEs and docs still describe Redis as a normal live-state path.

Do not delete the remaining Redis modules until all remaining consumers are
confirmed dead or explicitly migrated; otherwise we risk breaking diagnostics,
examples, or back-compat tooling.

## Verification Commands

Recent checks that passed during this cleanup:

- `cargo test -p eth_token token_update_router --lib`
- `cargo check -p eth_chain_server`
- `cargo check -p mempool_processor`
- `cargo build -p eth_chain_server -p mempool_processor --release --bin eth_chain_server --bin mempool_signal_detector`
- `cargo run -p tx_simulator --release --example verify_database_setup`

## Immediate Next Actions

1. Reproduce the block `25,091,919` V3 pool identity miss and restore
   chain-server soak expectations to `tx_failures=0` and `pipeline_issues=0`.
2. Decide the policy for unmapped V3/V4 position approvals: unresolved-only, or
   on-demand position-manager backfill before public signal publication.
3. Add a mempool readiness wait against chain-server HTTP health.
4. Run a longer chain-server and mempool soak:
   chain-server must keep `tx_failures=0` and `pipeline_issues=0`, while
   mempool simulation target lag should stay within the accepted threshold.
5. Remove or feature-gate legacy Redis modules only after the production
   mempool path no longer needs any Redis-compatible fallback.
6. Continue the 70k strategy comparison and frontend review using complete
   top-10/worst-10 position diagnostics.
