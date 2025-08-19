from web3 import Web3
from typing import List, Dict, Any, Optional, Tuple
import asyncio
import aiohttp

# Import ethtx for direct database access (10-40x faster than RPC)
try:
    import ethtx
    RS_TX_PROCESSOR_AVAILABLE = True
except ImportError:
    RS_TX_PROCESSOR_AVAILABLE = False
    print("Warning: ethtx not available. Falling back to RPC calls.")


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
        
        # Initialize ethtx if available for massive speedup
        if RS_TX_PROCESSOR_AVAILABLE:
            try:
                self.rust_processor = ethtx.TxProcessor()
                self.use_rust = True
                print("✅ Using ethtx for 10-40x performance improvement")
            except Exception as e:
                print(f"Warning: Failed to initialize ethtx: {e}")
                self.use_rust = False
        else:
            self.use_rust = False
        
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

            if not 'error' in receipts_data:
                # Map results to transaction hashes
                receipt_map = {
                    receipt['transactionHash']: receipt 
                    for receipt in receipts_data['result']
                }
            else:
                receipt_map = receipts_data
            if not 'error' in traces_data:
                # Map traces to transaction hashes
                trace_map = {}
                for tx_hash, trace in zip(receipt_map.keys(), traces_data['result']):
                    if trace and 'result' in trace:
                        trace_map[tx_hash] = trace['result']
            else:
                trace_map = traces_data

        return receipt_map, trace_map
        
    async def fetch_transaction_list_data(self, tx_hashes: List[str]) -> Tuple[Dict[str, Any], Dict[str, Any], Dict[str, Any]]:
        """
        Efficiently fetch complete transaction data using ethtx (10-40x faster) 
        or fallback to RPC calls.
        
        Args:
            tx_hashes: List of transaction hashes to fetch
            
        Returns:
            Tuple of (transaction_map, receipt_map, trace_map) where keys are transaction hashes
        """
        if not tx_hashes:
            return {}, {}, {}
        
        # Use ethtx if available (10-40x faster than RPC)
        if self.use_rust:
            try:
                return await self._fetch_with_rust_processor(tx_hashes)
            except Exception as e:
                print(f"Warning: ethtx failed, falling back to RPC: {e}")
                # Fall through to RPC method
        
        # Fallback to original RPC implementation
        return await self._fetch_with_rpc(tx_hashes)
    
    async def _fetch_with_rust_processor(self, tx_hashes: List[str]) -> Tuple[Dict[str, Any], Dict[str, Any], Dict[str, Any]]:
        """
        Fetch transaction data using ethtx for massive speedup.
        """
        loop = asyncio.get_event_loop()
        
        # Process all transactions in parallel using Rust (10-40x faster)
        processed_txs = await loop.run_in_executor(
            None, 
            self.rust_processor.process_transactions_batch, 
            tx_hashes
        )
        
        transaction_map = {}
        receipt_map = {}
        trace_map = {}
        
        # Convert ProcessedTransaction objects to expected format
        for tx_hash, ptx in zip(tx_hashes, processed_txs):
            if not ptx:
                continue
                
            # Build transaction data
            transaction_map[tx_hash] = {
                'hash': tx_hash,
                'blockNumber': hex(ptx.block_number),
                'blockHash': '0x' + '0' * 64,  # Placeholder
                'from': ptx.from_address,
                'to': ptx.to_address,
                'value': hex(int(ptx.value)),
                'nonce': hex(ptx.nonce),
                'gas': hex(ptx.fees.get('gas_used', 0) * 2),  # Estimate gas limit
                'gasPrice': hex(int(ptx.fees.get('gas_price', 0))),
                'input': ptx.input,
                'transactionIndex': hex(ptx.txn_index),
                'type': '0x2' if ptx.fees.get('max_fee_per_gas') else '0x0',
            }
            
            # Build receipt data with logs
            logs = self._convert_events_to_logs(ptx)
            receipt_map[tx_hash] = {
                'transactionHash': tx_hash,
                'blockNumber': hex(ptx.block_number),
                'blockHash': '0x' + '0' * 64,  # Placeholder
                'from': ptx.from_address,
                'to': ptx.to_address,
                'contractAddress': ptx.contract_address,
                'cumulativeGasUsed': hex(ptx.fees.get('gas_used', 0)),
                'gasUsed': hex(ptx.fees.get('gas_used', 0)),
                'effectiveGasPrice': hex(int(ptx.fees.get('gas_price', 0))),
                'status': '0x1' if ptx.status == '1' else '0x0',
                'logs': logs,
                'logsBloom': '0x' + '0' * 512,  # Placeholder
                'transactionIndex': hex(ptx.txn_index),
                'type': '0x2' if ptx.fees.get('max_fee_per_gas') else '0x0',
            }
            
            # Build trace data
            trace_map[tx_hash] = self._convert_to_call_trace(ptx)
            
        return transaction_map, receipt_map, trace_map
    
    def _convert_events_to_logs(self, ptx) -> List[Dict[str, Any]]:
        """Convert ProcessedTransaction events to Ethereum logs."""
        logs = []
        
        # Convert ERC20 transfers
        for transfer in ptx.erc20_transfers:
            logs.append({
                'address': transfer.get('token_address'),
                'topics': [
                    '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef',  # Transfer
                    self._pad_address(transfer.get('from_address')),
                    self._pad_address(transfer.get('to_address'))
                ],
                'data': self._pad_uint256(transfer.get('amount', 0)),
                'blockNumber': hex(ptx.block_number),
                'transactionHash': f"0x{ptx.hash}",
                'transactionIndex': hex(ptx.txn_index),
                'blockHash': '0x' + '0' * 64,
                'logIndex': hex(transfer.get('log_index', len(logs))),
                'removed': False
            })
        
        # Convert Uniswap V2 swaps
        for swap in ptx.uniswap_v2_swaps:
            logs.append({
                'address': swap.get('pair_address'),
                'topics': [
                    '0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822',  # Swap
                    self._pad_address(swap.get('sender')),
                    self._pad_address(swap.get('to'))
                ],
                'data': self._encode_swap_data(swap),
                'blockNumber': hex(ptx.block_number),
                'transactionHash': f"0x{ptx.hash}",
                'transactionIndex': hex(ptx.txn_index),
                'blockHash': '0x' + '0' * 64,
                'logIndex': hex(swap.get('log_index', len(logs))),
                'removed': False
            })
            
        return logs
    
    def _convert_to_call_trace(self, ptx) -> Dict[str, Any]:
        """Convert internal transactions to call trace format."""
        trace = {
            'type': 'CALL',
            'from': ptx.from_address,
            'to': ptx.to_address or '0x0000000000000000000000000000000000000000',
            'value': hex(int(ptx.value)),
            'gas': hex(ptx.fees.get('gas_used', 0) * 2),
            'gasUsed': hex(ptx.fees.get('gas_used', 0)),
            'input': ptx.input,
            'output': '0x',
            'calls': []
        }
        
        # Add internal transactions as nested calls
        for itx in ptx.internal_transactions:
            call = {
                'type': itx.get('trace_type', 'CALL'),
                'from': itx.get('from_address'),
                'to': itx.get('to_address'),
                'value': hex(int(itx.get('value', 0))),
                'gas': hex(itx.get('gas_used', 0) * 2),
                'gasUsed': hex(itx.get('gas_used', 0)),
                'input': '0x',
                'output': '0x'
            }
            trace['calls'].append(call)
            
        return trace
    
    def _pad_address(self, address: str) -> str:
        """Pad address to 32 bytes for topic format."""
        if not address:
            return '0x' + '0' * 64
        address = address.replace('0x', '').lower()
        return '0x' + '0' * (64 - len(address)) + address
    
    def _pad_uint256(self, value) -> str:
        """Pad uint256 value to 32 bytes."""
        if isinstance(value, str):
            value = int(value)
        hex_value = hex(value)[2:]
        return '0x' + '0' * (64 - len(hex_value)) + hex_value
    
    def _encode_swap_data(self, swap) -> str:
        """Encode swap amounts as data field."""
        data = ''
        data += self._pad_uint256(swap.get('amount0_in', 0))[2:]
        data += self._pad_uint256(swap.get('amount1_in', 0))[2:]
        data += self._pad_uint256(swap.get('amount0_out', 0))[2:]
        data += self._pad_uint256(swap.get('amount1_out', 0))[2:]
        return '0x' + data
    
    async def _fetch_with_rpc(self, tx_hashes: List[str]) -> Tuple[Dict[str, Any], Dict[str, Any], Dict[str, Any]]:
        """
        Original RPC implementation as fallback.
        """
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
        
