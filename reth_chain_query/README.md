# reth_chain_query

Agent operating map for direct Reth database queries, typed chain reads,
entity/DEX helpers, and lightweight indexes.

## Purpose

- Read Ethereum chain data directly from local Reth MDBX.
- Provide typed provider APIs for accounts, blocks, transactions, receipts,
  balances, tokens, entities, DEX state, and time conversion.
- Host RethIndex tables/writers for query shapes missing from Reth's canonical
  sequential storage.

## Owns

- `RethQueryProvider` and provider factories in `src/provider/`.
- Legacy `ChainQuery` compatibility in `src/query_engine.rs`.
- Entity/address catalogs in `src/entities/` and `src/common_addresses/`.
- DEX readers/helpers in `src/dex/`.
- Stateless AMM calldata builders in `src/tx_builders.rs`.
- PostgreSQL helpers and RethIndex tables/writers in `src/postgres_db/` and
  `src/reth_index/`.
- Block/time utilities in `src/utils/time_utils/`.

## Does Not Own

- Transaction simulation orchestration; use `tx_simulator`.
- Processed transaction semantics, decoded events, tax math, or block cache
  schemas; use `tx_processor`.
- Token lifecycle state; use `eth_token`.
- Mempool signal decisions or strategy policy.

## Data Flow

```text
Reth MDBX
  -> tx_simulator provider factory
  -> RethQueryProvider / ChainQuery
  -> typed reads, DEX/entity helpers, optional RethIndex/Postgres writes
  -> tx_processor, eth_token, eth_token_server, mempool_processor, pyreth
```

Builders are stateless: callers supply route/token/amount/deadline data, and
the builder returns calldata/spender information without DB mutation.

## Where To Look First

| Need | Start here |
| --- | --- |
| Public API and exports | `src/lib.rs` |
| Account/block/tx/receipt reads | `src/provider/` |
| RethIndex schemas and writers | `src/reth_index/` |
| PostgreSQL query helpers | `src/postgres_db/` |
| DEX state readers | `src/dex/` |
| AMM swap builders | `src/tx_builders.rs` |
| Address/entity catalogs | `src/common_addresses/`, `src/entities/` |
| Examples by query family | `examples/README.md` |

## Tests And Commands

```bash
cargo run -p reth_chain_query --example balances
cargo run -p reth_chain_query --example fetch_transaction_by_number
cargo run -p reth_chain_query --example verify_block_rpc_equivalence
cargo run -p reth_chain_query --example tx_arrival_index_smoke
cargo test -p reth_chain_query
```

## Current Hazards

- Keep DB reads typed and narrow. Do not turn query helpers into processing or
  simulation code.
- Historical account/storage reads need archive data; pruned nodes will fail or
  return incomplete history.
- RethIndex/Postgres helpers are optional side indexes. Do not make core direct
  reads depend on them unless the API explicitly says so.
- If adding a new AMM route, put route/spender/calldata builders here first,
  then call them from `tx_processor` or strategies.
