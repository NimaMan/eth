"""
Mempool Transaction Processor for Ethereum Nodes

Algorithm:
1. Retrieve a pending transaction from the mempool
2. Simulate the transaction execution using trace_call
3. Extract both logs and trace data from the simulation
4. Process logs using TransactionLogProcessor
5. Process traces using relevant trace models
6. Combine the results into a comprehensive transaction data structure

Transaction Simulation Methods:
-------------------------------

1. trace_call
   This method simulates a transaction and provides detailed execution information.
   
   Parameters:
   - Transaction Object: The transaction to simulate (from, to, value, data, gas, etc.)
   - Trace Types: What kinds of trace data to collect
     * 'trace': Records all calls and creates during execution
     * 'vmTrace': Provides detailed info about each EVM operation
     * 'stateDiff': Shows state changes that would result from the transaction
   - Block Parameter: Which block state to use for simulation
     * 'latest': Latest block state (default)
     * 'pending': Current pending state
     * Specific block number: Integer or hex string (e.g., 15001000 or '0xE4E1F8')
     * Block hash: Hex string of a specific block hash
   
   Example:
   ```
   trace_params = [
       transaction_dict,            # Transaction to simulate
       ['trace', 'vmTrace'],        # Types of trace data
       'latest'                     # Block state to use
   ]
   ```

2. debug_traceCall
   Available on some Ethereum clients (Geth, Erigon), provides detailed execution traces.
   
   Parameters:
   - Transaction Object: The transaction to simulate
   - Block Parameter: Which block state to use
   - Trace Options: Configuration for the tracer
     * 'tracer': Name of the tracer to use (e.g., 'callTracer', 'prestate')
     * 'timeout': How long to allow the trace to run (e.g., '5s')
     * Additional tracer-specific options

3. eth_call
   A lighter-weight simulation that doesn't provide trace data but can detect reverts.
   
   Parameters:
   - Transaction Object: The transaction to simulate
   - Block Parameter: Which block state to use
   - State Override (optional): Allows modifying state during simulation

Log Extraction:
--------------
Logs are not directly available from trace_call results and must be extracted by:
1. Analyzing the trace data for LOG opcodes
2. Looking for token transfer method signatures (0xa9059cbb for ERC20 transfer)
3. Decoding method inputs to extract token transfer details

Token transfers are identified by examining:
1. Method signatures in transaction input data
2. Internal calls made during transaction execution
3. State changes resulting from the transaction

This processor optimizes for different Ethereum clients (Geth, Reth, Erigon, etc.)
and falls back to different methods based on available APIs.
"""

from typing import Dict, Any, List, Optional, Union
from web3 import Web3

from eth_data.tx_processor.data_models import *
from eth_data.tx_processor.tx_simulator import TransactionSimulator


class StateDiffProcessor:
    """
    Analyzes state differences from transaction simulation to extract
    token balance changes and other state modifications.
    """
    
    def __init__(self, w3: Web3 = None, logger=None):
        """Initialize with Web3 instance and optional logger."""
        self.w3 = w3 or Web3()
        self.logger = logger
        self.tx_simulator = TransactionSimulator(w3=self.w3, logger=logger)

    async def extract_state_diffs(self, txn: Dict[str, Any]) -> Dict[str, Dict]:
        """
        Extract state diffs from a transaction.
        
        Returns a dictionary mapping addresses to their balance changes.
        """
        try:
            # Simulate the transaction - always use 'latest' for block_identifier to avoid 'block not found' errors
            simulation_result = await self.tx_simulator.simulate_transaction(txn, block_identifier='latest')
            
            # Check if simulation failed or has no state diff
            if not simulation_result or not simulation_result.get('success'):
                if simulation_result and simulation_result.get('revert', {}).get('would_revert'):
                    reason = simulation_result.get('revert', {}).get('reason')
                    if self.logger:
                        self.logger.debug(f"Transaction would revert: {reason}")
                return None
                
            # Process the state diff if present
            if not simulation_result.get('stateDiff'):
                if self.logger:
                    self.logger.debug("No state diff in simulation result")
                return None
                
            return self.process_state_diff(simulation_result)
            
        except Exception as e:
            if "Response is too big" in str(e):
                if self.logger:
                    self.logger.warning(f"trace_call failed: {e}")
            else:
                if self.logger:
                    self.logger.error(f"Error extracting state diff for {txn.get('hash')}: {e}")
            return None
    
    def process_state_diff(self, simulation_result) -> Dict[str, Dict]:
        """Process the state diff from a transaction simulation and extract ETH balance changes."""
        eth_balance_changes = {}
        
        # Check if we have state diff data
        state_diff = simulation_result.get("stateDiff")
        if not state_diff:
            return eth_balance_changes
            
        # Process each address in the state diff
        for address, changes in state_diff.items():
            try:
                # Skip if no balance changes or balance field is missing
                if not changes or 'balance' not in changes:
                    continue
                    
                # Convert to checksum address
                checksum_address = self.w3.to_checksum_address(address)
                
                # Skip unchanged balances
                balance_change = changes.get('balance')
                if balance_change == '=' or not balance_change:
                    continue
                    
                # Extract balance change
                balance_info = self._extract_balance_change(balance_change)
                if balance_info:
                    eth_balance_changes[checksum_address] = balance_info
                    
            except Exception as e:
                if self.logger:
                    self.logger.debug(f"Error processing changes for address {address}: {e}")
                continue
        
        return eth_balance_changes
    
    def _extract_balance_change(self, balance_data: Dict) -> Optional[Dict]:
        """
        Extract ETH balance change from state diff structure.
        Handles different balance data formats.
        """
        try:
            # Handle different possible balance_data structures
            
            # Format 1: {'*': {'from': '0x...', 'to': '0x...'}}
            if '*' in balance_data and isinstance(balance_data['*'], dict):
                from_val = int(balance_data['*'].get('from', '0x0'), 16)
                to_val = int(balance_data['*'].get('to', '0x0'), 16)
                change = to_val - from_val
                
                return {
                    'before': from_val/10**18,
                    'after': to_val/10**18,
                    'change': change/10**18,  # Convert to ETH units
                }
                
            # Format 2: {'+': '0x...'} (incremental change)
            elif '+' in balance_data:
                change_val = int(balance_data['+'], 16) if balance_data['+'] != '0x0' else 0
                
                return {
                    'before': None,  # We don't know the 'before' value
                    'after': None,   # We don't know the 'after' value
                    'change': change_val/10**18,  # Convert to ETH units
                }
                
            # Format 3: {'-': '0x...'} (decremental change)
            elif '-' in balance_data:
                change_val = -int(balance_data['-'], 16) if balance_data['-'] != '0x0' else 0
                
                return {
                    'before': None,  # We don't know the 'before' value
                    'after': None,   # We don't know the 'after' value
                    'change': change_val/10**18,  # Convert to ETH units
                }
                
            # Format 4: {'from': '0x...', 'to': '0x...'} (direct values)
            elif 'from' in balance_data and 'to' in balance_data:
                from_val = int(balance_data['from'], 16)
                to_val = int(balance_data['to'], 16)
                change = to_val - from_val
                
                return {
                    'before': from_val,
                    'after': to_val,
                    'change': change/10**18,  # Convert to ETH units
                }
                
            else:
                if self.logger:
                    self.logger.debug(f"Unknown balance data format: {balance_data}")
                return None
                
        except Exception as e:
            if self.logger:
                self.logger.debug(f"Error extracting balance change: {e}")
            return None
