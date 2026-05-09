#!/usr/bin/env python3
"""
Check whether a transaction was captured by the mempool arrival recorder.

This helper uses PyReth's ChainQuery bindings to look up the recorded
first-seen timestamp (in milliseconds since epoch) for a mined transaction.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone

import pyreth


def _format_ts(ms: int) -> str:
    """Return ISO timestamp string from milliseconds since epoch."""
    dt = datetime.fromtimestamp(ms / 1000, tz=timezone.utc)
    return dt.isoformat(timespec="microseconds")


def main(tx_hash) -> None:
    
    reth = pyreth.PyReth()

    chain_query = reth.chain_query()
    arrival_ms = chain_query.get_tx_arrival_ms(tx_hash)

    print(f"Transaction: {tx_hash}")
    if arrival_ms is None:
        print("Arrival timestamp: not recorded (tx may not have been observed in mempool recorder)")
    else:
        print(f"Arrival timestamp: {arrival_ms} ms ({_format_ts(arrival_ms)})")

    tx_processor = reth.tx_processor()
    processed = tx_processor.process_transaction_from_hash_with_simulation(tx_hash)
    print(f"Block number: {processed.block_number}")
    print(f"Block timestamp: {processed.block_timestamp}")
    print(f"Creator / from: {processed.from_address}")


if __name__ == "__main__":
    main(tx_hash= "0x3ce7ec43f6526fac03e7f303785ab132c07e34f61e2c53a0714e11abcb5c2314")
