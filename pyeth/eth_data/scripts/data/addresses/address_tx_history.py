"""Simple CLI to dump Reth-backed address transaction history."""

from __future__ import annotations

import argparse
import json
from typing import Any, Dict, List

from eth_data.reth_chain_query.reth_index.address_tx_history import (
    RethAddressTxHistory,
)


def _record_to_dict(record) -> Dict[str, Any]:
    return {
        "block_number": record.block_number,
        "tx_index": record.tx_index,
        "tx_number": record.tx_number,
        "tx_hash": record.tx_hash,
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Dump transactions for an address using the Reth address index",
    )
    parser.add_argument("address", help="Address to inspect (with or without 0x prefix)")
    parser.add_argument(
        "--limit",
        type=int,
        default=None,
        help="Maximum number of transactions to return (default: all)",
    )
    parser.add_argument(
        "--newest-first",
        action="store_true",
        help="Return results newest-first instead of oldest-first",
    )
    args = parser.parse_args()

    history = RethAddressTxHistory()
    records = history.get_transactions(
        args.address,
        limit=args.limit,
        reverse=args.newest_first,
    )

    print(json.dumps([_record_to_dict(r) for r in records], indent=2))


if __name__ == "__main__":
    main()
