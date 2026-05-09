"""
Fetch a transaction by its global txumber using PyReth's ChainQuery.

Usage:
    PYTHONPATH=$(pwd) python pyreth/examples/python/fetch_transaction_by_number.py \
        --tx-number 3033187389

Environment:
    PYRETH_DATADIR            Override the Reth datadir (default ~/.local/share/reth/mainnet).
"""

import argparse
from typing import Any

from pyreth import chain_query


def format_address(address: Any) -> str:
    if address is None:
        return "ContractCreation"
    return str(address)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--tx-number",
        type=int,
        required=True,
        help="Global sequential transaction number (txumber).",
    )
    args = parser.parse_args()

    chain = chain_query()
    tx_number = args.tx_number
    print(f"Looking up transaction number {tx_number}...")

    tx = chain.transaction_by_number(tx_number)
    if tx is None:
        print("Transaction not found.")
        return

    print("Transaction loaded:")
    print(f"  hash: {tx.hash}")
    print(f"  block: #{tx.block_number} (tx_index {tx.tx_index})")
    print(f"  from: {format_address(tx.from_address)}")
    print(f"  to:   {format_address(tx.to_address)}")
    print(f"  value: {tx.value}")
    print(f"  gas_limit: {tx.gas_limit}")
    print(f"  gas_price: {tx.gas_price}")
    print(f"  nonce: {tx.nonce}")
    print(f"  type: {tx.transaction_type}")


if __name__ == "__main__":
    main()
