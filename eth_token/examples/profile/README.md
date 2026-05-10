# eth_token Profile Examples

This folder keeps repeatable commands for profiling token tracking behavior.

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
