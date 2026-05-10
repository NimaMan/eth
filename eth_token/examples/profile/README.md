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

The helper script starts an isolated `eth_token_server`, runs one historical
range-index job, waits for completion, and summarizes
`token_pipeline_profile.log`.

Defaults match the range used while measuring the post-block simulation change:

```bash
START_BLOCK=25052270 END_BLOCK=25059269 \
  blockchains/eth/eth_token/examples/profile/run_token_pipeline_profile.sh
```

Useful overrides:

```bash
TOKEN_PROFILE_BIND=127.0.0.1:8766
TOKEN_PROFILE_LOG_DIR=/home/nima/code/crypto/blockchains/eth/logs/eth_token_server_profile_post_block
ETH_CONFIG_PATH=/home/nima/code/crypto/blockchains/eth/config.env
TOKEN_PROFILE_RUST_LOG=info
```

The script writes a temporary config to
`/tmp/eth_token_server_profile_post_block.env` by copying the shared config and
overriding only the isolated bind address, log directory, and live auto-start
setting.

After the run, the per-block CSV is written to:

```text
/tmp/token_pipeline_profile_<run_id>_<start>_<end>.csv
```

## Captured Result: 25052270-25059269

Captured on 2026-05-10 after commit `9b72177`.

This profile is intentionally block-level. It measures the real pipeline shape:
apply decoded transaction facts for each mined block, then simulate affected
pools from that block's post-state. It does not use range-level final-state
shortcuts.

Reproduce:

```bash
TOKEN_PROFILE_RUST_LOG='warn,token_range_apply_profile=info,token_block_processor_profile=info,token_sim_session_profile=info' \
TOKEN_PROFILE_BIND=127.0.0.1:8767 \
TOKEN_PROFILE_LOG_DIR=/home/nima/code/crypto/blockchains/eth/logs/eth_token_server_profile_post_block_8767_final \
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
/home/nima/code/crypto/blockchains/eth/logs/eth_token_server_profile_post_block_8767_final/token_pipeline_profile.log.2026-05-10
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
