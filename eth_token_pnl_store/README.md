# Token PnL Store

Crate: `eth_token_pnl_store`

PostgreSQL persistence for pool-scoped address PnL calculated by
`eth_token::pnl`.

## Ownership

`eth_token` owns calculation. This crate owns persistence, migrations, and query
helpers. Keep SQL, connection pools, and database config out of `eth_token`.

## Schema

The store creates schema `token_pnl` with four first-pass tables:

| Table | Purpose |
| --- | --- |
| `token_pnl.calculation_runs` | One historical or live calculation run, tagged by algorithm version and range. |
| `token_pnl.pool_pnl_states` | Pool-level rollup and conservation totals for a run. |
| `token_pnl.pool_address_pnl` | Address-level independent PnL rollup for one pool/run. |
| `token_pnl.pool_pnl_movements` | Append-only movement rows used to audit conservation and rebuild address rollups. |

Raw token/denom/native amounts are stored as `NUMERIC(78,0)` so they can hold
full `U256` values. Scaled floats are convenience columns only and must not be
treated as canonical.

## Runtime Shape

For historical replay:

```text
TokenStateBuilder
  -> ERC20Token.pnl.pool(...)
  -> PoolPnlTracker::export(...)
  -> TokenPnlStore::write_pool_export(...)
```

For live tracking, write the same exported shape after the live token runtime
applies a block or on a bounded flush interval.

## Configuration

The default config file is `blockchains/eth/config.toml`. The store reads
`databases.token_pnl.url` from that file. Examples accept `--config` when you
need to point at a different shared config file.

## Examples

Persist one Uniswap V2 pool replay:

```bash
cargo run -p eth_token_pnl_store --example persist_uniswap_v2_pool_pnl -- \
  --run-id <run_id> \
  --token <erc20> \
  --pool <uniswap-v2-pair> \
  --start <block> \
  --end <block> \
  --config /home/nima/code/crypto/blockchains/eth/config.toml
```

Inspect persisted address rollups:

```bash
cargo run -p eth_token_pnl_store --example inspect_pool_pnl_db -- \
  --run-id <run_id> \
  --pool <pool_id> \
  --config /home/nima/code/crypto/blockchains/eth/config.toml
```
