# reth_chain_query

Agent operating map for direct Reth database queries, typed chain reads,
contract metadata helpers, address identification, entity/DEX helpers, live
chain helpers, and lightweight indexes.

## Purpose

- Read Ethereum chain data directly from local Reth MDBX.
- Provide typed provider APIs for accounts, blocks, transactions, receipts,
  balances, tokens, entities, DEX state, and time conversion.
- Detect contract metadata such as ERC20 identity using direct reads and
  scoped view-call simulation.
- Classify important addresses through a single canonical address catalog.
- Host RethIndex tables/writers for query shapes missing from Reth's canonical
  sequential storage.

## Owns

- `RethQueryProvider` and provider factories in `src/provider/`.
- `ChainQuery` facade in `src/chain_query.rs`.
- Contract read helpers and ERC20 metadata detection in `src/contracts/`.
- Pending transaction replay that is only needed to enrich token metadata reads
  in `src/contracts/erc20/pending_replay.rs`.
- Canonical known-address catalogs and identification helpers in
  `src/common_addresses/`.
- Entity analysis helpers in `src/entities/`.
- DEX readers/helpers in `src/dex/`.
- Live execution/beacon head helpers in `src/live_chain/`.
- PostgreSQL helpers and RethIndex tables/writers in `src/postgres_db/` and
  `src/reth_index/`.
- Block/time utilities in `src/utils/time_utils/`.

## Does Not Own

- Transaction simulation orchestration; use `tx_simulator`.
- Stateless AMM calldata builders; use `tx_simulator::tx_builders`.
- Processed transaction semantics, decoded events, tax math, or block cache
  schemas; use `tx_processor`.
- Token lifecycle state; use `eth_token`.
- Mempool signal decisions or strategy policy.

## Data Flow

```text
Reth MDBX
  -> tx_simulator provider factory
  -> RethQueryProvider / ChainQuery
  -> typed reads, contract metadata, address identification, DEX/entity helpers,
     optional RethIndex/Postgres writes
  -> tx_processor, eth_token, eth_chain_server, mempool_processor, pyreth
```

Known addresses are centralized under `src/common_addresses/`; callers can use
`identify_known_address` to classify stablecoins, denom tokens, CEX/ETF
addresses, fee recipients, DEX factories/routers, wallets, and other named
addresses. Entity modules consume those catalogs for analysis; they should not
own duplicate address lists.

## Where To Look First

| Need | Start here |
| --- | --- |
| Public API and exports | `src/lib.rs` |
| ChainQuery facade | `src/chain_query.rs` |
| Account/block/tx/receipt reads | `src/provider/` |
| Contract/ERC20 metadata detection | `src/contracts/` |
| Pending replay for token metadata | `src/contracts/erc20/pending_replay.rs` |
| RethIndex schemas and writers | `src/reth_index/` |
| PostgreSQL query helpers | `src/postgres_db/` |
| DEX state readers | `src/dex/` |
| Live chain head/beacon helpers | `src/live_chain/` |
| AMM swap builders | `tx_simulator/src/tx_builders/` |
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
- Keep pending replay scoped to token metadata enrichment. General transaction
  simulation belongs in `tx_simulator`; processed transaction interpretation
  belongs in `tx_processor`.
- Keep important address lists under `src/common_addresses/`; entity modules
  should classify and aggregate, not maintain duplicate catalogs.
- If adding a new AMM route, put route/spender/calldata builders in
  `tx_simulator::tx_builders`, then call them from `tx_processor` or
  strategies.
