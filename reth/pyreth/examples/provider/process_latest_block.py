"""
Process the latest canonical block via the processed transaction provider and
print a compact summary.

Usage (from repo root after building pyreth):
  PYTHONPATH=$(pwd) python rust/pyreth/examples/provider/process_latest_block.py

Optionally export PYRETH_DATADIR if your Reth database lives elsewhere.
"""

from __future__ import annotations

from pyreth import processed_tx_provider as pyreth_processed_tx_provider


def main() -> None:
    provider = pyreth_processed_tx_provider()

    latest_block_number = provider.get_latest_block()
    processed_block = provider.process_block(latest_block_number)

    base_fee = processed_block.base_fee_per_gas or "N/A"
    print(
        f"Processed block {processed_block.number} (hash={processed_block.hash})\n"
        f"  parent={processed_block.parent_hash}\n"
        f"  tx_count={len(processed_block.transactions)}\n"
        f"  gas_used={processed_block.gas_used} / {processed_block.gas_limit}\n"
        f"  base_fee={base_fee}"
    )

    print("First few transactions:")
    for tx in processed_block.transactions[:3]:
        actions_preview = ", ".join(tx.actions[:2]) if tx.actions else "-"
        print(
            f"  {tx.hash} status={tx.status} from={tx.from_address}"
            f" to={tx.to_address or '-'} actions={actions_preview}"
        )


if __name__ == "__main__":
    main()
