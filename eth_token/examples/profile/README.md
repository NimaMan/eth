# eth_token Profile Examples

This folder keeps repeatable commands for profiling token tracking behavior.

## Block-Level Scope

The token tracker should be profiled and optimized at mined-block granularity.
That matches the live system: blocks arrive one at a time, decoded transaction
facts are applied in block order, and pool trading checks should describe the
post-state of that block.

For historical range runs, do not collapse simulation across the whole range as
the default behavior. Range-level final-state simulation can be a separate
analysis mode later, but it does not replicate the real block-by-block pipeline.

The current historical block flow is:

```text
for tx in block order:
  apply token and pool facts
  collect pending pool simulations

after the block:
  coalesce pending simulations by (token, pool_kind, pool_id)
  open/reuse one post-block state session for that block
  simulate each affected pool once
```

## 7K Token Pipeline Profile

The helper script starts an isolated `eth_chain_server`, runs one historical
range-index job, waits for completion, and summarizes
`token_pipeline_profile.jsonl` from that process run directory.

Defaults match the range used while measuring the post-block simulation change:

```bash
START_BLOCK=25052270 END_BLOCK=25059269 \
  blockchains/eth/eth_token/examples/profile/run_token_pipeline_profile.sh
```

Useful overrides:

```bash
TOKEN_PROFILE_BIND=127.0.0.1:8766
TOKEN_PROFILE_LOG_DIR=/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server_profile_post_block
TOKEN_PROFILE_LOG_RUN_ID=profile_25052270_25059269
TOKEN_PROFILE_BASE_CONFIG=/home/nima/code/crypto/blockchains/eth/config.env
TOKEN_PROFILE_CONFIG=/tmp/eth_chain_server_profile_post_block.env
TOKEN_PROFILE_DEFAULT_BLOCKS=7000
```

The script writes a temporary config to
`/tmp/eth_chain_server_profile_post_block.env` by copying the shared config and
overriding only the isolated bind address, log root, log run id, and live
auto-start setting. The profiled `eth_chain_server` is launched with
`--config <temporary-config>`, so runtime values still come from a config file
instead of ambient process environment variables.

The process log files are written under:

```text
<TOKEN_PROFILE_LOG_DIR>/<TOKEN_PROFILE_LOG_RUN_ID>/
```

The profile summary reads:

```text
<TOKEN_PROFILE_LOG_DIR>/<TOKEN_PROFILE_LOG_RUN_ID>/token_pipeline_profile.jsonl
```

After the run, the per-block CSV is written to:

```text
/tmp/token_pipeline_profile_<run_id>_<start>_<end>.csv
```

## Captured Partial Result: 25018863-25088862

Captured on 2026-05-14 from the active 70K token-builder run:

```text
log=/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server/run-20260514-064919Z-pid-309313/token_pipeline_profile.jsonl
csv=/tmp/token_pipeline_profile_run-1_partial.csv
processed_blocks=22,950 / 70,000
range=25,018,863 - 25,088,862
```

Summary:

```text
disk_cache_read=83.085s total, 3.62ms avg/block, p95=7ms, p99=9ms, max=32ms
block_apply_wall=165.961s total, 7.23ms avg/block, p95=19.86ms, p99=52.04ms, max=251.98ms
token_apply=164.999s total, 7.19ms avg/block, p95=19.80ms, p99=52.00ms, max=251.91ms
token_apply_minus_simulation=71.112s total, 3.10ms avg/block, p95=7.87ms, p99=12.67ms
state_mutation_no_candidate_no_sim=6.640s total, 0.29ms avg/block, p95=0.90ms, p99=1.68ms
simulation_total=93.886s total, 4.09ms avg/block, p95=14.53ms, p99=45.98ms
candidate_routing=42.720s total, 1.86ms avg/block, p95=5.92ms, p99=10.90ms
```

Interpretation:

The token aggregate mutation itself is not the bottleneck. On this sample it is
about `0.29ms/block`; even at 70K blocks that is only about 20 seconds. The
meaningful per-block costs are processed-block cache read/deserialization,
candidate routing, and post-block pool simulation.

Projected 70K cost from this partial run:

```text
disk cache reads only:          ~4.2 min
token apply only:               ~8.4 min
cache read + token apply:       ~12.6 min
token apply without simulation: ~3.6 min
state mutation only:            ~20 sec
```

This means a heavy persisted token-env store is not justified only to avoid
token-state rebuild cost. A lightweight in-memory range environment over cached
processed blocks is the better first step.

## Processed-Block Cache Placement

Current config on 2026-05-14:

```text
PROCESSED_BLOCK_DISK_CACHE_DIR=/home/nima/storage/samsung8tb/ethereum/processed-block-cache
cache_size=8.24GiB
cache_files=50,216
avg_file_size=172KiB
mount=/home/nima/storage/samsung8tb
device=Samsung SSD 9100 PRO 8TB
```

The root filesystem is mounted on the 4TB Gen5 drive:

```text
mount=/
device=CT4000T705SSD5
free_space=~1.1TiB
```

Tradeoff estimates using the partial 70K profile:

```text
Keep processed blocks in RAM:
  best possible saving is the measured disk_cache_read cost.
  upper-bound saving: ~3.62ms/block, ~4.2 min over 70K blocks.
  projected cache+apply time drops from ~12.6 min to ~8.4 min.

Move cache to the 4TB Gen5 root SSD:
  current cache is already on a fast NVMe SSD, so gains may be modest.
  if reads drop from 3.62ms to 1.8ms/block, saving is ~2.1 min over 70K.
  if reads drop to 1.0ms/block, saving is ~3.1 min over 70K.
  benchmark before moving; this timing includes file open, decompression, and
  deserialization, not only raw storage latency.
```

For repeated strategy comparison, the stronger optimization is to build the
token range once and keep that range state in memory while multiple strategies
consume it. RAM-caching processed blocks helps rebuild speed, but it does not
remove candidate routing or pool simulation.

### 1K Cache Placement A/B

Captured on 2026-05-14 using the same contiguous 1K processed-block slice:

```text
range=25,011,176 - 25,012,175
source=/home/nima/storage/samsung8tb/ethereum/processed-block-cache
gen5_copy=/home/nima/eth-processed-block-cache-gen5-profile
copied_size=139MiB
```

Command shape:

```bash
cargo run --manifest-path blockchains/eth/Cargo.toml \
  -p eth_chain_server --release \
  --example processed_block_disk_cache_size -- \
  --cache-dir <cache-dir> \
  --start 25011176 \
  --end 25012175 \
  --read-only
```

Four alternating read-only passes:

```text
Samsung 9100 PRO 8TB cache:
  avg read_ms/block:    3.307
  median read_ms/block: 2.885
  p95 read_ms/block:    6.793
  wall_ms/block:        0.472

4TB Gen5 root SSD copy:
  avg read_ms/block:    3.405
  median read_ms/block: 3.022
  p95 read_ms/block:    7.067
  wall_ms/block:        0.489
```

Interpretation:

Moving the processed-block cache from the current 8TB Samsung NVMe to the 4TB
Gen5 root SSD did not improve this workload. The difference is within normal
run-to-run noise, and the current cache was slightly faster on this sample. The
measured cache-read cost is likely dominated by many small-file opens,
decompression, deserialization, and OS page-cache behavior more than raw SSD
bandwidth.

RAM can still save at most the cache-read component. For this 1K slice, that is
about `3.3ms/block` by individual read timing, while the parallel wall-clock
read path is about `0.47-0.49ms/block`.

## V2 Candidate Routing Profile: 25011176-25012175

Captured on 2026-05-14 after splitting V2 candidate routing into a cheap pool
identity path and a full pool metadata path.

```bash
START_BLOCK=25011176 \
END_BLOCK=25012175 \
TOKEN_PROFILE_BIND=127.0.0.1:8771 \
TOKEN_PROFILE_LOG_DIR=/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server_profile_v2_candidate_fix \
TOKEN_PROFILE_LOG_RUN_ID=profile_v2_candidate_fix_1k_<timestamp> \
  blockchains/eth/eth_token/examples/profile/run_token_pipeline_profile.sh
```

Output CSV:

```text
/tmp/token_pipeline_profile_run-1_25011176_25012175.csv
```

The relevant before/after comparison on the same 1K block range:

```text
before:
  router=5.00ms/block avg
  candidate routing=4.87ms/block avg
  V2 metadata lookup path=4.64ms/block avg

after:
  router=2.79ms/block avg
  candidate routing=2.58ms/block avg
  V2 identity lookup path=2.36ms/block avg
```

The fixed requirement is that candidate routing only needs V2 pool identity:
`token0`, `token1`, and validated known V2 protocol. Full metadata, especially
token decimals, is only needed when the pool is actually registered on a tracked
token. The candidate path also validates known V2 pairs with local CREATE2
address calculation instead of simulating factory `getPair(token0, token1)`.

The important remaining counts from the after profile:

```text
V2 pool events=12,443
V2 identity lookup attempts=10,742
V2 identity hits=10,331
transfer-routed skips=52
known-pool/index skips=1,649
candidate tokens=3,553
update reports=3,961
```

Transfer-based routing was therefore not the main win: it skipped only 52 of
10,742 identity attempts. The large improvement came from making the fallback
identity lookup cheaper by removing decimals and simulated factory reads from
candidate routing.

### Remaining V2 Candidate Patterns

The same 1K CSV shows a few follow-up opportunities:

```text
registry_tokens=0:
  blocks=26
  identity_lookups=335
  identity_time=183ms

registry_tokens<=5:
  blocks=79
  identity_lookups=1,059
  identity_time=454ms

candidate_tokens=0:
  blocks=160
  identity_lookups=1,579
  identity_time=477ms

indexed_pools=0:
  blocks=28
  identity_lookups=367
  identity_time=192ms
```

The safest small optimization would be to skip unknown-V2-pool identity lookup
when there are no tracked tokens in the registry/index. With no tracked token,
no V2 pool identity can produce a candidate. On this 1K slice that would remove
335 identity attempts and about 183ms.

The broader pattern was repeated unknown V2 pools that return a valid identity
but do not resolve to a tracked token. The candidate router now keeps a
candidate-level cache for two facts:

- pool identity, so repeated V2 events for the same pair can route from
  `token0`/`token1` without another identity-provider call;
- "irrelevant for the current candidate set", keyed by token-index generation
  and registry token count, so a pool that cannot currently route to a token is
  rechecked when new tokens appear or retention changes the tracked set.

This cache is intentionally separate from the chain metadata cache. Metadata
cache answers "what is this pool?"; candidate cache answers "can this pool
currently route to a tracked token?"

Final 1K profile after adding the candidate cache:

```text
V2 pool events=12,443
V2 identity lookup attempts=1,949
V2 identity hits=1,538
V2 identity cache hits=3,577
irrelevant cache hits=5,216
irrelevant cache inserts=5,115
transfer-routed skips=52

router=3.24ms/block avg
candidate routing=3.01ms/block avg
V2 identity lookup path=2.77ms/block avg
```

Compared with the post-identity-split profile, identity-provider calls dropped
from 10,742 to 1,949. Wall time did not improve on this sample because the
removed calls were mostly cheap chain-metadata-cache hits; the remaining
first-time pool identity reads still dominate the V2 candidate path. Treat this
cache as correctness and provider-churn cleanup, not as the primary speed fix.
The next meaningful speed work should target first-time identity reads or avoid
the V2 identity path entirely when no tracked token can be affected.

## Captured Result: 25052270-25059269

Captured on 2026-05-10 after commit `9b72177`.

This profile is intentionally block-level. It measures the real pipeline shape:
apply decoded transaction facts for each mined block, then simulate affected
pools from that block's post-state. It does not use range-level final-state
shortcuts.

Reproduce:

```bash
TOKEN_PROFILE_BIND=127.0.0.1:8767 \
TOKEN_PROFILE_LOG_DIR=/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server_profile_post_block_8767_final \
TOKEN_PROFILE_LOG_RUN_ID=profile_25052270_25059269_final \
START_BLOCK=25052270 \
END_BLOCK=25059269 \
  blockchains/eth/eth_token/examples/profile/run_token_pipeline_profile.sh
```

Output CSV:

```text
/tmp/token_pipeline_profile_run-1_25052270_25059269.csv
```

Profile log:

```text
/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server_profile_post_block_8767_final/profile_25052270_25059269_final/token_pipeline_profile.jsonl
```

Summary:

```text
blocks=7000
profile window=76.5s
block_apply_wall=47.725s total, 6.82ms avg/block, p95=18ms, p99=70ms, max=261ms
token_apply=47.599s
router=13.652s
candidate routing=11.569s
simulation_total=25.236s
state_update=0.116s
take_processor=0.003s
```

Simulation detail:

```text
candidate pools=55,946
simulated pools=3,278
historical session creates=2,396
historical branches=3,278
max simulations in one block=7
historical session create time=91ms total
historical branch clone time=3ms total
```

By pool kind:

```text
v2 simulations=1,272, total=5.647s, avg=4.44ms
v3 simulations=116, total=2.544s, avg=21.93ms
v4 simulations=1,890, total=17.044s, avg=9.02ms
```

1K buckets:

```text
blocks 1-1000:    wall=6.760s avg=6.8ms p95=17ms sim=2.717s candidates=3,775  simulated=303
blocks 1001-2000: wall=4.633s avg=4.6ms p95=13ms sim=1.890s candidates=5,255  simulated=318
blocks 2001-3000: wall=4.650s avg=4.7ms p95=12ms sim=2.215s candidates=2,994  simulated=291
blocks 3001-4000: wall=5.039s avg=5.0ms p95=12ms sim=2.630s candidates=3,660  simulated=255
blocks 4001-5000: wall=9.954s avg=10.0ms p95=23ms sim=6.666s candidates=12,453 simulated=893
blocks 5001-6000: wall=8.427s avg=8.4ms p95=20ms sim=4.398s candidates=17,369 simulated=540
blocks 6001-7000: wall=8.262s avg=8.3ms p95=20ms sim=4.720s candidates=10,440 simulated=678
```

Interpretation:

The previous slow path replayed intra-block prefixes and could simulate the same
pool many times in a single mined block. The current block-level path removes
that cost by deferring simulation until the block is applied and coalescing
within that block.

The remaining time is dominated by real pool simulations, especially V4, plus
candidate routing. Since `simulation_total` is 52.9% of wall time, even deleting
all simulation would only improve this run by about 2.1x. A second 10x is not
available inside this measured block-level path without changing semantics.

The next block-level candidates are:

```text
1. Profile V4 buy/sell simulation internals. V4 is 17.044s of the 25.236s simulation total.
2. Reduce candidate routing work. Candidate routing is 11.569s and visits 77,213 token candidates.
3. Add block-level skip rules for repeated failed pools only when the rule is semantically safe for the next block.
```

Do not make the default profile faster by simulating each pool once at the end
of the whole range. That is a different range-analysis mode, not the live
block-by-block pipeline.

## Captured Live Warmup Result: 7K Request

Captured on 2026-05-10 after adding `live_token_apply_profile`.

Run:

```text
CHAIN_SERVER_LOG_RUN_ID=live_warmup_7000_profile_1778416273
CHAIN_SERVER_BIND=127.0.0.1:8768
LIVE_TOKEN_TRACKER_WARMUP_BLOCKS=7000
```

Profile log:

```text
/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server/live_warmup_7000_profile_1778416273/token_pipeline_profile.jsonl
```

The run was stopped after 1,423 warmup blocks because the bottleneck was already
clear and the remaining 7K run would only extend the same slope.

Summary:

```text
live_profile_rows=1423
profile window=300.0s
block_apply_wall=300.020s total, 210.84ms avg/block, p95=377.85ms, p99=401.60ms, max=452.20ms
clone_processor=135.709s total, 95.37ms avg/block, 45.2% of wall
restore_processor=154.441s total, 108.53ms avg/block, 51.5% of wall
token_apply=9.868s total, 6.94ms avg/block, 3.3% of wall
disk_cache_read=6.913s total, 4.86ms avg/block
```

250-block buckets:

```text
blocks 1-250:     wall=4.672s  clone=1.517s  restore=1.659s  token=1.496s  tokens=39  pools=13
blocks 251-500:   wall=24.317s clone=10.398s restore=12.206s token=1.713s  tokens=54  pools=20
blocks 501-750:   wall=53.313s clone=24.034s restore=27.550s token=1.728s  tokens=74  pools=34
blocks 751-1000:  wall=70.864s clone=32.259s restore=37.097s token=1.508s  tokens=91  pools=44
blocks 1001-1250: wall=82.583s clone=37.902s restore=42.731s token=1.950s  tokens=115 pools=55
blocks 1251-1423: wall=64.271s clone=29.599s restore=33.198s token=1.474s  tokens=130 pools=63
```

Interpretation:

The live warmup slowdown is not processed-block cache read time and not token
block processing. The current live runtime clones the full
`LiveBlockTokenProcessor` before every block apply, then restores it into
`LiveTokenState`, dropping the previous full processor value. That copy/drop
cycle grows as tracked token/pool/network state grows and dominates live warmup.

This was fixed by applying live blocks to one in-place processor. Heavy read
endpoints may wait for the current block apply to finish, which is the right
tradeoff until we add lower-cadence view snapshots.

## Captured Live Warmup Result: In-Place Processor

Captured on 2026-05-10 after removing per-block processor clone/restore from
`LiveTokenRuntime`.

Run:

```text
CHAIN_SERVER_LOG_RUN_ID=live_warmup_7000_inplace_1778417096
CHAIN_SERVER_BIND=127.0.0.1:8768
LIVE_TOKEN_TRACKER_WARMUP_BLOCKS=7000
```

Profile log:

```text
/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server/live_warmup_7000_inplace_1778417096/token_pipeline_profile.jsonl
```

Summary:

```text
live_profile_rows=7008
warmup rows=7000
warmup elapsed from server status=83.4s until live
warmup block_apply_wall=59.708s total, 8.53ms avg/block, p95=19.86ms, p99=70.75ms, max=190.99ms
warmup token_apply=53.508s total, 7.64ms avg/block
warmup state_update=6.191s total, 0.88ms avg/block
warmup disk_cache_read=18.650s total, 2.66ms avg/block
state_lock_wait=0.000s total
```

1K buckets:

```text
blocks 1-1000:    wall=5.493s  token=4.890s  disk=2.315s  update=0.601s  tokens=87  pools=43
blocks 1001-2000: wall=7.469s  token=6.722s  disk=2.541s  update=0.746s  tokens=187 pools=108
blocks 2001-3000: wall=6.434s  token=5.650s  disk=2.477s  update=0.782s  tokens=294 pools=169
blocks 3001-4000: wall=5.522s  token=4.712s  disk=2.468s  update=0.808s  tokens=357 pools=208
blocks 4001-5000: wall=10.845s token=9.883s  disk=2.707s  update=0.960s  tokens=445 pools=274
blocks 5001-6000: wall=11.298s token=10.179s disk=2.927s  update=1.117s  tokens=557 pools=345
blocks 6001-7000: wall=12.648s token=11.470s disk=3.215s  update=1.176s  tokens=661 pools=410
```

The run entered live mode and then caught up 8 live-tail blocks. The profile
shows the clone/restore bottleneck was removed; remaining warmup cost is now
normal token block work plus processed-block cache reads.
