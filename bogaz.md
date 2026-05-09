# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only bottleneck management here:
what is currently limiting the pipeline, how we measured it, what owns it, and
the next action that moves or removes it.

## Operating Rule

A run is useful only if it tells us which limit dominates: live-state freshness,
cache fill/read speed, memory pressure, mempool signal recall, decision
auditing, fill modeling, strategy quality, or execution.

## Focus Order

| Order | Bottleneck | Owner | What To Watch | Next Focus |
| --- | --- | --- | --- | --- |
| 1 | Live feed readiness and failure isolation | `alpha/live/feed`, `eth_token_server`, `eth_token`, `tx_simulator` | live status, warmup progress, failed block, block apply time, simulation validation errors, V2/V3/V4 tracked-pool counters | Make live token apply resilient: optional pool metadata and buy/sell simulation failures must be recorded on the affected pool and must not fail the whole live tracker. |
| 2 | Live warmup memory pressure while filling processed-block cache | `eth_token_server`, `alpha/live/feed`, `tx_processor`, Reth static files | systemd cgroup memory, RSS, cgroup `anon`/`file`, disk-cache hits/misses, cache write time, Reth file mappings | Add allocator trimming to the live warmup path and avoid running large backfills while token-server warmup is filling missing cache entries. |
| 3 | Mempool signal recall and timing | `mempool_processor`, future `alpha/mempool_risk` | IPC drops, queue depth, arrival writes, first-seen timestamps, LP approvals before liquidity removals, V2/V3/V4 pool identity coverage | Improve early liquidity-removal detection across pool types. LP approval, removal intent, token, canonical `TokenPoolId`, and first-seen time must be persisted before the trader consumes them. |
| 4 | Trader decision ledger completeness | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | every decision input has `TokenPoolId`, market payload, signal payload, rule id, decision, order, execution report, exit reason, and PnL snapshot | Make every skip, entry, and exit auditable in Postgres so the frontend can explain strategy behavior. |
| 5 | Paper fill realism | `alpha/engine` | synthetic execution reports versus worst achievable block price | Replace placeholder paper fills with worst-case block fill modeling before trusting PnL. This belongs in `PaperExecutionAdapter`, not mempool risk. |
| 6 | Strategy policy quality | `alpha/strategies` | Snipe All v1 entries, exits, risk reactions, skipped candidates, V2/V3/V4 behavior | Keep `Snipe All v1` as the baseline and extend it with LP approval response, creator public/private labels, tax/honeypot response, position sizing, and pool filters. |
| 7 | Backtest and replay alignment | `alpha/backtest`, `alpha/engine` | same strategy state machine in historical and live paper runs, same `TokenPoolId` matching | Historical replay should use confirmed blocks only unless recorded mempool arrivals/signals exist. Compare historical lower-bound PnL to live paper behavior. |
| 8 | Real execution handoff | `alpha/engine`, `tx_executor` | adapter boundary, execution reports, nonce/gas failures, real order id to `TokenPoolId` mapping | Only replace the paper adapter with a `tx_executor` adapter after live state, signal recall, decision persistence, and fill modeling are measurable. |

## Frontend Alignment

ASENA renders this ledger at `/eth/bogaz/`. The page must keep the same order
as the table above and should only derive metrics from existing read-only APIs.

| Frontend Card | Source API | Metrics Shown |
| --- | --- | --- |
| Live Feed Readiness And Failure Isolation | `/eth/tokens/api/live/status`, `/eth/tokens/api/live/pools` | status, warmup, current block, apply time, processed-block source, tracked tokens, V2/V3/V4 pools, cache hit/miss, updated time, failures |
| Live Warmup Memory Pressure While Filling Processed-Block Cache | `/eth/tokens/api/live/status` | warmup, cache hit/miss, miss rate, cache read/write time, upstream time, apply time, tracked tokens/pools, memory API exposure status |
| Mempool Signal Recall And Timing | `/eth/tokens/api/mempool/signals?since_days=14&limit=500` | total signals, latest signal age, trading-enabled count, LP approvals, liquidity removals, approval/removal ratio, tax signals, token/pool coverage |
| Trader Decision Ledger Completeness | `/eth/trade/api/strategies/:strategy_id` | runtime, mode, trading flag, heartbeat age, live block/status, positions, orders/reports, risk count |
| Paper Fill Realism | `/eth/trade/api/strategies/:strategy_id` | report count, confirmed count, reports with block, reports with tx hash, gas modeled, fill errors |
| Strategy Policy Quality | `/eth/trade/api/strategies/:strategy_id` | active/scaffolded rules, open positions, risk counts by kind, latest risk, latest order |
| Backtest And Replay Alignment | `/eth/trade/api/runs`, `/eth/trade/api/strategies/:strategy_id` | range runs, completed range runs, latest range status/heartbeat, trader runs, latest trader status |
| Real Execution Handoff | `/eth/trade/api/strategies/:strategy_id` | mode, trading flag, reports with tx hash, adapter, run id, runtime status |

Frontend status semantics:

- `blocked`: source API is unavailable, live tracker failed, or a hard runtime
  error is present.
- `watch`: the stage is running but still incomplete, intentionally paper-only,
  or missing a required measurement such as memory diagnostics or worst-case
  fill modeling.
- `clear`: the stage has no currently known blocker for its role.

If this file changes the focus order, stage names, or monitored fields, update
`interface/asena/eth/static/bogaz.js` in the same change.

## Active Bottleneck: Live Warmup Memory Pressure While Filling Processed-Block Cache

Observed on May 9, 2026 while `eth-token-server.service` was warming live token
state and filling missing processed-block disk-cache entries.

Measured service state:

```text
eth-token-server.service
  status: warming
  progress: 2525 / 7000 warmup blocks
  current block: 25045525
  processed txs: 686,908
  tracked tokens: 186
  tracked pools: 27
  cache hits: 0
  cache misses: 2525
```

Memory accounting:

```text
systemd Memory:      ~7.9G / 8G
process RSS:         ~3.4G
cgroup memory.current: ~8.6G
cgroup anon:         ~2.2G
cgroup file:         ~5.8G
cgroup inactive_file: ~5.6G
cgroup pagetables:   ~454M
cgroup kernel/slab:  ~587M
```

Interpretation:

- The dominant cgroup charge is file cache, not canonical token registry size.
- The service maps and reads large Reth static files, RocksDB/MDBX files, and
  processed-block cache files while cache misses are being filled.
- `inactive_file` is mostly reclaimable by the kernel, but it still counts
  against the service cgroup `MemoryMax=8G`.
- The anonymous/heap side is still meaningful at roughly `2.2G`.
- No historical range run was active when measured; `/runs` returned empty.
- Current tracked token/pool counts were too small to explain the 8G cgroup
  footprint by themselves.

Relevant code paths:

- Live warmup loads or fills one processed block in
  `alpha/live/feed/src/runtime/service.rs`.
- The live runtime clones `LiveBlockTokenProcessor` before applying each block,
  then restores it after apply. At the current token count this is not the main
  consumer, but it can amplify allocator retention as the tracked set grows.
- Historical range runs call `eth_token_server::memory::trim_allocator()` after
  chunks. The live warmup path does not currently trim the allocator.

Immediate operating rule:

- Do not run a large processed-block backfill while `eth-token-server` warmup is
  filling cache misses under the current `8G` service cap.
- Prefer waiting for warmup to finish, stopping live tracking, or temporarily
  increasing the service memory cap before large cache-fill work.

Next actions:

| Priority | Action | Owner | Validation |
| --- | --- | --- | --- |
| 1 | Add allocator trimming to the live warmup path after block apply or every N warmup blocks. | `alpha/live/feed`, `eth_token_server` | Compare process RSS/anon before and after 500 warmup blocks. |
| 2 | Add memory diagnostics to the token-server cache page or `/live/status`: process RSS, cgroup current, anon, file, inactive_file. | `eth_token_server`, Asena | Cache page shows why MemoryMax is high without shell access. |
| 3 | Run a controlled 1K cached-read warmup after cache is already populated. | `eth_token_server` | Cache hits should dominate and cgroup file growth should be lower than miss/fill warmup. |
| 4 | Measure whether `LiveBlockTokenProcessor` cloning becomes significant at larger tracked token counts. | `alpha/live/feed`, `eth_token` | Record clone/apply/restore time and heap/RSS deltas as tokens grow. |
| 5 | Decide whether token-server warmup and bulk cache backfill should run in separate service profiles with different `MemoryMax`. | systemd config | Backfill cannot OOM or starve the live token server. |

## Processed-Block Cache Baseline

Current cache layout objective:

```text
processed-block-cache/
  ethereum-mainnet/
    <block_number>.pblock.zst
```

Recent measured numbers after switching to the flat binary layout:

```text
single fresh block fill:
  block: 25050002
  txs: 507
  EVM/process: 512.7 ms
  cache write: 12.0 ms
  immediate read: 6.6 ms

single cached block read:
  block: 25050002
  read: 5.3 ms

100-block cached range:
  range: 25043001-25043100
  cache hit rate: 100%
  plan time: 0.255 ms
  parallel read wall time: 56.8 ms
  wall avg: 0.57 ms/block
  individual read p50/p95/max: 3.16 / 6.51 / 9.02 ms
```

What this means:

- Cache read planning by block number is not currently the bottleneck.
- Cached range replay is fast enough for warmup once blocks are present.
- The expensive path is cache miss fill: Reth block processing plus file-cache
  pressure from Reth static files and DB access.

## Live Feed Readiness And Failure Isolation

The live token tracker must reach `live` reliably before paper trading can
produce dependable measurements.

Known failure class:

- Simulation validation errors, such as insufficient simulated funds, can fail
  runtime warmup if treated as block-level errors.

Target behavior:

- Optional pool metadata failures and buy/sell simulation failures should be
  recorded on the affected pool.
- They should not fail the whole live tracker.
- Runtime failure should be reserved for unrecoverable processed-block loading,
  ordering, or state corruption errors.
