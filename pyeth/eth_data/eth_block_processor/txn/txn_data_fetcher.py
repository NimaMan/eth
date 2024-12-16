from web3 import Web3
from web3.types import TxData, LogReceipt
from typing import List, Dict, Any, Optional, Tuple
import asyncio
import aiohttp
import json
from hexbytes import HexBytes


class TransactionDataFetcher:
    def __init__(self, w3: Web3):
        self.w3 = w3

    def get_transaction_data(self, tx_hash: str, receipt: bool = True, trace: bool = True, state_diff: bool = False) -> Dict[str, Any]:
        transaction = self.get_base_transaction(tx_hash)
        if receipt: 
            receipt = self.get_transaction_receipt(tx_hash)
        if trace:
            trace = self.get_transaction_trace(tx_hash)
        if state_diff:
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
    

class BatchTransactionDataFetcher:
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.endpoint_url = w3.provider.endpoint_uri
        
    async def fetch_block_data(self, block_number: int) -> Tuple[Dict[str, Any], Dict[str, Any]]:
        """
        Fetch all receipts and traces for a block using RETH's optimized methods
        Returns: (receipt_map, trace_map) where keys are transaction hashes
        """
        # Use RETH's block_receipts method
        receipts_request = {
            "jsonrpc": "2.0",
            "method": "eth_getBlockReceipts",
            "params": [hex(block_number)],
            "id": 1
        }
        
        # Use RETH's debug_traceBlockByNumber
        traces_request = {
            "jsonrpc": "2.0",
            "method": "debug_traceBlockByNumber",
            "params": [hex(block_number), {"tracer": "callTracer"}],
            "id": 2
        }
        
        async with aiohttp.ClientSession() as session:
            receipts_task = session.post(
                self.endpoint_url,
                json=receipts_request,
                headers={'Content-Type': 'application/json'}
            )
            traces_task = session.post(
                self.endpoint_url,
                json=traces_request,
                headers={'Content-Type': 'application/json'}
            )
                
            receipts_response, traces_response = await asyncio.gather(
                receipts_task,
                traces_task
            )
                
            receipts_data = await receipts_response.json()
            traces_data = await traces_response.json()

            if 'error' in receipts_data:
                raise Exception(f"RPC error in receipts: {receipts_data['error']}")
            if 'error' in traces_data:
                raise Exception(f"RPC error in traces: {traces_data['error']}")

            # Map results to transaction hashes
            receipt_map = {
                receipt['transactionHash']: receipt 
                for receipt in receipts_data['result']
            }
            
            # Map traces to transaction hashes
            trace_map = {}
            for tx_hash, trace in zip(receipt_map.keys(), traces_data['result']):
                if trace and 'result' in trace:
                    trace_map[tx_hash] = trace['result']
            
        return receipt_map, trace_map
        
