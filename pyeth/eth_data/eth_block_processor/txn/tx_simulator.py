"""
Transaction Simulator for Ethereum Transactions

This module provides a dedicated class for simulating Ethereum transactions
before they are mined. It supports multiple simulation strategies to extract
comprehensive information about transaction effects.

Algorithm:
1. Prepare transaction parameters for simulation
2. Execute simulation using available methods (trace_call, debug_traceCall, etc.)
3. Process simulation results to extract trace data, state changes, and potential events
4. Return a structured simulation result with all relevant data

Simulation Methods:
------------------
- trace_call: Provides detailed execution trace and state differences
- debug_traceCall: Offers VM-level tracing with custom tracers
- eth_call: Basic execution simulation with revert detection

This implementation prioritizes extracting maximum information from simulations
to support accurate analysis of transaction effects, including:
- Internal ETH transfers
- Token transfers (direct and indirect)
- State changes
- Contract interactions
- Potential reverts
"""

import asyncio
from typing import Dict, Any, List, Optional, Union, Tuple
from web3 import Web3


class TransactionSimulator:
    """
    Dedicated class for simulating transactions and extracting comprehensive results.
    
    This class focuses exclusively on transaction simulation, providing multiple
    strategies to extract the maximum amount of information from pre-execution
    simulation of transactions.
    """
    
    def __init__(self, w3: Optional[Web3] = None, logger=None):
        """
        Initialize the transaction simulator.
        
        Args:
            w3: Web3 instance for blockchain interaction
            logger: Optional logger for debugging
        """
        self.w3 = w3 or Web3()
        self.logger = logger
        self.trace_options = ['trace', 'stateDiff'] # ['trace', 'stateDiff', 'vmTrace']

    def _safe_hex_to_int(self, value):
        """Convert hex string to integer, handling different input formats."""
        if isinstance(value, int):
            return value
        if isinstance(value, str):
            return int(value, 16) if value.startswith('0x') else int(value)
        return 0
    
    async def simulate_transaction(self, txn: Dict[str, Any], 
                                  block_identifier: Union[str, int] = 'latest', 
                                  simulation_type: str = 'trace_call') -> Dict[str, Any]:
        """
        Simulate a transaction using the specified simulation type.
        
        Args:
            txn: Transaction to simulate
            block_identifier: Block state to use for simulation
            simulation_type: Simulation strategy to use
            
        Returns:
            Dictionary containing simulation results with:
            - trace: Execution trace data
            - stateDiff: State changes
            - revert: Information about potential reverts
            - logs: Attempt to capture logs (if available)
        """
        # Prepare transaction for simulation
        sim_txn = await self._prepare_transaction(txn)
        
        # Format block identifier
        formatted_block_id = self._format_block_identifier(block_identifier)
        
        # Initialize result structure
        simulation_result = {
            'success': False,
            'trace': None,
            'stateDiff': None,
            'revert': {
                'would_revert': False,
                'reason': None
            }
        }
        
        # Check if transaction would revert
        revert_info = await self._check_revert(sim_txn, formatted_block_id)
        simulation_result['revert'] = revert_info
        
        # If transaction would revert and we wanted basic revert check, we're done
        if revert_info['would_revert'] and simulation_type == 'eth_call':
            return simulation_result
        
        # Get detailed simulation data
        if simulation_type == 'trace_call':
            trace_result = await self._execute_trace_call(sim_txn, formatted_block_id)
            if trace_result:
                simulation_result.update(trace_result)
                simulation_result['success'] = True
                
        elif simulation_type == 'debug_traceCall':
            debug_result = await self._execute_debug_trace_call(sim_txn, formatted_block_id)
            if debug_result:
                simulation_result.update(debug_result)
                simulation_result['success'] = True
        
        return simulation_result
    
    async def _prepare_transaction(self, txn: Dict[str, Any]) -> Dict[str, Any]:
        """
        Prepare a transaction for simulation by normalizing all fields.
        
        Args:
            txn: Raw transaction data
            
        Returns:
            Normalized transaction dictionary ready for simulation
        """
        sim_txn = {
            'from': self.w3.to_checksum_address(txn['from']),
            'data': txn.get('input', txn.get('data', '0x')),
            'value': self._safe_hex_to_int(txn.get('value', 0)),
            'gas': self._safe_hex_to_int(txn.get('gas', 0)),
            'gasPrice': self._safe_hex_to_int(txn.get('gasPrice', 0))
        }
        
        # Add 'to' only if it exists (contract creation transactions don't have 'to')
        if txn.get('to'):
            sim_txn['to'] = self.w3.to_checksum_address(txn.get('to'))
            
        # Remove any None values
        sim_txn = {k: v for k, v in sim_txn.items() if v is not None}
        
        return sim_txn
    
    def _format_block_identifier(self, block_identifier: Union[str, int]) -> Union[str, Dict]:
        """
        Format block identifier according to EIP-1898.
        
        Args:
            block_identifier: Block identifier (number, hash, or tag)
            
        Returns:
            Properly formatted block identifier
        """
        if isinstance(block_identifier, int):
            return hex(block_identifier)  # Convert to hex for EIP-1898 compliance
        return block_identifier
    
    async def _check_revert(self, sim_txn: Dict[str, Any], block_identifier: Union[str, Dict]) -> Dict[str, Any]:
        """
        Check if a transaction would revert using eth_call.
        
        Args:
            sim_txn: Transaction to check
            block_identifier: Block state to use
            
        Returns:
            Dictionary with revert info
        """
        revert_info = {
            'would_revert': False,
            'reason': None
        }
        
        try:
            # Use eth_call to check for reverts
            await asyncio.to_thread(self.w3.eth.call, sim_txn, block_identifier)
        except Exception as e:
            revert_info['would_revert'] = True
            revert_info['reason'] = str(e)
            if self.logger:
                self.logger.debug(f"Transaction would revert: {str(e)}")
        
        return revert_info
    
    async def _execute_trace_call(self, sim_txn: Dict[str, Any], block_identifier: Union[str, Dict]) -> Dict[str, Any]:
        """
        Execute a trace_call simulation.
        
        Args:
            sim_txn: Transaction to simulate
            block_identifier: Block state to use
            
        Returns:
            Dictionary with trace and stateDiff data
        """
        # Clean the transaction values to prevent floating point errors
        cleaned_txn = {}
        for key, value in sim_txn.items():
            if key in ['value', 'gas', 'gasPrice'] and isinstance(value, (int, float)):
                cleaned_txn[key] = hex(int(value))  # Convert to hex to avoid floating point
            else:
                cleaned_txn[key] = value
        
        trace_params = [cleaned_txn, self.trace_options, block_identifier]
        
        try:
            # Use timeout to prevent blocking
            result = await asyncio.wait_for(
                asyncio.to_thread(
                    self.w3.provider.make_request,
                    "trace_call",
                    trace_params
                ),
                timeout=1.0  # 1 second timeout
            )
            
            if 'result' in result:
                return result['result']
            
            # Handle specific error cases silently
            if 'error' in result:
                error_code = result['error'].get('code')
                error_msg = result['error'].get('message', '')
                
                # Don't log warnings for common expected errors
                if (error_code == -32003 and 'insufficient funds' in error_msg) or \
                   (error_code == -32602 and 'Invalid params' in error_msg) or \
                   (error_code == -32008 and 'Response is too big' in error_msg):
                    if self.logger and self.logger.isEnabledFor(10):  # DEBUG level
                        self.logger.debug(f"trace_call issue: {error_code}")
                else:
                    # Only log unusual errors as warnings
                    if self.logger:
                        self.logger.warning(f"trace_call failed: {result['error']}")
                
        except asyncio.TimeoutError:
            if self.logger and self.logger.isEnabledFor(10):  # DEBUG level
                self.logger.debug("trace_call timed out")
        except Exception as e:
            # Only log unexpected exceptions as warnings
            if "insufficient funds" in str(e) or "Invalid params" in str(e) or "Response is too big" in str(e):
                if self.logger and self.logger.isEnabledFor(10):  # DEBUG level
                    self.logger.debug(f"Expected trace_call error: {str(e)[:30]}...")
            else:
                if self.logger:
                    self.logger.warning(f"Unexpected error in trace_call: {str(e)}")
            
        return None
    
    async def _execute_debug_trace_call(self, sim_txn: Dict[str, Any], block_identifier: Union[str, Dict]) -> Dict[str, Any]:
        """
        Execute a debug_traceCall with custom tracer.
        
        Args:
            sim_txn: Transaction to simulate
            block_identifier: Block state to use
            
        Returns:
            Dictionary with tracer-specific results
        """
        # This requires a node with debug_traceCall support
        tracer_config = {
            "tracer": "callTracer",
            "tracerConfig": {
                "onlyTopCall": False,
                "withLog": True
            }
        }
        
        debug_params = [sim_txn, block_identifier, tracer_config]
        
        try:
            result = await asyncio.to_thread(
                self.w3.provider.make_request,
                "debug_traceCall",
                debug_params
            )
            
            if 'result' in result:
                return {'debug_trace': result['result']}
            
            if self.logger:
                self.logger.warning(f"debug_traceCall failed: {result.get('error', 'Unknown error')}")
                
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error in debug_traceCall: {str(e)}")
                
        return None

    async def simulate_transactions_batch(self, 
                                         txns: List[Dict[str, Any]], 
                                         block_identifier: Union[str, int] = 'latest',
                                         simulation_type: str = 'trace_call') -> List[Dict[str, Any]]:
        """
        Simulate multiple transactions in a batch operation.
        
        Algorithm:
        1. Prepare all transactions for simulation
        2. Create a batch of RPC requests
        3. Execute the batch request
        4. Process the results and map them back to the original transactions
        
        Args:
            txns: List of transactions to simulate
            block_identifier: Block state to use for simulation
            simulation_type: Simulation strategy to use
            
        Returns:
            List of simulation results in the same order as the input transactions
        """
        if not txns:
            return []
        
        # Format block identifier once for all transactions
        formatted_block_id = self._format_block_identifier(block_identifier)
        
        # Prepare all transactions for simulation
        prepared_txns = []
        for txn in txns:
            prepared_txn = await self._prepare_transaction(txn)
            prepared_txns.append(prepared_txn)
        
        # Initialize results with transaction hashes for identification
        results = []
        
        # Build batch requests based on simulation type
        if simulation_type == 'trace_call':
            # Create batch of trace_call requests
            batch_results = await self._execute_trace_call_batch(prepared_txns, formatted_block_id)
            results = batch_results
            
        elif simulation_type == 'eth_call':
            # Create batch of eth_call requests for revert checking
            batch_results = await self._check_revert_batch(prepared_txns, formatted_block_id)
            results = batch_results
            
        elif simulation_type == 'debug_traceCall':
            # Create batch of debug_traceCall requests
            batch_results = await self._execute_debug_trace_call_batch(prepared_txns, formatted_block_id)
            results = batch_results
        
        # If we need both trace_call and revert checks, we can combine them
        if simulation_type == 'comprehensive':
            # First check for reverts
            revert_results = await self._check_revert_batch(prepared_txns, formatted_block_id)
            
            # Then get trace data for transactions that wouldn't revert
            trace_txns = []
            trace_indices = []
            
            for i, revert_info in enumerate(revert_results):
                results.append({
                    'revert': revert_info,
                    'trace': None,
                    'stateDiff': None,
                    'success': not revert_info['would_revert']
                })
                
                # Add to trace batch if it wouldn't revert
                if not revert_info['would_revert']:
                    trace_txns.append(prepared_txns[i])
                    trace_indices.append(i)
            
            # Execute trace_call for non-reverting transactions
            if trace_txns:
                trace_results = await self._execute_trace_call_batch(trace_txns, formatted_block_id)
                
                # Map trace results back to the original indices
                for batch_idx, result_idx in enumerate(trace_indices):
                    if batch_idx < len(trace_results) and trace_results[batch_idx]:
                        results[result_idx].update(trace_results[batch_idx])
        
        return results

    async def _execute_trace_call_batch(self, 
                                       prepared_txns: List[Dict[str, Any]], 
                                       block_identifier: Union[str, Dict]) -> List[Dict[str, Any]]:
        """
        Execute trace_call for multiple transactions as a batch.
        
        Args:
            prepared_txns: List of prepared transactions
            block_identifier: Block state to use
            
        Returns:
            List of trace_call results
        """
        batch_requests = []
        
        # Build the batch request array
        for i, txn in enumerate(prepared_txns):
            trace_params = [txn, ['trace', 'stateDiff'], block_identifier]
            
            batch_requests.append({
                "jsonrpc": "2.0",
                "method": "trace_call",
                "params": trace_params,
                "id": i + 1  # Use index+1 as the request ID
            })
        
        try:
            # Send the batch request
            # Note: Web3.py doesn't directly support batch requests, so we use the provider directly
            batch_response = await asyncio.to_thread(
                self._send_batch_request,
                batch_requests
            )
            
            # Process results
            results = []
            for i in range(len(prepared_txns)):
                # Find the corresponding response by ID
                response = next((r for r in batch_response if r.get('id') == i + 1), None)
                
                if response and 'result' in response:
                    results.append({
                        'success': True,
                        'trace': response['result'].get('trace'),
                        'stateDiff': response['result'].get('stateDiff')
                    })
                else:
                    # Handle error or missing response
                    results.append({
                        'success': False,
                        'error': response.get('error', {'message': 'Missing or invalid response'})
                        if response else {'message': 'No response received'}
                    })
                
            return results
            
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error in batch trace_call: {str(e)}")
            
            # Return a list of failed results
            return [{'success': False, 'error': {'message': str(e)}} for _ in prepared_txns]

    async def _check_revert_batch(self, 
                                   prepared_txns: List[Dict[str, Any]], 
                                   block_identifier: Union[str, Dict]) -> List[Dict[str, Any]]:
        """
        Check for reverts for multiple transactions as a batch.
        
        Args:
            prepared_txns: List of prepared transactions
            block_identifier: Block state to use
            
        Returns:
            List of revert check results
        """
        batch_requests = []
        
        # Build the batch request array
        for i, txn in enumerate(prepared_txns):
            batch_requests.append({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [txn, block_identifier],
                "id": i + 1
            })
        
        try:
            # Send the batch request
            batch_response = await asyncio.to_thread(
                self._send_batch_request,
                batch_requests
            )
            
            # Process results
            results = []
            for i in range(len(prepared_txns)):
                # Find the corresponding response by ID
                response = next((r for r in batch_response if r.get('id') == i + 1), None)
                
                revert_info = {
                    'would_revert': False,
                    'reason': None
                }
                
                if response:
                    if 'error' in response:
                        revert_info['would_revert'] = True
                        revert_info['reason'] = response['error'].get('message', 'Unknown error')
                else:
                    revert_info['would_revert'] = True
                    revert_info['reason'] = 'No response received'
                
                results.append(revert_info)
                
            return results
            
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error in batch eth_call: {str(e)}")
            
            # Return a list of failed results indicating potential reverts
            return [{'would_revert': True, 'reason': str(e)} for _ in prepared_txns]

    def _send_batch_request(self, batch_requests):
        """
        Send a batch JSON-RPC request to the Ethereum node.
        
        Args:
            batch_requests: List of JSON-RPC request objects
            
        Returns:
            List of response objects
        """
        # Access the underlying provider to send the batch request
        provider = self.w3.provider
        
        # Different providers might have different ways to handle batch requests
        if hasattr(provider, 'make_batch_request'):
            # Some providers have a built-in batch method
            return provider.make_batch_request(batch_requests)
        else:
            # For providers without a built-in batch method, we need to manually construct the request
            endpoint_uri = provider.endpoint_uri if hasattr(provider, 'endpoint_uri') else None
            
            if not endpoint_uri:
                raise ValueError("Provider does not have an endpoint_uri attribute")
            
            import requests
            import json
            
            headers = {'Content-Type': 'application/json'}
            response = requests.post(
                endpoint_uri,
                data=json.dumps(batch_requests),
                headers=headers
            )
            
            if response.status_code == 200:
                return response.json()
            else:
                raise ValueError(f"Batch request failed with status code {response.status_code}")
