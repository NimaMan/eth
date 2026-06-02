# ETH Token Store

Crate: `eth_token_store`

PostgreSQL persistence for ETH token read models calculated by `eth_token`.
This crate is the single persistence boundary for:

- pool-scoped address PnL in schema `token_pnl`
- latest token and token-pool state in schema `token_state`

## Ownership

`eth_token` owns calculation and in-memory tracking. This crate owns SQL,
migrations, connection pools, and database config for persisted token read
models. Keep database concerns out of `eth_token`.

## Schema

### `token_pnl`

Pool-scoped address PnL calculated by `eth_token::pnl`.

| Table | Purpose |
| --- | --- |
| `token_pnl.calculation_runs` | One historical or live calculation run, tagged by algorithm version and range. |
| `token_pnl.pool_pnl_states` | Pool-level rollup and conservation totals for a run. |
| `token_pnl.pool_address_pnl` | Address-level independent PnL rollup for one pool/run. |
| `token_pnl.pool_pnl_movements` | Append-only movement rows used to audit conservation and rebuild address rollups. |

Raw token/denom/native amounts are stored as `NUMERIC(78,0)` so they can hold
full `U256` values. Scaled floats are convenience columns only and must not be
treated as canonical.

### `token_state`

Latest token and token-pool state.

| Table | Purpose |
| --- | --- |
| `token_state.token_latest` | One row per `(scope_id, chain_id, token_address)` with latest token lifecycle/activity/scam summary. |
| `token_state.pool_latest` | One row per `(scope_id, chain_id, pool_id)` with latest pool reserves, lifecycle, valuation status, trading flags, and liquidity-removal state. |

`scope_id` separates live state from historical as-of runs. Live writers should
use `live`. Historical examples use `historical:<run_id>` so they do not
overwrite live rows.

## Runtime Shape

For historical replay:

```text
TokenStateBuilder
  -> ERC20Token.pnl.pool(...)
  -> PoolPnlTracker::export(...)
  -> TokenPnlStore::write_pool_export(...)
  -> TokenStateStore::write_token_latest(...)
  -> TokenStateStore::write_pool_latest(...)
```

For live tracking, write the same exported shape after the live token runtime
applies a block or on a bounded flush interval.

## Configuration

The default config file is `blockchains/eth/config.toml`. The store reads
`databases.token_pnl.url` and `databases.token_state.url` from that file.
Examples accept `--config` when you need to point at a different shared config
file.

## Examples

Persist one Uniswap V2 pool replay:

```bash
cargo run -p eth_token_store --example persist_uniswap_v2_pool_pnl -- \
  --run-id <run_id> \
  --token <erc20> \
  --pool <uniswap-v2-pair> \
  --start <block> \
  --end <block> \
  --config /home/nima/code/crypto/blockchains/eth/config.toml
```

Inspect persisted address rollups:

```bash
cargo run -p eth_token_store --example inspect_pool_pnl_db -- \
  --run-id <run_id> \
  --pool <pool_id> \
  --config /home/nima/code/crypto/blockchains/eth/config.toml
```

Run a historical aggregate sweep with the `terminal_or_idle_50k` retention
policy. The block count is a runtime parameter; it defaults to 50,000 blocks:

```bash
cargo run -p eth_token_store --example run_historical_pnl -- \
  --run-id <run_id> \
  --blocks 50000 \
  --chunk-blocks 250 \
  --retention-every-blocks 250
```

This example writes pool/address aggregate snapshots only. It does not persist
per-movement rows; movement details remain rebuildable from the processed block
cache or chain.

## Assessment

Use `docs/pnl_db_assessment.md` to assess whether a run is suitable for
user-facing PnL. Conservation passing is necessary, but not sufficient: address
roles, entity grouping, valuation quality, and retention behavior must also be
understood before calling this v1.
