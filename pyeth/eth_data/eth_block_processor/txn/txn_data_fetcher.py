from web3 import Web3
from typing import List, Dict, Any, Optional, Tuple
import asyncio
import aiohttp


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
    

class TransactionBatchDataFetcher:
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
        
    async def fetch_transaction_list_data(self, tx_hashes: List[str]) -> Tuple[Dict[str, Any], Dict[str, Any], Dict[str, Any]]:
        """
        Efficiently fetch complete transaction data (transactions, receipts, and traces) in a single operation
        with optimized batching of all RPC calls.
        
        Args:
            tx_hashes: List of transaction hashes to fetch
            
        Returns:
            Tuple of (transaction_map, receipt_map, trace_map) where keys are transaction hashes
        """
        if not tx_hashes:
            return {}, {}, {}
        
        transaction_map = {}
        receipt_map = {}
        trace_map = {}
        
        # Create all batch requests at once
        async with aiohttp.ClientSession() as session:
            # Prepare all batch requests - transactions, receipts, and traces
            tx_batch_requests = []
            receipt_batch_requests = []
            trace_batch_requests = []
            
            for i, tx_hash in enumerate(tx_hashes):
                # Transaction request
                tx_batch_requests.append({
                    "jsonrpc": "2.0",
                    "method": "eth_getTransactionByHash",
                    "params": [tx_hash],
                    "id": i
                })
                
                # Receipt request
                receipt_batch_requests.append({
                    "jsonrpc": "2.0",
                    "method": "eth_getTransactionReceipt",
                    "params": [tx_hash],
                    "id": i
                })
                
                # Trace request
                trace_batch_requests.append({
                    "jsonrpc": "2.0",
                    "method": "debug_traceTransaction",
                    "params": [tx_hash, {"tracer": "callTracer"}],
                    "id": i
                })
            
            # Execute all batch requests concurrently
            tasks = [
                session.post(
                    self.w3.provider.endpoint_uri,
                    json=tx_batch_requests,
                    headers={'Content-Type': 'application/json'}
                ),
                session.post(
                    self.w3.provider.endpoint_uri,
                    json=receipt_batch_requests,
                    headers={'Content-Type': 'application/json'}
                ),
                session.post(
                    self.w3.provider.endpoint_uri,
                    json=trace_batch_requests,
                    headers={'Content-Type': 'application/json'}
                )
            ]
            
            tx_response, receipt_response, trace_response = await asyncio.gather(*tasks)
            
            # Process transaction results
            if tx_response.status == 200:
                tx_results = await tx_response.json()
                for result in tx_results:
                    if 'result' in result and result['result']:
                        request_id = result['id']
                        tx_hash = tx_hashes[request_id]
                        transaction_map[tx_hash] = result['result']
            
            # Process receipt results
            if receipt_response.status == 200:
                receipt_results = await receipt_response.json()
                for result in receipt_results:
                    if 'result' in result and result['result']:
                        request_id = result['id']
                        tx_hash = tx_hashes[request_id]
                        receipt_map[tx_hash] = result['result']
            
            # Process trace results
            if trace_response.status == 200:
                trace_results = await trace_response.json()
                for result in trace_results:
                    if 'result' in result and result['result']:
                        request_id = result['id']
                        tx_hash = tx_hashes[request_id]
                        trace_map[tx_hash] = result['result']
        
        return transaction_map, receipt_map, trace_map
        
