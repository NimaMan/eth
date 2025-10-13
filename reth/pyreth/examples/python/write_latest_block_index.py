"""
Index the latest block's processed transactions into the address_to_txs MDBX table.

Usage:
    PYTHONPATH=$(pwd) python rust/pyreth/examples/python/write_latest_block_index.py

Environment variables:
    PYRETH_DATADIR            Override the Reth datadir (default ~/.local/share/reth/mainnet).
    PYRETH_ADDRESS_INDEX_DIR  Override the address index directory (default <datadir>/reth_index).
"""

from __future__ import annotations

import pyreth


def main() -> None:
    provider = pyreth.ProcessedTxProvider()
    latest_block = provider.get_latest_block()
    processed_block = provider.process_block(latest_block)

    entries = []
    for tx in processed_block.transactions:
        addresses = set(tx.unique_addresses)
        if tx.from_address:
            addresses.add(tx.from_address)
        if tx.to_address:
            addresses.add(tx.to_address)
        if not addresses:
            continue
        entries.append((tx.tx_index, sorted(addresses)))

    indexer = pyreth.AddressTxIndexer()
    inserted = indexer.write_transactions(latest_block, entries)
    print(
        f"Indexed block {latest_block} with {len(entries)} participating txs; "
        f"appended {inserted} address→tx entries",
    )


if __name__ == "__main__":
    main()
