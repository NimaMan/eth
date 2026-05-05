"""Process recent blocks, persist address→tx links, and verify via address index."""

import argparse
import asyncio
from datetime import timedelta
from typing import Optional, Set, Tuple

from web3 import Web3
from pyreth import AddressTxIndexer, block_processor

from eth_data.database.writers.transaction_writer import TransactionAddresstoTxIndexer

SECONDS_PER_DAY = 24 * 60 * 60


def _find_first_block_after_timestamp(
    w3: Web3, cutoff_ts: int, latest_block: int
) -> int:
    """Binary search to find the earliest block whose timestamp >= cutoff."""
    low = 0
    high = latest_block
    candidate = latest_block

    while low <= high:
        mid = (low + high) // 2
        block = w3.eth.get_block(mid, full_transactions=False)
        block_ts = block["timestamp"]
        if block_ts >= cutoff_ts:
            candidate = mid
            high = mid - 1
        else:
            low = mid + 1

    return candidate


def _verify_index_write(
    processed_block, target_block: int
) -> Tuple[bool, Optional[str]]:
    """Return (verified, message) for the first tx with address participation."""
    sample_tx = next((tx for tx in processed_block if tx.unique_addresses), None)
    if not sample_tx:
        return False, "No transactions with address participation to verify."

    sample_address = next(iter(sample_tx.unique_addresses))
    indexer = AddressTxIndexer()
    tx_refs = indexer.address_transactions(sample_address)
    matching = [
        ref
        for ref in tx_refs
        if ref.block_number == target_block and ref.tx_hash.lower() == sample_tx.hash.lower()
    ]

    if matching:
        msg = (
            "Verified address index entry: "
            f"address={sample_address} block={matching[0].block_number} "
            f"tx_hash={matching[0].tx_hash}"
        )
        return True, msg

    return False, (
        "Warning: no address index entry found for "
        f"{sample_address} tx {sample_tx.hash}"
    )


async def _process_recent_blocks(days: int, target_address: Optional[str]) -> None:
    processor = block_processor()
    transaction_writer = TransactionAddresstoTxIndexer()
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))

    latest_block = w3.eth.get_block_number()
    latest_block_data = w3.eth.get_block(latest_block, full_transactions=False)
    latest_timestamp = latest_block_data["timestamp"]
    lookback_seconds = days * SECONDS_PER_DAY
    cutoff_timestamp = max(0, latest_timestamp - lookback_seconds)

    start_block = _find_first_block_after_timestamp(w3, cutoff_timestamp, latest_block)
    total_blocks = latest_block - start_block + 1
    print(
        f"Processing {total_blocks} block(s) "
        f"from {start_block} to {latest_block} "
        f"(lookback={timedelta(days=days)})"
    )

    target_checksum: Optional[str] = None
    if target_address:
        try:
            target_checksum = Web3.to_checksum_address(target_address)
        except ValueError as err:
            raise SystemExit(f"Invalid target address {target_address}: {err}") from err

    total_transactions = 0
    total_appended = 0
    verified = False
    verification_message: Optional[str] = None
    target_seen_hashes: Set[Tuple[int, str]] = set()

    for block_number in range(start_block, latest_block + 1):
        processed_block = await asyncio.to_thread(processor.process_block, block_number)
        processed = list(processed_block.transactions)
        total_transactions += len(processed)
        appended = transaction_writer.write_transactions_address_tx(processed)
        total_appended += appended or 0

        print(
            f"Block {block_number}: processed {len(processed)} txs, "
            f"appended {appended or 0} address->tx entries"
        )

        if target_checksum:
            for tx in processed:
                if target_checksum in getattr(tx, "unique_addresses", set()):
                    target_seen_hashes.add((block_number, tx.hash.lower()))

        if not verified and appended and not target_checksum:
            verified, verification_message = _verify_index_write(processed, block_number)

    print(
        f"Completed processing of {total_blocks} block(s); "
        f"total transactions={total_transactions}, "
        f"total address->tx entries appended={total_appended}"
    )
    if target_checksum:
        indexer = AddressTxIndexer()
        tx_refs = indexer.address_transactions(target_checksum)
        indexed_hashes: Set[Tuple[int, str]] = {
            (ref.block_number, ref.tx_hash.lower())
            for ref in tx_refs
            if start_block <= ref.block_number <= latest_block
        }

        missing = target_seen_hashes - indexed_hashes
        extra = indexed_hashes - target_seen_hashes

        print(
            f"Target address {target_checksum}: "
            f"pre-write unique tx count={len(target_seen_hashes)}, "
            f"indexed tx count={len(indexed_hashes)}"
        )
        if not missing and not extra:
            print("Target address verification passed: all transactions accounted for.")
        else:
            if missing:
                sample = sorted(missing)[:10]
                print(
                    f"Missing {len(missing)} transactions in index. Sample: {sample}"
                )
            if extra:
                sample = sorted(extra)[:10]
                print(
                    f"Found {len(extra)} extra transactions in index not seen during processing. Sample: {sample}"
                )
        # Ensure general verification message does not override target-specific check
        verified = True

    if verification_message:
        print(verification_message)
    elif not verified:
        print("No verification performed (no address participation found).")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Process recent blocks and persist address participation."
    )
    parser.add_argument(
        "--days",
        type=int,
        default=7,
        help="Number of days to look back from the latest block (default: 7).",
    )
    parser.add_argument(
        "--address",
        type=str,
        default=None,
        help="Target address to verify against the address→tx index.",
    )
    args = parser.parse_args()
    asyncio.run(_process_recent_blocks(days=args.days, target_address=args.address))


if __name__ == "__main__":
    main()
