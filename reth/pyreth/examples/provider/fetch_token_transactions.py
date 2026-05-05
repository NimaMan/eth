"""
Load processed transactions for an ERC20 token contract using the token-level
provider. Useful for feeding token tracking or analytics pipelines.

Usage (from repo root after building pyreth):
  PYTHONPATH=$(pwd) python rust/pyreth/examples/provider/fetch_token_transactions.py \
      --token 0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48

Optionally export PYRETH_DATADIR if your Reth database lives elsewhere.
"""

from __future__ import annotations
import argparse

from pyreth import processed_tx_provider as pyreth_processed_tx_provider


DEFAULT_WINDOW = 500


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--token", required=True, help="Checksum ERC20 token contract")
    parser.add_argument(
        "--window",
        type=int,
        default=DEFAULT_WINDOW,
        help=f"Number of most recent blocks to scan (default: {DEFAULT_WINDOW})",
    )
    args = parser.parse_args()

    core_provider = pyreth_processed_tx_provider()
    token_provider = core_provider.token_provider()

    latest = core_provider.get_latest_block()
    start = max(latest + 1 - args.window, 0)

    token_provider.load_blocks_for_token(args.token, start, latest)
    transactions = token_provider.transactions_for(args.token)

    print(
        f"Loaded {len(transactions)} processed txs for token {args.token} in blocks"
        f" [{start}, {latest}]"
    )

    for tx in transactions[:10]:
        print(
            f"  block={tx.block_number} hash={tx.hash} status={tx.status}"
            f" from={tx.from_address} to={tx.to_address or '-'}"
        )

    if len(transactions) > 10:
        print(f"  ... {len(transactions) - 10} additional transactions omitted")


if __name__ == "__main__":
    main()
