from collections import defaultdict
from typing import Iterable, List
from eth_data.utils.pyreth_client import PyrethClient


class TransactionAddresstoTxIndexer:
   
    def __init__(self) -> None:
        self._address_indexer = PyrethClient.instance().address_indexer()
        self.last_appended = 0

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

        grouped = defaultdict(list)
        for tx in transactions:
            block_key = int(tx.block_number)
            tx_index = int(tx.tx_index)

            addresses = tx.unique_addresses
            if not addresses:
                continue

            grouped[block_key].append((tx_index, sorted(addresses)))
        if not grouped:
            return 0

        total_appended = 0
        for block_number, batches in grouped.items():
            if not batches:
                continue
            appended = indexer.write_transactions(block_number, batches)
            if appended:
                total_appended += int(appended)
        self.last_appended = total_appended
        return total_appended