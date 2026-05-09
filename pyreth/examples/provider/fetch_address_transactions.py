"""
Load processed transactions touching a specific address over a recent block
range using the address-level provider.

Usage (from repo root after building pyreth):
  PYTHONPATH=$(pwd) python pyreth/examples/provider/fetch_address_transactions.py \
      --address 0xdAC17F958D2ee523a2206206994597C13D831ec7

Optionally export PYRETH_DATADIR if your Reth database lives elsewhere.
"""

from __future__ import annotations

import argparse
from typing import Sequence

from pyreth import processed_tx_provider as pyreth_processed_tx_provider


DEFAULT_WINDOW = 500  # how many recent blocks to inspect by default


def format_actions(actions: Sequence[str]) -> str:
    if not actions:
        return "-"
    if len(actions) == 1:
        return actions[0]
    return f"{actions[0]} (+{len(actions) - 1} more)"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--address", required=True, help="Checksum target address")
    parser.add_argument(
        "--window",
        type=int,
        default=DEFAULT_WINDOW,
        help=f"Number of most recent blocks to scan (default: {DEFAULT_WINDOW})",
    )
    args = parser.parse_args()

    core_provider = pyreth_processed_tx_provider()
    addr_provider = core_provider.address_provider()

    latest = core_provider.get_latest_block()
    start = max(latest + 1 - args.window, 0)

    addr_provider.load_blocks_for_address(args.address, start, latest)
    transactions = addr_provider.transactions_for(args.address)

    print(
        f"Loaded {len(transactions)} processed txs for {args.address} in blocks"
        f" [{start}, {latest}]"
    )

    for tx in transactions[:10]:
        print(
            f"  block={tx.block_number} hash={tx.hash} status={tx.status}"
            f" value={tx.value} actions={format_actions(tx.actions)}"
        )

    if len(transactions) > 10:
        print(f"  ... {len(transactions) - 10} additional transactions omitted")


if __name__ == "__main__":
    main()
