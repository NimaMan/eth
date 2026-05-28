# Block RPC Fetching

This folder owns raw block reads from local Reth/MDBX or RPC.

## Live-Tail RPC Data Flow

```text
LiveBlockProcessor receives execution head B
  -> RpcBlockDataFetcher::fetch_block(block_hash)
  -> RpcBlockDataFetcher::fetch_receipts(block_hash)
  -> RpcBlockDataFetcher::trace_block_by_hash(block_hash)
  -> RpcBlockDataFetcher::trace_block_state_diffs_by_hash(block_hash)
  -> parser builds RawBlockData and prestate diff frames for the same block hash
  -> BlockProcessor converts RawBlockData into ProcessedBlock B
```

Live callers must prefer hash-based reads for block-coupled data. A block number
is not enough around reorgs: `eth_getBlockByHash`, `eth_getBlockReceipts`,
`debug_traceBlockByHash` with `callTracer`, and `debug_traceBlockByHash` with
`prestateTracer` must all target the same hash.

Number-based helpers remain for historical batch/range paths, where canonical
history is already stable enough for the caller's purpose. They should not be
used for fresh live-tail state that will feed `LiveTxSimulator`.
