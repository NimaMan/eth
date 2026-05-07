"""Benchmark historical address-to-transaction RethIndex writes.

The benchmark writes to a temporary index directory by default so it can be run
without modifying the production ``<reth_datadir>/reth_index`` database.
"""

from __future__ import annotations

import argparse
import os
import tempfile
import time
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Set, Tuple


AddressEntries = List[Tuple[int, List[str]]]


@dataclass
class BlockPayload:
    block_number: int
    tx_count: int
    entries: AddressEntries
    process_s: float
    extract_s: float


@dataclass
class ChunkStats:
    start_block: int
    end_block: int
    blocks: int
    tx_count: int
    participating_txs: int
    appended: int
    process_s: float
    extract_s: float
    write_s: float


def _get_tx_index(tx: object) -> Optional[int]:
    tx_index = getattr(tx, "tx_index", None)
    if tx_index is None and isinstance(tx, dict):
        tx_index = tx.get("tx_index")
    if tx_index is None:
        return None
    try:
        return int(tx_index)
    except (TypeError, ValueError):
        return None


def _entries_from_address_index(processed_block: object) -> Optional[AddressEntries]:
    address_index = getattr(processed_block, "address_index", None)
    if not address_index:
        return None

    tx_to_addresses: Dict[int, Set[str]] = defaultdict(set)
    for address, transactions in address_index.items():
        if not address or not transactions:
            continue
        for tx in transactions:
            tx_index = _get_tx_index(tx)
            if tx_index is not None:
                tx_to_addresses[tx_index].add(address)

    if not tx_to_addresses:
        return None

    return [
        (tx_index, sorted(addresses))
        for tx_index, addresses in sorted(tx_to_addresses.items())
        if addresses
    ]


def _entries_from_transactions(transactions: Sequence[object]) -> AddressEntries:
    entries: AddressEntries = []
    for tx in transactions:
        addresses = set(getattr(tx, "unique_addresses", []) or [])
        from_address = getattr(tx, "from_address", None)
        to_address = getattr(tx, "to_address", None)
        if not addresses and isinstance(tx, dict):
            raw = tx.get("unique_addresses")
            if raw:
                addresses.update(raw)
            from_address = from_address or tx.get("from_address")
            to_address = to_address or tx.get("to_address")
        if from_address:
            addresses.add(from_address)
        if to_address:
            addresses.add(to_address)

        tx_index = _get_tx_index(tx)
        if addresses and tx_index is not None:
            entries.append((tx_index, sorted(addresses)))
    return entries


def _entries_from_processed_block(processed_block: object) -> AddressEntries:
    entries = _entries_from_address_index(processed_block)
    if entries is not None:
        return entries
    return _entries_from_transactions(processed_block.transactions)


def _chunks(items: Sequence[int], size: int) -> Iterable[Sequence[int]]:
    for offset in range(0, len(items), size):
        yield items[offset : offset + size]


def _percentile(values: Sequence[float], pct: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    idx = int(round((len(ordered) - 1) * pct))
    return ordered[idx]


def _format_rate(entries: int, seconds: float) -> str:
    if seconds <= 0:
        return "inf"
    return f"{entries / seconds:.0f}"


def _resolve_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Benchmark historical address_to_blocks RethIndex writes.",
    )
    parser.add_argument(
        "--datadir",
        default=os.environ.get("PYRETH_DATADIR"),
        help="Reth datadir. Defaults to PYRETH_DATADIR or PyReth's configured default.",
    )
    parser.add_argument(
        "--index-dir",
        default=None,
        help="RethIndex output dir. Defaults to a temporary benchmark directory.",
    )
    parser.add_argument(
        "--sync-mode",
        choices=("durable", "no-metasync", "safe-no-sync", "utterly-no-sync"),
        default=os.environ.get("PYRETH_INDEX_DB_SYNC_MODE", "safe-no-sync"),
        help="MDBX sync mode for the benchmark index.",
    )
    parser.add_argument("--start-block", type=int, default=None)
    parser.add_argument("--end-block", type=int, default=None)
    parser.add_argument(
        "--blocks",
        type=int,
        default=20,
        help="Number of trailing blocks to process when --start-block is omitted.",
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=20,
        help="Number of blocks per AddressBlockParticipationIndexer.write_block_participation_batch call.",
    )
    parser.add_argument(
        "--per-block",
        action="store_true",
        help="Write each block independently instead of batching each chunk.",
    )
    return parser.parse_args()


def main() -> None:
    args = _resolve_args()

    if args.datadir:
        os.environ["PYRETH_DATADIR"] = args.datadir
    os.environ["PYRETH_INDEX_DB_SYNC_MODE"] = args.sync_mode

    index_dir = args.index_dir
    if index_dir is None:
        index_dir = tempfile.mkdtemp(prefix="reth_index_bench_")
    else:
        Path(index_dir).mkdir(parents=True, exist_ok=True)
    os.environ["PYRETH_ADDRESS_INDEX_DIR"] = index_dir

    import pyreth
    from pyreth import block_processor

    provider = block_processor()
    latest = provider.get_latest_block()
    end_block = args.end_block if args.end_block is not None else latest
    start_block = args.start_block
    if start_block is None:
        start_block = max(0, end_block - max(args.blocks, 1) + 1)
    if start_block > end_block:
        raise SystemExit("--start-block must be <= --end-block")

    block_numbers = list(range(start_block, end_block + 1))
    batch_size = max(1, args.batch_size)
    indexer = pyreth.AddressBlockParticipationIndexer()
    stats: List[ChunkStats] = []

    print(f"index_dir={index_dir}")
    print(f"sync_mode={args.sync_mode}")
    print(f"range={start_block}-{end_block} blocks={len(block_numbers)}")
    print(f"write_mode={'per_block' if args.per_block else 'batch'} batch_size={batch_size}")

    for block_chunk in _chunks(block_numbers, batch_size):
        payload: List[BlockPayload] = []
        for block_number in block_chunk:
            t0 = time.perf_counter()
            processed_block = provider.process_block(block_number)
            t1 = time.perf_counter()
            entries = _entries_from_processed_block(processed_block)
            t2 = time.perf_counter()
            payload.append(
                BlockPayload(
                    block_number=block_number,
                    tx_count=len(processed_block.transactions),
                    entries=entries,
                    process_s=t1 - t0,
                    extract_s=t2 - t1,
                )
            )

        t0 = time.perf_counter()
        if args.per_block:
            appended_counts = [
                int(indexer.write_block_participation(item.block_number, item.entries))
                for item in payload
            ]
        else:
            appended_counts = [
                int(value)
                for value in indexer.write_block_participation_batch(
                    [(item.block_number, item.entries) for item in payload]
                )
            ]
        t1 = time.perf_counter()

        chunk_stats = ChunkStats(
            start_block=payload[0].block_number,
            end_block=payload[-1].block_number,
            blocks=len(payload),
            tx_count=sum(item.tx_count for item in payload),
            participating_txs=sum(len(item.entries) for item in payload),
            appended=sum(appended_counts),
            process_s=sum(item.process_s for item in payload),
            extract_s=sum(item.extract_s for item in payload),
            write_s=t1 - t0,
        )
        stats.append(chunk_stats)

        print(
            "chunk "
            f"{chunk_stats.start_block}-{chunk_stats.end_block} "
            f"blocks={chunk_stats.blocks} "
            f"txs={chunk_stats.tx_count} "
            f"participating_txs={chunk_stats.participating_txs} "
            f"appended={chunk_stats.appended} "
            f"process_s={chunk_stats.process_s:.4f} "
            f"extract_s={chunk_stats.extract_s:.4f} "
            f"write_s={chunk_stats.write_s:.4f} "
            f"entries_per_write_s={_format_rate(chunk_stats.appended, chunk_stats.write_s)}"
        )

    total_blocks = sum(item.blocks for item in stats)
    total_txs = sum(item.tx_count for item in stats)
    total_participating = sum(item.participating_txs for item in stats)
    total_appended = sum(item.appended for item in stats)
    total_process_s = sum(item.process_s for item in stats)
    total_extract_s = sum(item.extract_s for item in stats)
    total_write_s = sum(item.write_s for item in stats)
    write_ms = [item.write_s * 1000.0 for item in stats]

    print(
        "summary "
        f"blocks={total_blocks} "
        f"txs={total_txs} "
        f"participating_txs={total_participating} "
        f"appended={total_appended} "
        f"process_s={total_process_s:.4f} "
        f"extract_s={total_extract_s:.4f} "
        f"write_s={total_write_s:.4f} "
        f"write_ms_p50={_percentile(write_ms, 0.50):.3f} "
        f"write_ms_p95={_percentile(write_ms, 0.95):.3f} "
        f"entries_per_write_s={_format_rate(total_appended, total_write_s)}"
    )


if __name__ == "__main__":
    main()
