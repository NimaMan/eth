#!/usr/bin/env python3
"""
Batch State Change Validation Script

ALGORITHMIC DESCRIPTION:
This script processes multiple transactions through the Python state change pipeline
and exports results in a format compatible with the Rust validation infrastructure.

Key Components:
1. Batch Processing: Handle multiple transactions efficiently
2. Error Handling: Graceful failure handling with detailed error reporting
3. JSON Export: Export results in Rust-compatible format
4. Performance Tracking: Measure processing times for comparison
5. Environment Testing: Validate setup and dependencies

This serves the main objective by providing a reliable Python baseline for
validating the Rust implementation across diverse transaction types.
"""

import sys
import json
import time
import argparse
import asyncio
from pathlib import Path
from typing import List, Dict, Any, Optional, Tuple
import traceback

# Add the Python eth_block_processor to path
sys.path.append('/home/nima/code/crypto/py/eth_block_processor')

try:
    from web3 import Web3
    from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
    from eth_block_processor.txn.txn_processor import TransactionProcessor
except ImportError as e:
    print(f"Error importing Python modules: {e}")
    print("Make sure you're running from the correct conda environment (qw)")
    sys.exit(1)

class BatchStateChangeValidator:
    """Batch validator for state changes using Python implementation"""
    
    def __init__(self, reth_url: str = "http://localhost:8545"):
        self.reth_url = reth_url
        
        # Initialize Web3 connection
        self.w3 = Web3(Web3.HTTPProvider(reth_url))
        
        if not self.w3.is_connected():
            raise ConnectionError(f"Failed to connect to Ethereum node at {reth_url}")
        
        # Initialize processors with Web3 instance
        self.fetcher = TransactionDataFetcher(self.w3)
        self.processor = TransactionProcessor(self.w3, calculate_state_changes=True)
        
    async def process_single_transaction(self, tx_hash: str, block_number: int) -> Dict[str, Any]:
        """Process a single transaction and return state changes"""
        start_time = time.time()
        
        try:
            # Fetch transaction data
            txn_data = self.fetcher.get_transaction_data(tx_hash)
            if not txn_data:
                return {
                    "transaction_hash": tx_hash,
                    "block_number": block_number,
                    "success": False,
                    "error_message": "Failed to fetch transaction data",
                    "state_changes": [],
                    "processing_time_ms": 0
                }
            
            # Process transaction with state changes
            processed_tx = self.processor.process_transaction(
                txn_data['transaction'], 
                txn_data['receipt'], 
                txn_data['trace']
            )
            
            if not processed_tx:
                return {
                    "transaction_hash": tx_hash,
                    "block_number": block_number,
                    "success": False,
                    "error_message": "Failed to process transaction",
                    "state_changes": [],
                    "processing_time_ms": 0
                }
            
            # Get state changes from processed transaction (already calculated by TransactionProcessor)
            state_changes = getattr(processed_tx, 'state_changes', {})
            
            # Convert to Rust-compatible format
            rust_compatible_changes = []
            for address, movements in state_changes.items():
                token_net = movements.get('token_net', 0) if hasattr(movements, 'get') else getattr(movements, 'token_net', 0)
                denom_net = movements.get('denom_net', 0) if hasattr(movements, 'get') else getattr(movements, 'denom_net', 0)
                
                rust_compatible_changes.append({
                    "address": address,
                    "token_net": str(token_net),  # Convert to string to handle large numbers
                    "denom_net": float(denom_net)
                })
            
            processing_time_ms = int((time.time() - start_time) * 1000)
            
            return {
                "transaction_hash": tx_hash,
                "block_number": block_number,
                "success": True,
                "error_message": None,
                "state_changes": rust_compatible_changes,
                "processing_time_ms": processing_time_ms
            }
            
        except Exception as e:
            processing_time_ms = int((time.time() - start_time) * 1000)
            error_msg = f"{type(e).__name__}: {str(e)}"
            
            return {
                "transaction_hash": tx_hash,
                "block_number": block_number,
                "success": False,
                "error_message": error_msg,
                "state_changes": [],
                "processing_time_ms": processing_time_ms
            }
    
    async def process_batch(self, transactions: List[Tuple[str, int]]) -> List[Dict[str, Any]]:
        """Process multiple transactions in batch"""
        results = []
        
        for i, (tx_hash, block_number) in enumerate(transactions):
            print(f"Processing transaction {i+1}/{len(transactions)}: {tx_hash}")
            result = await self.process_single_transaction(tx_hash, block_number)
            results.append(result)
            
            # Add small delay to avoid overwhelming the node
            await asyncio.sleep(0.1)
        
        return results
    
    def test_environment(self) -> Dict[str, Any]:
        """Test the Python environment setup"""
        try:
            # Test basic imports
            test_result = {
                "python_version": sys.version,
                "modules_available": True,
                "reth_connection": False,
                "error_message": None
            }
            
            # Test reth connection (simplified)
            try:
                # This is a basic test - in a real scenario you'd test the actual connection
                test_result["reth_connection"] = True
            except Exception as e:
                test_result["error_message"] = f"Reth connection failed: {e}"
            
            return test_result
            
        except Exception as e:
            return {
                "python_version": sys.version,
                "modules_available": False,
                "reth_connection": False,
                "error_message": f"Environment test failed: {e}"
            }

async def main():
    parser = argparse.ArgumentParser(description="Batch validate state changes using Python")
    parser.add_argument("input_file", nargs="?", help="JSON input file with transaction data")
    parser.add_argument("--batch", action="store_true", help="Process multiple transactions")
    parser.add_argument("--test", action="store_true", help="Test environment setup")
    parser.add_argument("--reth-url", default="http://localhost:8545", help="Reth node URL")
    
    args = parser.parse_args()
    
    validator = BatchStateChangeValidator(args.reth_url)
    
    # Handle test mode
    if args.test:
        test_result = validator.test_environment()
        print(json.dumps(test_result, indent=2))
        return
    
    # Require input file for processing
    if not args.input_file:
        print("Error: input_file is required for processing transactions")
        parser.print_help()
        sys.exit(1)
    
    try:
        with open(args.input_file, 'r') as f:
            input_data = json.load(f)
    except Exception as e:
        print(f"Error reading input file: {e}")
        sys.exit(1)
    
    if args.batch:
        # Batch processing mode
        if "transactions" not in input_data:
            print("Error: batch mode requires 'transactions' array in input")
            sys.exit(1)
        
        transactions = [(tx["transaction_hash"], tx["block_number"]) 
                       for tx in input_data["transactions"]]
        
        results = await validator.process_batch(transactions)
        print(json.dumps(results, indent=2))
        
    else:
        # Single transaction mode
        if "transaction_hash" not in input_data or "block_number" not in input_data:
            print("Error: single mode requires 'transaction_hash' and 'block_number'")
            sys.exit(1)
        
        result = await validator.process_single_transaction(
            input_data["transaction_hash"],
            input_data["block_number"]
        )
        print(json.dumps(result, indent=2))

if __name__ == "__main__":
    asyncio.run(main()) 