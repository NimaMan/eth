from web3.types import Hash32, Wei
from dataclasses import dataclass
from typing import List
from eth_data.tx_processor.data_models.txn_models import ProcessedTransaction


@dataclass
class BlockTransactions:
    number: int
    hash: Hash32
    timestamp: int
    gas_used: int
    gas_limit: int
    base_fee_per_gas: Wei
    transactions: List[ProcessedTransaction]
    parent_hash: Hash32
    state_root: Hash32
    transactions_root: Hash32
    receipts_root: Hash32