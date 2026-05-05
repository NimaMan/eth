import asyncio
import csv
import statistics
import time
from pathlib import Path

from pyreth import block_processor
from web3 import Web3


async def collect_metrics(
    http_url: str = "http://127.0.0.1:8545",
    block_count: int = 1000,
    output_path: Path = Path("block_processing_metrics.csv"),
) -> None:
    w3 = Web3(Web3.HTTPProvider(http_url))
    latest = w3.eth.block_number
    start = max(0, latest - block_count + 1)

    processor = block_processor()

    rows = []
    print(f"Processing blocks {start} → {latest} …")
    for block_number in range(start, latest + 1):
        wall_start = time.perf_counter()
        result = await asyncio.to_thread(processor.process_block, block_number)
        wall_end = time.perf_counter()
        metrics = getattr(result, "metrics", {}) or {}
        rows.append(
            {
                "block_number": block_number,
                "fetch_duration": metrics.get("fetch_duration", 0.0),
                "tx_processing_duration": metrics.get("tx_processing_duration", 0.0),
                "total_duration": metrics.get("total_duration", wall_end - wall_start),
                "wall_clock": wall_end - wall_start,
                "tx_count": metrics.get("tx_count", 0),
                "failed_tx_count": metrics.get("failed_tx_count", 0),
            }
        )

    with output_path.open("w", newline="") as f:
        writer = csv.DictWriter(
            f,
            [
                "block_number",
                "fetch_duration",
                "tx_processing_duration",
                "total_duration",
                "wall_clock",
                "tx_count",
                "failed_tx_count",
            ],
        )
        writer.writeheader()
        writer.writerows(rows)

    totals = [row["total_duration"] for row in rows]
    fetches = [row["fetch_duration"] for row in rows]
    txs = [row["tx_processing_duration"] for row in rows]
    print(f"Wrote {len(rows)} rows to {output_path}")
    print(
        "Totals avg={:.2f}s p95={:.2f}s max={:.2f}s".format(
            statistics.mean(totals),
            statistics.quantiles(totals, n=20)[-1],
            max(totals),
        )
    )
    print(
        "Fetch avg={:.2f}s, Tx avg={:.2f}s".format(
            statistics.mean(fetches), statistics.mean(txs)
        )
    )


if __name__ == "__main__":
    asyncio.run(collect_metrics())
