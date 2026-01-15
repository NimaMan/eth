#!/usr/bin/env python3
"""
Print the most recent transactions recorded in the mempool arrival index.

We walk backwards from the latest block, gather transaction numbers, and show
the arrival timestamp (if any) along with basic metadata so we can confirm the
recorder is operating as expected.
"""

from __future__ import annotations

import argparse
from datetime import datetime, timezone

import pyreth


def _format_ms(value: int | None) -> str:
    if value is None:
        return "None"
    dt = datetime.fromtimestamp(value / 1000, tz=timezone.utc)
    return f"{value} ({dt.isoformat(timespec='milliseconds')})"


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Inspect the latest entries in the mempool arrival table."
    )
    parser.add_argument(
        "--count",
        type=int,
        default=10,
        help="How many transactions to display.",
    )
    parser.add_argument(
        "--max-blocks",
        type=int,
        default=20,
        help="How many recent blocks to scan (fails fast once enough transactions are found).",
    )
    args = parser.parse_args()

    reth = pyreth.PyReth()
    query = reth.chain_query()

    latest_block = query.get_latest_block()
    remaining = args.count
    block = latest_block
    visited_blocks = 0

    rows: list[tuple[int, str, int, int | None]] = []

    while remaining > 0 and block > 0 and visited_blocks < args.max_blocks:
        try:
            first_tx_num, tx_count = query.block_tx_indices(block)
        except Exception:
            block -= 1
            visited_blocks += 1
            continue

        for offset in range(int(tx_count) - 1, -1, -1):
            tx_number = first_tx_num + offset
            tx_data = query.transaction_by_number(tx_number)
            arrival = query.get_tx_arrival_ms(tx_data.hash)
            rows.append((tx_number, tx_data.hash, block, arrival))
            remaining -= 1
            if remaining == 0:
                break

        block -= 1
        visited_blocks += 1

    if not rows:
        print("No transactions collected (arrival index may be empty).")
        return

    print(f"Latest block scanned: {latest_block}")
    print(f"Rows collected: {len(rows)}")
    print("-" * 120)
    print(f"{'tx_number':>12}  {'block':>8}  {'arrival_ms (iso)':<40}  hash")
    print("-" * 120)
    for tx_number, tx_hash, block_number, arrival in rows:
        print(
            f"{tx_number:12d}  {block_number:8d}  {_format_ms(arrival):<40}  {tx_hash}"
        )


if __name__ == "__main__":
    main()
