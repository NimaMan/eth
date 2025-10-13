#!/usr/bin/env python3
"""
Example: Inspect address_balance_changes for a transaction

Usage:
  python3 test_address_balance_changes.py 0x<tx_hash>
  # or set TX_HASH env var

This script runs process_transaction_from_hash_with_simulation() so balance
changes and internal traces are available.
"""

import os
import sys
from pprint import pprint

import pyreth


def main():
    if len(sys.argv) >= 2:
        tx_hash = sys.argv[1]
    else:
        tx_hash = os.environ.get("TX_HASH")
    if not tx_hash:
        print("Provide a tx hash as argv or set TX_HASH env var.")
        sys.exit(1)

    reth = pyreth.PyReth()
    processor = reth.tx_processor()

    print("=== Address Balance Changes Demo ===\n")
    print(f"Processing with simulation: {tx_hash}")
    tx = processor.process_transaction_from_hash_with_simulation(tx_hash)

    print(f"\nBlock: {tx.block_number}  Index: {tx.tx_index}")
    print(f"From: {tx.from_address}  To: {tx.to_address}")
    print(f"Status: {'Success' if tx.status == 'True' else 'Failed'}")

    changes = tx.address_balance_changes
    print(f"\nAddresses with changes: {len(changes)}")

    # Show up to 10 addresses and their currency/token nets
    shown = 0
    for addr, change in changes.items():
        print(f"\n- {addr}")
        currency = change.get("currency_net", {})
        token = change.get("token_net", {})
        if currency:
            print("  currency_net:")
            for sym, amt in list(currency.items())[:6]:
                print(f"    {sym}: {amt}")
        if token:
            print("  token_net:")
            for tok_addr, amt in list(token.items())[:6]:
                print(f"    {tok_addr}: {amt}")
        shown += 1
        if shown >= 10:
            break

    print("\n✅ Done.")


if __name__ == "__main__":
    main()

