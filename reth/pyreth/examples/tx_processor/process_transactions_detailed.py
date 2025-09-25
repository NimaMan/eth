#!/usr/bin/env python3
"""
Example: Process multiple transactions with detailed results

Demonstrates TxProcessor.process_transactions_detailed(...) which returns a
dict with 'success' and 'failed' lists for robust batch processing.
"""

import pyreth

def main():
    reth = pyreth.PyReth()
    processor = reth.tx_processor()

    # Example list (include an invalid hash to show error handling)
    tx_hashes = [
        "0x6a85c721e474ad9774c5c30c6d6f4169e152005cc5d2b1e5503446d65d5f9bb8",
        "0xINVALIDBADHASH",
    ]

    print("=== TxProcessor: process_transactions_detailed ===\n")
    result = processor.process_transactions_detailed(tx_hashes, parallel=True)

    print(f"Total: {result['total']}")
    print(f"Successful: {result['successful']}")
    print(f"Failed: {result['failed_count']}\n")

    print("Success entries:")
    for entry in result["success"]:
        tx = entry["transaction"]
        print(f"  idx={entry['index']} hash={entry['hash'][:10]}... block={tx.block_number} type={tx.txn_type}")

    print("\nFailed entries:")
    for entry in result["failed"]:
        print(f"  idx={entry['index']} hash={entry['hash']} error={entry['error']}")

    print("\n✅ Detailed batch example complete!")

if __name__ == "__main__":
    main()

