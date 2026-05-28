# Live Block Processor

This folder owns the live execution-head ingestion boundary used by
chain-server.

## Live-Tail Data Flow

```text
chain-server receives execution head B from newHeads
  -> LiveBlockProcessor parses block hash, number, and timestamp
  -> BlockProcessor::process_block_via_rpc(block_hash, block_number, include_traces)
  -> RpcBlockDataFetcher fetches block B and receipts by block hash
  -> RpcBlockDataFetcher fetches call traces for B by block hash
  -> BlockProcessor produces ProcessedBlock B
  -> LiveBlockProcessor fetches prestate diff frames for B by the same block hash
  -> LiveProcessedBlock { ProcessedBlock B, state_diffs } is returned
  -> LiveChainRuntime hands the block to LiveTokenRuntime
```

The important invariant is hash coupling: the processed block, call traces, and
prestate diff frames must all be fetched for the same block hash. Fetching any
of those by number can mix data across branches during a short reorg.

If prestate diffs are missing or the diff count does not match the transaction
count, the live processed block is still emitted without direct live state
diffs. The downstream runtime must treat that as missing exact simulation state,
not as a trade failure.
