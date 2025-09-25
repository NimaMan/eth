#!/usr/bin/env python3
"""
Verify address_balance_changes with a simple unsigned ETH transfer.

Usage:
  python3 verify_balance_changes_unsigned_tx.py \
      --from 0x... --to 0x... --value-wei 10000000000000000 [--block N]

Notes:
- Sender must have at least `value` ETH at the chosen block.
- We compare the expected +/- ETH amounts (in ETH units) with a tolerance,
  since the binding converts ETH to float for convenience.
"""

import argparse
import math
import pyreth


def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--from", dest="from_address", required=True)
    p.add_argument("--to", dest="to_address", required=True)
    p.add_argument("--value-wei", dest="value_wei", required=True,
                   help="Amount in wei (decimal or 0x-hex)")
    p.add_argument("--block", dest="block_number", type=int, default=None)
    return p.parse_args()


def to_wei_int(s: str) -> int:
    s = s.strip().lower()
    if s.startswith("0x"):
        return int(s, 16)
    return int(s)


def main():
    args = parse_args()
    value_wei = to_wei_int(args.value_wei)
    value_eth = value_wei / 1e18

    reth = pyreth.PyReth()
    processor = reth.tx_processor()

    print("=== Verify address_balance_changes (unsigned ETH transfer) ===\n")
    print(f"From: {args.from_address}")
    print(f"To:   {args.to_address}")
    print(f"Value: {value_wei} wei ({value_eth} ETH)")
    if args.block_number is not None:
        print(f"Block: {args.block_number}")

    # Simulate unsigned transaction
    tx = processor.simulate_unsigned_transaction(
        args.from_address,
        args.to_address,
        hex(value_wei),  # value in 0x-hex string
        None,            # data
        21000,           # gas_limit
        None,            # gas_price (let simulator compute)
        None,            # max_fee_per_gas
        None,            # max_priority_fee
        None,            # nonce
        args.block_number,
    )

    # Extract balance changes (ETH shown in floats by the binding)
    changes = tx.address_balance_changes
    from_change = None
    to_change = None
    if args.from_address in changes:
        from_change = changes[args.from_address]["currency_net"].get("ETH")
    if args.to_address in changes:
        to_change = changes[args.to_address]["currency_net"].get("ETH")

    print("\nObserved currency_net[ETH]:")
    print(f"  Sender:   {from_change}")
    print(f"  Receiver: {to_change}")

    # Validate with tolerance (1e-12 ETH)
    ok_from = (from_change is not None) and math.isclose(from_change, -value_eth, rel_tol=0, abs_tol=1e-12)
    ok_to = (to_change is not None) and math.isclose(to_change, value_eth, rel_tol=0, abs_tol=1e-12)

    if ok_from and ok_to:
        print("\n✅ address_balance_changes match expected +/- ETH amounts.")
    else:
        print("\n❌ address_balance_changes do not match expected values.")
        print("   Check that the sender has sufficient balance at the chosen block and that thresholds aren't filtering small values.")

    print("\nDone.")


if __name__ == "__main__":
    main()

