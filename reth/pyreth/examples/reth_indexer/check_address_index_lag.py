"""Check how far Reth transaction lookup indices lag behind the node head.

This script compares the latest block reported by the RPC endpoint against the
latest block whose canonical Reth transaction indices are available. This is a
diagnostic for direct tx-number lookups. The current ``address_to_blocks`` index
does not need tx lookup indices to write block participation.
"""
import os
from typing import Optional
from web3 import Web3
import pyreth

RPC_URL = os.environ.get("PYRETH_RPC_URL", "http://127.0.0.1:8545")
MAX_LOOKBACK = int(os.environ.get("PYRETH_INDEX_LAG_LOOKBACK", "256"))


def find_latest_indexed_block(indexer: "pyreth.AddressBlockIndexer", latest_block: int) -> Optional[int]:
    for block in range(latest_block, max(latest_block - MAX_LOOKBACK, -1), -1):
        if indexer.block_has_indices(block):
            return block
    return None


def main() -> None:
    w3 = Web3(Web3.HTTPProvider(RPC_URL))
    if not w3.is_connected():
        raise RuntimeError(f"Failed to connect to RPC at {RPC_URL}")

    latest_rpc_block = w3.eth.get_block_number()
    indexer = pyreth.AddressBlockIndexer()
    latest_indexed_block = find_latest_indexed_block(indexer, latest_rpc_block)

    print(f"RPC latest block:             {latest_rpc_block}")
    if latest_indexed_block is None:
        print(
            "No Reth tx lookup indices found in the last "
            f"{MAX_LOOKBACK} blocks. The Reth database may be significantly behind."
        )
        return

    lag = latest_rpc_block - latest_indexed_block
    print(f"Latest block with tx lookup: {latest_indexed_block}")
    print(f"Reth tx lookup lag:          {lag} block(s)")

    if lag > 0:
        print(
            "\nWarning: Reth tx lookup indices are lagging behind the node head. "
            "Ensure the Reth database is syncing its indexing stages."
        )
    else:
        print("\nReth tx lookup indices are caught up with the node head.")


if __name__ == "__main__":
    main()
