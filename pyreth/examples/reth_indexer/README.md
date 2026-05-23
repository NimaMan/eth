# RethIndex Examples

These examples cover the Python-facing address-to-block index. They are
separate from the Rust mempool arrival examples in
`reth_chain_query/examples/reth_index`.

## What To Use

- `benchmark_address_index_history.py` measures historical `address_to_blocks`
  backfill performance. It writes to a temporary index directory by default and
  reports block processing, address extraction, and MDBX write time separately.
- `write_latest_block_index.py` is a small smoke test that writes one latest
  block into the configured index.
- `check_address_index_lag.py` checks whether Reth's canonical transaction
  lookup indices are available near head. It does not prove that our
  `address_to_blocks` table has been backfilled through that block.

## Recommended Benchmark Flow

Run against a temporary index first:

```bash
env \
  PYRETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth \
  /home/nima/miniconda3/bin/python \
  /home/nima/code/crypto/blockchains/eth/pyreth/examples/reth_indexer/benchmark_address_index_history.py \
  --blocks 100 \
  --batch-size 20
```

Only write to the production sidecar DB when the timing looks healthy:

```bash
env \
  PYRETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth \
  /home/nima/miniconda3/bin/python \
  /home/nima/code/crypto/blockchains/eth/pyreth/examples/reth_indexer/benchmark_address_index_history.py \
  --start-block 25000000 \
  --end-block 25001000 \
  --batch-size 20 \
  --index-dir /home/nima/storage/samsung8tb/ethereum/reth/reth_index
```

The metric to watch for bottlenecks is `write_s`, not `process_s`. For live
processing, block processing and tracing can dominate while MDBX appends remain
small.

The live block processor now hosts
`LiveAddressBlockParticipationIndexWorker`, which writes this same
`address_to_blocks` table through the processed-block replay-store path. Use the
benchmark here for temporary-index measurements and historical backfill timing
before writing large ranges into the production sidecar DB.
