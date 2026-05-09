# AddressBlockParticipationWriter Notes

This folder hosts the Rust writers for RethIndex. The address participation
writer is `AddressBlockParticipationWriter` in `address_block_participation_writer.rs`.

## Current Address Index

The address index stores candidate processed blocks, not transaction numbers:

- Table: `address_to_blocks`
- Key: 20-byte Ethereum address
- Duplicate value: 8-byte `block_number`, encoded big-endian
- MDBX flags: `DUP_SORT | DUP_FIXED`
- Write semantics: one value per `(address, block_number)`

`AddressBlockParticipationWriter` receives tx-level address participations for a block, unions
all addresses across that block, and writes each address once for the block. MDBX
`NO_DUP_DATA` makes the write idempotent if a block is replayed.

In the live system this writer is hosted by
`tx_processor::live::LiveAddressBlockParticipationIndexWorker`. The live block
processor enqueues processed blocks after Redis publication succeeds; the worker
does extraction and MDBX writes on a background task so index writes cannot
delay live block publication.

For historical ranges, `tx_processor`'s
`refresh_processed_block_disk_cache` example writes this index while it fills or
reads the processed-block disk cache. That keeps the replay cache and
`address_to_blocks` synchronized from the same block data.

## Why Blocks, Not Tx Numbers

For this project, the useful query is usually "which blocks should I replay for
this token/address/pool?" Once we have candidate blocks, we load the processed
block cache or replay the block and filter the actual `ProcessedTransaction`s in
block order.

Storing block numbers has three practical benefits:

- It removes the dependency on Reth `BlockBodyIndices` from the writer path.
- It avoids writing multiple duplicate tx-number entries when the same address
  appears many times in one block.
- It matches the downstream reconstruction path, which needs the full block
  anyway for same-block ordering and context.

## Query Contract

Readers should treat `address_to_blocks[address]` as a sorted candidate block
set. The table does not prove that every transaction in those blocks involved
the address; callers must load/replay the blocks and filter.

The writer is safe to backfill or replay: duplicate `(address, block_number)`
values are skipped.

## Migration

This is a schema rename and semantic change from the old `address_to_txs` table.
MDBX cannot convert the old table in place. If an existing database still has the
old layout, rebuild the RethIndex directory so the new `address_to_blocks`
dupsort table is created cleanly.
