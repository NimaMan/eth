import os
import time
from collections import defaultdict
from dataclasses import dataclass
from typing import Dict, Iterable, List, Optional, Tuple

from eth_data.pyreth_client import PyrethClient


def _env_int(name: str, default: int) -> int:
    value = os.getenv(name)
    if value is None:
        return default
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


ADDRESS_TX_WRITE_LAG_SECONDS = max(0, _env_int("PYRETH_ADDRESS_TX_WRITE_LAG_SECONDS", 120))
ADDRESS_TX_MAX_FLUSH_BLOCKS = max(1, _env_int("PYRETH_ADDRESS_TX_MAX_FLUSH_BLOCKS", 20))


@dataclass
class _PendingBlock:
    timestamp: Optional[int]
    batches: List[Tuple[int, List[str]]]


class TransactionAddresstoTxIndexer:
    def __init__(self) -> None:
        self._address_indexer = PyrethClient.instance().address_indexer()
        self.last_appended = 0
        self._write_lag_seconds = ADDRESS_TX_WRITE_LAG_SECONDS
        self._pending_blocks: Dict[int, _PendingBlock] = {}

    def write_transactions_address_tx(self, processed_txs: Iterable[object]) -> int:
        self.last_appended = 0
        transactions: List[object]
        if not processed_txs:
            return 0
        if isinstance(processed_txs, Iterable) and not isinstance(processed_txs, (str, bytes)):
            transactions = list(processed_txs)
        else:
            transactions = [processed_txs]  # type: ignore[list-item]

        if not transactions:
            return 0

        indexer = self._address_indexer
        if indexer is None:
            return 0

        grouped: Dict[int, List[Tuple[int, List[str]]]] = defaultdict(list)
        timestamps: Dict[int, Optional[int]] = {}
        for tx in transactions:
            block_key = self._extract_int_field(tx, "block_number")
            tx_index = self._extract_int_field(tx, "tx_index")
            if block_key is None or tx_index is None:
                continue
            block_timestamp = self._extract_block_timestamp(tx)

            addresses = self._extract_addresses(tx)
            if not addresses:
                continue

            grouped[block_key].append((tx_index, sorted(addresses)))
            timestamps.setdefault(block_key, block_timestamp)

        if not grouped:
            return 0

        for block_number, batches in grouped.items():
            if not batches:
                continue
            timestamp = timestamps.get(block_number)
            self._enqueue_block(block_number, timestamp, batches)

        total_appended = self._flush_ready_blocks(indexer)
        self.last_appended = total_appended
        return total_appended

    def flush_all_pending(self) -> int:
        indexer = self._address_indexer
        if indexer is None or not self._pending_blocks:
            return 0
        flushed = self._flush_ready_blocks(indexer, force=True)
        self.last_appended = flushed
        return flushed

    def _enqueue_block(
        self,
        block_number: int,
        timestamp: Optional[int],
        batches: List[Tuple[int, List[str]]],
    ) -> None:
        existing = self._pending_blocks.get(block_number)
        if existing:
            existing.batches.extend(batches)
            if timestamp is not None:
                existing.timestamp = timestamp
            return
        self._pending_blocks[block_number] = _PendingBlock(timestamp=timestamp, batches=batches)

    def _flush_ready_blocks(self, indexer, force: bool = False) -> int:
        if not self._pending_blocks:
            return 0
        now = time.time()
        ready_blocks = [
            block_number
            for block_number, pending in self._pending_blocks.items()
            if force or self._is_past_lag(pending.timestamp, now)
        ]
        if not ready_blocks:
            return 0

        total_batches = 0
        payload: List[Tuple[int, List[Tuple[int, List[str]]]]] = []
        sorted_ready = sorted(ready_blocks)
        limit = ADDRESS_TX_MAX_FLUSH_BLOCKS if not force else len(sorted_ready)
        for block_number in sorted_ready[:limit]:
            pending = self._pending_blocks.pop(block_number, None)
            if pending is None or not pending.batches:
                continue
            payload.append((block_number, pending.batches))
            total_batches += len(pending.batches)

        if not payload:
            return 0

        appended_counts = indexer.write_transactions_batch(payload)
        total_appended = int(sum(appended_counts)) if appended_counts else 0
        return total_appended

    def _is_past_lag(self, block_timestamp: Optional[int], now: float) -> bool:
        if self._write_lag_seconds == 0:
            return True
        if block_timestamp is None:
            return True
        return now - block_timestamp >= self._write_lag_seconds

    @staticmethod
    def _extract_addresses(tx: object):
        addresses = getattr(tx, "unique_addresses", None)
        if addresses is None and isinstance(tx, dict):
            addresses = tx.get("unique_addresses")
        return addresses

    @staticmethod
    def _extract_int_field(tx: object, field: str) -> Optional[int]:
        value = getattr(tx, field, None)
        if value is None and isinstance(tx, dict):
            value = tx.get(field)
        if value is None:
            return None
        try:
            return int(value)
        except (TypeError, ValueError):
            return None

    @staticmethod
    def _extract_block_timestamp(tx: object) -> Optional[int]:
        timestamp = getattr(tx, "block_timestamp", None)
        if timestamp is None and isinstance(tx, dict):
            timestamp = tx.get("block_timestamp")
        if timestamp is None:
            return None
        try:
            return int(timestamp)
        except (TypeError, ValueError):
            return None
