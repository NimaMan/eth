#!/usr/bin/env python3
"""
State Changes Validation Script

ALGORITHMIC DESCRIPTION:
This script processes Ethereum transactions using your existing Python pipeline to:
1. Fetch transaction data (transaction, receipt, trace) from local reth node
2. Process transactions using TransactionProcessor with state_changes=True
3. Extract comprehensive state changes for each address
4. Output results in JSON format compatible with Rust validation
5. Provide detailed analysis of ETH and token movements

This serves the main objective by:
- Validating our Rust implementation against proven Python code
- Providing reference data for comprehensive state change calculation
- Enabling cross-language compatibility testing
- Ensuring accuracy of our state diff algorithms
"""

import sys
import os
import json
import argparse
from typing import Dict, Any, Optional

# Add the Python eth_block_processor to path
sys.path.append('/home/nima/code/crypto/py/eth_block_processor')

try:
    from web3 import Web3
    from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
    from eth_block_processor.txn.txn_processor import TransactionProcessor
    # Optional imports - only import if available
    try:
        from eth_block_processor.txn.txn_log_processor import TransactionLogProcessor
    except ImportError:
        TransactionLogProcessor = None
    
    try:
        from eth_block_processor.txn.txn_batch_processor import TransactionBatchProcessor
    except ImportError:
        TransactionBatchProcessor = None
        
except ImportError as e:
    print(f"❌ Failed to import required modules: {e}")
    print("Make sure you're running from the correct environment with all dependencies installed")
    print("Try: conda activate qw")
    sys.exit(1)

class StateChangesValidator:
    def __init__(self, rpc_url: str = "http://localhost:8545"):
        """Initialize the validator with Web3 connection"""
        self.w3 = Web3(Web3.HTTPProvider(rpc_url))
        
        if not self.w3.is_connected():
            raise ConnectionError(f"Failed to connect to Ethereum node at {rpc_url}")
        
        # Initialize processors
        self.txn_data_fetcher = TransactionDataFetcher(self.w3)
        self.txn_processor = TransactionProcessor(self.w3, calculate_state_changes=True)
        
        print(f"✅ Connected to Ethereum node: {rpc_url}")
        print(f"📊 Latest block: {self.w3.eth.block_number}")

    def process_transaction(self, tx_hash: str, verbose: bool = False) -> Optional[Dict[str, Any]]:
        """
        Process a single transaction and extract state changes
        
        Args:
            tx_hash: Transaction hash to process
            verbose: Enable verbose output
            
        Returns:
            Dictionary containing processed transaction data with state changes
        """
        try:
            print(f"🔍 Fetching transaction data for: {tx_hash}")
            
            # Fetch transaction data
            txn_data = self.txn_data_fetcher.get_transaction_data(tx_hash)
            if not txn_data:
                print(f"❌ Transaction {tx_hash} not found")
                return None
            
            print(f"📦 Transaction found in block: {txn_data['transaction']['blockNumber']}")
            
            # Process transaction with state changes
            print("🧮 Processing transaction with state change calculation...")
            processed_tx = self.txn_processor.process_transaction(
                txn_data['transaction'], 
                txn_data['receipt'], 
                txn_data['trace']
            )
            
            if verbose:
                self.print_transaction_details(processed_tx)
            
            return processed_tx
            
        except Exception as e:
            print(f"❌ Error processing transaction {tx_hash}: {e}")
            if verbose:
                import traceback
                traceback.print_exc()
            return None

    async def process_transaction_async(self, tx_hash: str, verbose: bool = False) -> Optional[Dict[str, Any]]:
        """
        Process a single transaction asynchronously
        
        Args:
            tx_hash: Transaction hash to process
            verbose: Enable verbose output
            
        Returns:
            Dictionary containing processed transaction data with state changes
        """
        try:
            print(f"🔍 Fetching transaction data for: {tx_hash}")
            
            # Fetch transaction data
            txn_data = self.txn_data_fetcher.get_transaction_data(tx_hash)
            if not txn_data:
                print(f"❌ Transaction {tx_hash} not found")
                return None
            
            print(f"📦 Transaction found in block: {txn_data['transaction']['blockNumber']}")
            
            # Process transaction with state changes asynchronously
            print("🧮 Processing transaction with state change calculation (async)...")
            processed_tx = await self.txn_processor.process_transaction_async(
                txn_data['transaction'], 
                txn_data['receipt'], 
                txn_data['trace'],
                state_diff=True
            )
            
            if verbose:
                self.print_transaction_details(processed_tx)
            
            return processed_tx
            
        except Exception as e:
            print(f"❌ Error processing transaction {tx_hash}: {e}")
            if verbose:
                import traceback
                traceback.print_exc()
            return None

    def print_transaction_details(self, processed_tx):
        """Print detailed transaction information"""
        print("\n📊 TRANSACTION DETAILS:")
        print("=" * 50)
        print(f"Hash: {getattr(processed_tx, 'hash', 'N/A')}")
        print(f"Block: {getattr(processed_tx, 'block_number', 'N/A')}")
        print(f"From: {getattr(processed_tx, 'from_address', 'N/A')}")
        print(f"To: {getattr(processed_tx, 'to_address', 'N/A')}")
        print(f"Value: {getattr(processed_tx, 'value', 0)} ETH")
        print(f"Status: {'✅ Success' if getattr(processed_tx, 'status', False) else '❌ Failed'}")
        print(f"Transaction Type: {getattr(processed_tx, 'txn_type', 'N/A')}")
        print(f"Actions: {getattr(processed_tx, 'actions', 'N/A')}")
        
        # Print fees
        fees = getattr(processed_tx, 'fees', {})
        if hasattr(fees, 'get'):
            print(f"Gas Used: {fees.get('gas_used', 'N/A')}")
            print(f"Gas Price: {fees.get('gas_price', 'N/A')}")
            print(f"Transaction Fee: {fees.get('txn_fee', 'N/A')} ETH")
        else:
            print(f"Fees: {fees}")
        
        # Print unique addresses
        unique_addresses = getattr(processed_tx, 'unique_addresses', set())
        print(f"Unique Addresses: {len(unique_addresses)}")
        
        # Print transfers
        eth_transfers = getattr(processed_tx, 'eth_transfers', [])
        erc20_transfers = getattr(processed_tx, 'erc20_transfers', [])
        print(f"ETH Transfers: {len(eth_transfers)}")
        print(f"ERC20 Transfers: {len(erc20_transfers)}")

    def print_state_changes(self, state_changes):
        """Print state changes in a readable format"""
        if not state_changes:
            print("📊 No state changes detected")
            return
        
        print(f"\n📊 STATE CHANGES ({len(state_changes)} addresses):")
        print("=" * 60)
        print("ℹ️  Note: Python implementation filters using thresholds:")
        print("   - denom_state_change_threshold: 0.0005 (ETH/WETH)")
        print("   - token_state_change_threshold: 0.1 (ERC20 tokens)")
        print("   - Transaction sender is always included")
        
        for address, changes in state_changes.items():
            print(f"\n🏠 Address: {address}")
            if hasattr(changes, 'get'):
                print(f"   Token Net: {changes.get('token_net', 0):.6f}")
                print(f"   Denom Net: {changes.get('denom_net', 0):.6f}")
                
                # Print movements if available
                movements = changes.get('movements', {})
            else:
                print(f"   Token Net: {getattr(changes, 'token_net', 0):.6f}")
                print(f"   Denom Net: {getattr(changes, 'denom_net', 0):.6f}")
                
                # Print movements if available
                movements = getattr(changes, 'movements', {})
            
            if movements:
                if hasattr(movements, 'get'):
                    token_movements = movements.get('token', {})
                    denom_movements = movements.get('denom', {})
                else:
                    token_movements = getattr(movements, 'token', {})
                    denom_movements = getattr(movements, 'denom', {})
                
                if token_movements:
                    if hasattr(token_movements, 'get'):
                        token_in = token_movements.get('in', {})
                        token_out = token_movements.get('out', {})
                    else:
                        token_in = getattr(token_movements, 'incoming', {})
                        token_out = getattr(token_movements, 'outgoing', {})
                    if token_in or token_out:
                        print(f"   Token Movements: In={len(token_in)}, Out={len(token_out)}")
                
                if denom_movements:
                    if hasattr(denom_movements, 'get'):
                        denom_in = denom_movements.get('in', {})
                        denom_out = denom_movements.get('out', {})
                    else:
                        denom_in = getattr(denom_movements, 'incoming', {})
                        denom_out = getattr(denom_movements, 'outgoing', {})
                    if denom_in or denom_out:
                        print(f"   Denom Movements: In={len(denom_in)}, Out={len(denom_out)}")

    def convert_movements_for_json(self, movements):
        """Convert movements data to JSON-serializable format"""
        if not movements:
            return {}
        
        json_movements = {}
        for movement_type, movement_data in movements.items():
            if hasattr(movement_data, 'items'):
                json_movements[movement_type] = {}
                for direction, direction_data in movement_data.items():
                    if hasattr(direction_data, 'items'):
                        # Convert tuple keys to strings
                        json_movements[movement_type][direction] = {
                            str(k): v for k, v in direction_data.items()
                        }
                    else:
                        json_movements[movement_type][direction] = direction_data
            else:
                json_movements[movement_type] = movement_data
        
        return json_movements

    def export_for_rust_validation(self, processed_tx, output_file: Optional[str] = None) -> str:
        """
        Export state changes in format compatible with Rust validation
        
        Args:
            processed_tx: Processed transaction data
            output_file: Optional output file path
            
        Returns:
            JSON string of state changes
        """
        # Get state changes from the processed transaction object
        if hasattr(processed_tx, 'state_changes'):
            state_changes = processed_tx.state_changes
        elif hasattr(processed_tx, 'get'):
            state_changes = processed_tx.get('state_changes', {})
        else:
            state_changes = getattr(processed_tx, 'state_changes', {})
        
        # Convert to format expected by Rust test
        rust_compatible = {}
        for address, changes in state_changes.items():
            if hasattr(changes, 'get'):
                movements = self.convert_movements_for_json(changes.get('movements', {}))
                rust_compatible[address] = {
                    'token_net': float(changes.get('token_net', 0)),
                    'denom_net': float(changes.get('denom_net', 0)),
                    'movements': movements
                }
            else:
                movements = self.convert_movements_for_json(getattr(changes, 'movements', {}))
                rust_compatible[address] = {
                    'token_net': float(getattr(changes, 'token_net', 0)),
                    'denom_net': float(getattr(changes, 'denom_net', 0)),
                    'movements': movements
                }
        
        json_output = json.dumps(rust_compatible, indent=2, default=str)
        
        if output_file:
            with open(output_file, 'w') as f:
                f.write(json_output)
            print(f"📝 Exported state changes to: {output_file}")
        
        return json_output

def main():
    parser = argparse.ArgumentParser(description='Validate state changes using Python transaction processor')
    parser.add_argument('tx_hash', help='Transaction hash to process')
    parser.add_argument('--rpc-url', default='http://localhost:8545', help='Ethereum RPC URL')
    parser.add_argument('--verbose', '-v', action='store_true', help='Enable verbose output')
    parser.add_argument('--output', '-o', help='Output file for JSON results')
    parser.add_argument('--rust-format', action='store_true', help='Output in Rust-compatible format')
    parser.add_argument('--use-async', action='store_true', help='Use async processing')
    
    args = parser.parse_args()
    
    try:
        # Initialize validator
        validator = StateChangesValidator(args.rpc_url)
        
        # Process transaction
        if args.use_async:
            import asyncio
            processed_tx = asyncio.run(validator.process_transaction_async(args.tx_hash, args.verbose))
        else:
            processed_tx = validator.process_transaction(args.tx_hash, args.verbose)
        
        if not processed_tx:
            print("❌ Failed to process transaction")
            sys.exit(1)
        
        # Extract and print state changes
        if hasattr(processed_tx, 'state_changes'):
            state_changes = processed_tx.state_changes
        else:
            state_changes = getattr(processed_tx, 'state_changes', {})
            
        validator.print_state_changes(state_changes)
        
        # Export results
        if args.rust_format or args.output:
            output_file = args.output if args.output else f"state_changes_{args.tx_hash[:10]}.json"
            json_output = validator.export_for_rust_validation(processed_tx, output_file)
            
            if args.rust_format:
                print("\n🔄 RUST-COMPATIBLE JSON OUTPUT:")
                print("=" * 50)
                print(json_output)
        
        print(f"\n✅ Successfully processed transaction: {args.tx_hash}")
        print(f"📊 Found state changes for {len(state_changes)} addresses")
        
    except Exception as e:
        print(f"❌ Error: {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main() 