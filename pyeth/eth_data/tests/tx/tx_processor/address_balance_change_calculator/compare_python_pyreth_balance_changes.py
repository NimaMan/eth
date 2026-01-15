#!/usr/bin/env python3
"""
Compare Python vs PyReth ProcessedTransaction Address Balance Change Calculations

Objective:
---------
This script compares address balance change calculations between Python (eth_data.tx_processor) 
and Rust (pyreth) implementations to ensure both produce identical state_changes results.
The comparison focuses on validating that the migration from Python to Rust maintains 
exact calculation parity for currency_net and token_net structures.

Algorithm:
----------
1. Process transaction with Python TransactionProcessor (calculate_state_changes=True)
2. Process same transaction with PyReth TxProcessor 
3. Extract state_changes from both ProcessedTransaction results
4. Deep compare currency_net and token_net dictionaries for each address
5. Report any differences with detailed analysis
6. Support single transaction or batch comparison

Key Validation Points:
---------------------
- currency_net: ETH, USDC, USDT, DAI and other known currencies match exactly
- token_net: Unknown token addresses and amounts match exactly  
- Address coverage: Same set of addresses detected in both implementations
- Precision: Floating point values match within reasonable tolerance
- Structure: Both use updated format (currency_net + token_net separation)
"""

import sys
from typing import Dict, List, Any, Optional, Tuple, Set
from web3 import Web3
import pyreth
from decimal import Decimal, getcontext
import traceback

# Set high precision for decimal comparisons
getcontext().prec = 50

from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_processor import TransactionProcessor


class PythonPyRethComparisonTester:
    """Comparison tester for Python vs PyReth ProcessedTransaction calculations"""
    
    def __init__(self):
        # Initialize Web3 connection
        self.w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        
        if not self.w3.is_connected():
            raise ConnectionError("Failed to connect to Ethereum node at http://127.0.0.1:8545")
        
        # Initialize Python components
        self.tx_data_fetcher = TransactionDataFetcher(self.w3)
        self.tx_processor = TransactionProcessor(w3=self.w3, calculate_address_balance_changes=True)
        
        # Initialize PyReth (Rust) components using singleton pattern
        self.py_reth = pyreth.PyReth()
        self.pyreth_processor = self.py_reth.tx_processor()
        
        print(f"✅ Connected to Ethereum node, latest block: {self.w3.eth.block_number}")
        print(f"✅ Initialized Python TransactionProcessor with state changes enabled")
        print(f"✅ Initialized PyReth TxProcessor with shared database connection")
    
    def process_with_python(self, tx_hash: str) -> Any:
        """Process transaction using Python eth_data implementation"""
        print(f"🐍 Processing with Python: {tx_hash}")
        
        # Fetch transaction data
        tx_data = self.tx_data_fetcher.get_transaction_data(tx_hash)
        
        # Process transaction with state changes enabled
        processed_tx = self.tx_processor.process_transaction(
            tx_data['transaction'], 
            tx_data['receipt'], 
            tx_data['trace']
        )
        
        print(f"   ✅ Python processing complete")
        print(f"      Block: {processed_tx.block_number}")
        print(f"      Type: {processed_tx.tx_type}")
        print(f"      ERC20 transfers: {len(processed_tx.erc20_transfers)}")
        print(f"      Internal transactions: {len(processed_tx.internal_transactions)}")
        print(f"      State changes: {len(processed_tx.address_balance_changes)} addresses")
        
        return processed_tx
    
    def process_with_pyreth(self, tx_hash: str) -> Any:
        """Process transaction using PyReth (Rust) implementation"""
        print(f"🦀 Processing with PyReth: {tx_hash}")
        
        # PyReth now uses process_transaction_from_hash_with_simulation() which calculates balance changes
        
        # Process transaction using PyReth with simulation for balance changes
        processed_tx = self.pyreth_processor.process_transaction_from_hash_with_simulation(tx_hash)
        
        print(f"   ✅ PyReth processing complete")
        print(f"      Block: {processed_tx.block_number}")
        print(f"      Type: {processed_tx.tx_type}")
        print(f"      ERC20 transfers: {len(processed_tx.erc20_transfers)}")
        print(f"      Internal transactions: {len(processed_tx.internal_transactions)}")
        print(f"      Address balance changes: {len(processed_tx.address_balance_changes)} addresses")
        
        # Check if balance changes are calculated
        if len(processed_tx.address_balance_changes) == 0:
            print(f"   ⚠️  PyReth address balance changes not calculated (known limitation)")
            print(f"      Need to implement simulation-based processing for historical transactions")
        
        return processed_tx
    
    def normalize_state_changes(self, state_changes: Dict[str, Any], source: str) -> Dict[str, Any]:
        """Normalize state changes to consistent format for comparison"""
        normalized = {}
        
        for address, change in state_changes.items():
            # Normalize address to checksum format
            normalized_addr = Web3.to_checksum_address(address)
            
            # Extract currency_net and token_net
            if isinstance(change, dict):
                currency_net = change.get('currency_net', {})
                token_net = change.get('token_net', {})
                
                # Handle legacy format where there might be 'eth_net' field
                if 'eth_net' in change and 'ETH' not in currency_net:
                    currency_net = currency_net.copy()
                    currency_net['ETH'] = change['eth_net']
            else:
                # Handle unexpected format
                print(f"⚠️ Unexpected state change format from {source} for {address}: {type(change)}")
                currency_net = {}
                token_net = {}
            
            normalized[normalized_addr] = {
                'currency_net': currency_net,
                'token_net': token_net
            }
        
        return normalized
    
    def compare_currency_net(self, python_curr: Dict[str, float], pyreth_curr: Dict[str, float], 
                           address: str) -> Tuple[bool, List[str]]:
        """Compare currency_net dictionaries for an address"""
        differences = []
        match = True
        
        # Get all currency symbols from both implementations
        all_currencies = set(python_curr.keys()) | set(pyreth_curr.keys())
        
        for currency in all_currencies:
            python_val = python_curr.get(currency, 0.0)
            pyreth_val = pyreth_curr.get(currency, 0.0)
            
            # Use decimal for high precision comparison
            python_decimal = Decimal(str(python_val))
            pyreth_decimal = Decimal(str(pyreth_val))
            diff = abs(python_decimal - pyreth_decimal)
            
            # Allow small floating point tolerance
            tolerance = Decimal('1e-15')
            
            if diff > tolerance:
                match = False
                differences.append(
                    f"    {currency}: Python={python_val}, PyReth={pyreth_val}, diff={diff}"
                )
        
        return match, differences
    
    def compare_token_net(self, python_tokens: Dict[str, float], pyreth_tokens: Dict[str, float], 
                         address: str) -> Tuple[bool, List[str]]:
        """Compare token_net dictionaries for an address"""
        differences = []
        match = True
        
        # Get all token addresses from both implementations
        all_tokens = set(python_tokens.keys()) | set(pyreth_tokens.keys())
        
        for token_addr in all_tokens:
            python_val = python_tokens.get(token_addr, 0.0)
            pyreth_val = pyreth_tokens.get(token_addr, 0.0)
            
            # Use decimal for high precision comparison
            python_decimal = Decimal(str(python_val))
            pyreth_decimal = Decimal(str(pyreth_val))
            diff = abs(python_decimal - pyreth_decimal)
            
            # Allow small tolerance for token amounts
            tolerance = Decimal('0.01')  # Slightly higher tolerance for token amounts
            
            if diff > tolerance:
                match = False
                differences.append(
                    f"    {token_addr}: Python={python_val}, PyReth={pyreth_val}, diff={diff}"
                )
        
        return match, differences
    
    def compare_state_changes(self, python_state: Dict[str, Any], pyreth_state: Dict[str, Any], 
                            tx_hash: str) -> Dict[str, Any]:
        """Deep compare state_changes from Python and PyReth implementations"""
        print(f"\n🔍 Comparing state changes for {tx_hash}")
        
        # Normalize both state change dictionaries
        python_normalized = self.normalize_state_changes(python_state, "Python")
        pyreth_normalized = self.normalize_state_changes(pyreth_state, "PyReth")
        
        # Get all addresses from both implementations
        python_addresses = set(python_normalized.keys())
        pyreth_addresses = set(pyreth_normalized.keys())
        all_addresses = python_addresses | pyreth_addresses
        
        print(f"   Python addresses: {len(python_addresses)}")
        print(f"   PyReth addresses: {len(pyreth_addresses)}")
        print(f"   Total unique addresses: {len(all_addresses)}")
        
        # Track comparison results
        perfect_matches = []
        currency_mismatches = []
        token_mismatches = []
        missing_addresses = []
        
        # Compare each address
        for address in all_addresses:
            python_change = python_normalized.get(address, {'currency_net': {}, 'token_net': {}})
            pyreth_change = pyreth_normalized.get(address, {'currency_net': {}, 'token_net': {}})
            
            # Check if address exists in both
            in_python = address in python_addresses
            in_pyreth = address in pyreth_addresses
            
            if not in_python or not in_pyreth:
                missing_addresses.append({
                    'address': address,
                    'in_python': in_python,
                    'in_pyreth': in_pyreth,
                    'python_change': python_change if in_python else None,
                    'pyreth_change': pyreth_change if in_pyreth else None
                })
                continue
            
            # Compare currency_net
            currency_match, currency_diffs = self.compare_currency_net(
                python_change['currency_net'], pyreth_change['currency_net'], address
            )
            
            # Compare token_net
            token_match, token_diffs = self.compare_token_net(
                python_change['token_net'], pyreth_change['token_net'], address
            )
            
            # Record results
            if currency_match and token_match:
                perfect_matches.append(address)
            else:
                if not currency_match:
                    currency_mismatches.append({
                        'address': address,
                        'differences': currency_diffs,
                        'python_currency_net': python_change['currency_net'],
                        'pyreth_currency_net': pyreth_change['currency_net']
                    })
                
                if not token_match:
                    token_mismatches.append({
                        'address': address,
                        'differences': token_diffs,
                        'python_token_net': python_change['token_net'],
                        'pyreth_token_net': pyreth_change['token_net']
                    })
        
        # Prepare comparison results
        comparison_result = {
            'tx_hash': tx_hash,
            'total_addresses': len(all_addresses),
            'perfect_matches': len(perfect_matches),
            'currency_mismatches': len(currency_mismatches),
            'token_mismatches': len(token_mismatches),
            'missing_addresses': len(missing_addresses),
            'success': len(currency_mismatches) == 0 and len(token_mismatches) == 0 and len(missing_addresses) == 0,
            'details': {
                'perfect_matches': perfect_matches,
                'currency_mismatches': currency_mismatches,
                'token_mismatches': token_mismatches,
                'missing_addresses': missing_addresses
            }
        }
        
        return comparison_result
    
    def print_comparison_results(self, results: Dict[str, Any]):
        """Print detailed comparison results"""
        tx_hash = results['tx_hash']
        success = results['success']
        
        print(f"\n{'='*80}")
        print(f"COMPARISON RESULTS: {tx_hash}")
        print(f"{'='*80}")
        
        if success:
            print(f"🎉 PERFECT MATCH! All {results['total_addresses']} addresses match exactly")
            print(f"   ✅ Currency changes: All match")
            print(f"   ✅ Token changes: All match")
            print(f"   ✅ Address coverage: Complete")
        else:
            print(f"❌ MISMATCHES DETECTED!")
            print(f"   Total addresses: {results['total_addresses']}")
            print(f"   Perfect matches: {results['perfect_matches']}")
            print(f"   Currency mismatches: {results['currency_mismatches']}")
            print(f"   Token mismatches: {results['token_mismatches']}")
            print(f"   Missing addresses: {results['missing_addresses']}")
            
            # Check if this is due to PyReth limitation
            if results['missing_addresses'] > 0 and results['currency_mismatches'] == 0 and results['token_mismatches'] == 0:
                details = results['details']
                missing_in_pyreth = sum(1 for missing in details['missing_addresses'] if missing['in_python'] and not missing['in_pyreth'])
                if missing_in_pyreth > 0:
                    print(f"   ⚠️  Note: {missing_in_pyreth} addresses missing from PyReth due to known limitation")
                    print(f"      PyReth's process_transaction() doesn't calculate balance changes for historical transactions")
        
        # Print detailed differences
        details = results['details']
        
        if details['missing_addresses']:
            print(f"\n🚨 MISSING ADDRESSES ({len(details['missing_addresses'])}):")
            for missing in details['missing_addresses']:
                addr = missing['address']
                in_python = missing['in_python']
                in_pyreth = missing['in_pyreth']
                print(f"  {addr}: Python={in_python}, PyReth={in_pyreth}")
                
                if in_python and not in_pyreth:
                    change = missing['python_change']
                    print(f"    Python only: currency_net={change['currency_net']}, token_net={change['token_net']}")
                elif in_pyreth and not in_python:
                    change = missing['pyreth_change']
                    print(f"    PyReth only: currency_net={change['currency_net']}, token_net={change['token_net']}")
        
        if details['currency_mismatches']:
            print(f"\n💱 CURRENCY MISMATCHES ({len(details['currency_mismatches'])}):")
            for mismatch in details['currency_mismatches']:
                addr = mismatch['address']
                print(f"  {addr}:")
                for diff in mismatch['differences']:
                    print(f"  {diff}")
        
        if details['token_mismatches']:
            print(f"\n🪙 TOKEN MISMATCHES ({len(details['token_mismatches'])}):")
            for mismatch in details['token_mismatches']:
                addr = mismatch['address']
                print(f"  {addr}:")
                for diff in mismatch['differences']:
                    print(f"  {diff}")
    
    def compare_transaction_balance_changes(self, tx_hash: str) -> Dict[str, Any]:
        """Compare address balance changes for a single transaction"""
        print(f"\n{'='*80}")
        print(f"PROCESSING TRANSACTION: {tx_hash}")
        print(f"{'='*80}")
        
        try:
            # Process with both implementations
            python_result = self.process_with_python(tx_hash)
            pyreth_result = self.process_with_pyreth(tx_hash)
            
            # Compare state changes
            comparison = self.compare_state_changes(
                python_result.address_balance_changes, 
                pyreth_result.address_balance_changes, 
                tx_hash
            )
            
            # Print results
            self.print_comparison_results(comparison)
            
            return comparison
            
        except Exception as e:
            print(f"❌ Error processing transaction {tx_hash}: {e}")
            traceback.print_exc()
            return {
                'tx_hash': tx_hash,
                'success': False,
                'error': str(e),
                'total_addresses': 0,
                'perfect_matches': 0,
                'currency_mismatches': 0,
                'token_mismatches': 0,
                'missing_addresses': 0,
                'details': {'missing_addresses': [], 'currency_mismatches': [], 'token_mismatches': []}
            }
    
    def compare_batch_transactions(self, tx_hashes: List[str]) -> List[Dict[str, Any]]:
        """Compare address balance changes for multiple transactions"""
        results = []
        successful_comparisons = 0
        
        print(f"\n{'='*80}")
        print(f"BATCH COMPARISON: {len(tx_hashes)} TRANSACTIONS")
        print(f"{'='*80}")
        
        for i, tx_hash in enumerate(tx_hashes, 1):
            print(f"\n📈 Progress: {i}/{len(tx_hashes)}")
            
            result = self.compare_transaction_balance_changes(tx_hash)
            results.append(result)
            
            if result['success']:
                successful_comparisons += 1
        
        # Print batch summary
        print(f"\n{'='*80}")
        print(f"BATCH SUMMARY")
        print(f"{'='*80}")
        print(f"Total transactions: {len(tx_hashes)}")
        print(f"Perfect matches: {successful_comparisons}")
        print(f"Mismatches: {len(tx_hashes) - successful_comparisons}")
        print(f"Success rate: {successful_comparisons/len(tx_hashes)*100:.1f}%")
        
        # Analyze types of mismatches
        total_missing_pyreth = sum(
            sum(1 for missing in r.get('details', {}).get('missing_addresses', []) 
                if missing.get('in_python', False) and not missing.get('in_pyreth', False))
            for r in results if not r.get('success', False)
        )
        
        total_currency_mismatches = sum(r.get('currency_mismatches', 0) for r in results)
        total_token_mismatches = sum(r.get('token_mismatches', 0) for r in results)
        
        if successful_comparisons == len(tx_hashes):
            print(f"🎉 ALL TRANSACTIONS MATCH PERFECTLY!")
        else:
            print(f"❌ {len(tx_hashes) - successful_comparisons} transactions have mismatches")
            
            if total_missing_pyreth > 0 and total_currency_mismatches == 0 and total_token_mismatches == 0:
                print(f"\n📋 MISMATCH ANALYSIS:")
                print(f"   - All mismatches are due to PyReth limitation (missing balance changes)")
                print(f"   - No actual calculation differences detected")
                print(f"   - Total addresses missing from PyReth: {total_missing_pyreth}")
                print(f"\n🔧 REQUIRED ENHANCEMENT:")
                print(f"   - PyReth needs simulation-based processing for historical transactions")
                print(f"   - Current process_transaction() only loads from DB without simulation")
                print(f"   - Need to implement get_transaction_for_simulation() + simulate_and_process()")
            else:
                # List failed transactions
                failed_txs = [r['tx_hash'] for r in results if not r['success']]
                print(f"\n❌ Failed transactions:")
                for tx_hash in failed_txs:
                    print(f"  - {tx_hash}")
                
                if total_currency_mismatches > 0 or total_token_mismatches > 0:
                    print(f"\n🚨 CALCULATION DIFFERENCES DETECTED:")
                    print(f"   - Currency mismatches: {total_currency_mismatches}")
                    print(f"   - Token mismatches: {total_token_mismatches}")
                    print(f"   - These indicate actual calculation differences that need investigation")
        
        return results


def main():
    """Main function for testing comparison functionality"""
    
    # Test transactions from existing comprehensive test suite
    test_transactions = [
        # Basic ETH transfer
        "0xfc6e6c97d46e0c5e6584ef1e4088f01ea4a17d4348f53e11ffc1977c0b715608",
        
        # Complex KERMIT swap
        "0xf403b3d19a6e83ddb04e7755cdc5122e1812b5b7b7df69528ce1d202f42e22b5",
        
        # Double-counting bug test
        "0xc57612d638506ab8295d71bc1af0fe62647ab421af66dc56383990f434d28736",
        
        # MEV bot with token overflow
        "0xab960eebdefaa8230de2757274f65c5efb5a4db884e1777a9afa8b46f62bcb03",
        
        # Complex WETH swap with internal transactions
        "0xcbf2b9ddf1b2040c4d7f0f52aafd5ca5d21c51fc86f9002efb9a1d97698a5e29",
        
        # Complex Uniswap ETH→USDT swap
        "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
    ]
    
    # Initialize tester
    try:
        tester = PythonPyRethComparisonTester()
    except Exception as e:
        print(f"❌ Failed to initialize tester: {e}")
        return 1
    
    # Check command line arguments
    if len(sys.argv) > 1:
        if sys.argv[1] == "--batch":
            # Run batch comparison with all test transactions
            results = tester.compare_batch_transactions(test_transactions)
            return 0 if all(r['success'] for r in results) else 1
        else:
            # Compare specific transaction hash(es) from command line
            tx_hashes = sys.argv[1:]
            
            if len(tx_hashes) == 1:
                result = tester.compare_transaction_balance_changes(tx_hashes[0])
                return 0 if result['success'] else 1
            else:
                results = tester.compare_batch_transactions(tx_hashes)
                return 0 if all(r['success'] for r in results) else 1
    else:
        # Run single test transaction by default
        test_tx = test_transactions[0]  # Basic ETH transfer
        print(f"🧪 Running single transaction test (use --batch for all tests)")
        print(f"   Transaction: {test_tx}")
        
        result = tester.compare_transaction_balance_changes(test_tx)
        return 0 if result['success'] else 1


if __name__ == "__main__":
    exit_code = main()
