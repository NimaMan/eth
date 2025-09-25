#!/usr/bin/env python3
"""
Example: Simulate an unsigned transaction

Demonstrates TxProcessor.simulate_unsigned_transaction(...), which is the
preferred way to create synthetic transactions for simulation.
"""

import pyreth

def main():
    reth = pyreth.PyReth()
    processor = reth.tx_processor()

    print("=== TxProcessor: simulate_unsigned_transaction ===\n")

    # Example params (replace with real ones for a working simulation)
    from_address = "0x0000000000000000000000000000000000000001"
    to_address = None
    value = None  # e.g., '0x0' or '0xde0b6b3a7640000'
    data = None   # e.g., ERC20 transfer calldata
    gas_limit = 3_000_000
    gas_price = None  # legacy; prefer EIP-1559 fields if available
    max_fee_per_gas = None
    max_priority_fee = None
    nonce = None
    block_number = None

    try:
        tx = processor.simulate_unsigned_transaction(
            from_address,
            to_address,
            value,
            data,
            gas_limit,
            gas_price,
            max_fee_per_gas,
            max_priority_fee,
            nonce,
            block_number,
        )

        print("Simulation done.")
        print(f"Block: {tx.block_number}")
        print(f"Status: {'Success' if tx.status == 'True' else 'Failed'}")
        print(f"ERC20 transfers: {len(tx.erc20_transfers)}")

        # Use convenience conversions
        d = tx.to_dict()
        print(f"Keys: {list(d.keys())[:10]} ...")
        print(f"JSON prefix: {tx.to_json()[:120]}...")

    except Exception as e:
        print(f"Error: {e}")

    print("\n✅ Unsigned transaction simulation example complete!")

if __name__ == "__main__":
    main()

