"""
Example: load processed transactions for a specific ERC20 token using the
PyReth processed transaction provider.

Token: 0xc8Ab73b7EaeE2FD3C858dcDB22fB03433A7aeB9F

Usage (from repo root after building pyreth):
  PYTHONPATH=$(pwd) python rust/pyreth/examples/provider/token_processed_transactions_example.py

This script scans a configurable window of most recent blocks (default 2,000)
for events touching the token and prints a short summary for the first
transactions found.

Override the window via the `TOKEN_WINDOW` environment variable if you want to
widen or narrow the search range. Export `PYRETH_DATADIR` to point at a custom
Reth database if needed.
"""

from __future__ import annotations

import os

import pyreth

TOKEN_ADDRESS = "0xc8Ab73b7EaeE2FD3C858dcDB22fB03433A7aeB9F"
DEFAULT_WINDOW = 2_000
MAX_PRINT = 15


def main() -> None:
    window = int(os.environ.get("TOKEN_WINDOW", DEFAULT_WINDOW))

    reth = pyreth.PyReth()
    core_provider = reth.processed_tx_provider()
    token_provider = core_provider.token_provider()

    latest = core_provider.get_latest_block()
    start = max(latest + 1 - window, 0)

    print(
        f"Scanning token {TOKEN_ADDRESS} across blocks [{start}, {latest}] using window={window}"
    )

    token_provider.load_blocks_for_token(TOKEN_ADDRESS, start, latest)
    transactions = token_provider.transactions_for(TOKEN_ADDRESS)

    print(f"Discovered {len(transactions)} processed transactions in the cached range")

    for tx in transactions[:MAX_PRINT]:
        actions = ", ".join(tx.actions[:3]) if tx.actions else "-"
        print(
            f"  block={tx.block_number} hash={tx.hash} status={tx.status}"
            f" from={tx.from_address} to={tx.to_address or '-'} actions={actions}"
        )

    if len(transactions) > MAX_PRINT:
        print(f"  ... {len(transactions) - MAX_PRINT} additional transactions omitted")


if __name__ == "__main__":
    main()
