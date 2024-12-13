from web3 import Web3
from web3.types import TxData, LogReceipt
from typing import List, Dict, Any, Optional


class TransactionDataFetcher:
    def __init__(self, w3: Web3):
        self.w3 = w3

    def get_transaction_data(self, tx_hash: str) -> Dict[str, Any]:
        transaction = self.get_base_transaction(tx_hash)
        receipt = self.get_transaction_receipt(tx_hash)
        trace = self.get_transaction_trace(tx_hash)
        state_diff = self.get_state_diff(tx_hash)
        return {
            'transaction': transaction,
            'receipt': receipt,
            'trace': trace,
            'state_diff': state_diff,
        }
    
    def get_base_transaction(self, tx_hash: str) -> Dict[str, Any]:
        return self.w3.eth.get_transaction(tx_hash)
    
    def get_transaction_receipt(self, tx_hash: str) -> Dict[str, Any]:
        return self.w3.eth.get_transaction_receipt(tx_hash)

    def get_transaction_trace(self, tx_hash: str) -> Dict[str, Any]:
        return self.w3.manager.request_blocking(
            "debug_traceTransaction", [tx_hash, {"tracer": "callTracer"}]
        )

    def get_state_diff(self, tx_hash: str) -> Dict[str, Any]:
        return self.w3.manager.request_blocking(
            "debug_traceTransaction", 
            [tx_hash, {"tracer": "prestateTracer"}]
        )