#!/usr/bin/env python3
"""Validate REVM state changes against Python implementation for 1K transactions."""

import subprocess
import json
import requests
import time
import os
import sys
from typing import Dict, List, Tuple, Optional
from collections import defaultdict

class StateChangeValidator:
    def __init__(self):
        self.rpc_url = "http://127.0.0.1:8545"
        self.revm_cmd = ["cargo", "run", "--example", "json_state_validator_no_rpc", "--"]
        self.python_dir = "/home/nima/code/crypto/py/eth_tx_manager"
        self.threshold = 0.0005  # ETH threshold for comparison
        
    def get_recent_transactions(self, count: int = 1000) -> List[str]:
        """Get recent transactions from the blockchain."""
        print(f"Fetching {count} recent transactions...")
        
        # Get latest block
        payload = {
            'jsonrpc': '2.0',
            'method': 'eth_blockNumber',
            'params': [],
            'id': 1
        }
        response = requests.post(self.rpc_url, json=payload)
        latest_block = int(response.json()['result'], 16)
        
        transactions = []
        blocks_checked = 0
        
        # Search back through blocks for transactions
        for block_num in range(latest_block, latest_block - 500, -1):
            if len(transactions) >= count:
                break
                
            blocks_checked += 1
            if blocks_checked % 10 == 0:
                print(f"\rChecking block {block_num}, found {len(transactions)} transactions...", end='', flush=True)
            
            # Get block
            payload = {
                'jsonrpc': '2.0',
                'method': 'eth_getBlockByNumber',
                'params': [hex(block_num), True],
                'id': 1
            }
            response = requests.post(self.rpc_url, json=payload)
            block = response.json().get('result')
            
            if not block or not block.get('transactions'):
                continue
                
            # Add transactions from this block
            for tx in block['transactions']:
                if len(transactions) >= count:
                    break
                    
                # Skip simple transfers and failed transactions
                if (tx.get('to') and 
                    int(tx.get('gas', '0x0'), 16) > 21000):  # More than simple transfer
                    transactions.append(tx['hash'])
        
        print(f"\nFound {len(transactions)} transactions in {blocks_checked} blocks")
        return transactions[:count]
    
    def run_revm_simulation(self, tx_hash: str) -> Optional[Dict]:
        """Run REVM simulation for a transaction."""
        try:
            result = subprocess.run(
                self.revm_cmd + [tx_hash],
                capture_output=True,
                text=True,
                timeout=30,
                cwd="/home/nima/code/crypto/rust/revm_tx_simulator"
            )
            
            if result.returncode == 0 and result.stdout.strip():
                return json.loads(result.stdout)
            else:
                return None
        except Exception:
            return None
    
    def run_python_simulation(self, tx_hash: str) -> Optional[Dict]:
        """Run Python simulation for a transaction."""
        python_cmd = [
            "python", "-m", "eth_tx_manager.simulation.eth_tx_simulation", 
            "--tx_hash", tx_hash,
            "--eth_rpc_url", self.rpc_url
        ]
        
        try:
            result = subprocess.run(
                python_cmd,
                capture_output=True,
                text=True,
                timeout=30,
                cwd=self.python_dir
            )
            
            if result.returncode == 0 and result.stdout.strip():
                return json.loads(result.stdout)
            else:
                return None
        except Exception:
            return None
    
    def normalize_address(self, addr: str) -> str:
        """Normalize address to lowercase without 0x prefix."""
        return addr.lower().replace('0x', '')
    
    def compare_state_changes(self, revm_data: Dict, python_data: Dict) -> Dict:
        """Compare state changes between REVM and Python."""
        comparison = {
            'match': True,
            'differences': [],
            'revm_only': [],
            'python_only': [],
            'eth_differences': {},
            'token_differences': {}
        }
        
        # Normalize addresses
        revm_addrs = {self.normalize_address(addr): data for addr, data in revm_data.items()}
        python_addrs = {self.normalize_address(addr): data for addr, data in python_data.items()}
        
        # Find addresses only in one implementation
        revm_only = set(revm_addrs.keys()) - set(python_addrs.keys())
        python_only = set(python_addrs.keys()) - set(revm_addrs.keys())
        
        if revm_only:
            comparison['revm_only'] = list(revm_only)
        if python_only:
            comparison['python_only'] = list(python_only)
        
        # Compare common addresses
        for addr in set(revm_addrs.keys()) & set(python_addrs.keys()):
            revm_changes = revm_addrs[addr]
            python_changes = python_addrs[addr]
            
            # Compare ETH changes
            revm_eth = float(revm_changes.get('eth_net', 0))
            python_eth = float(python_changes.get('eth_net', 0))
            
            if abs(revm_eth - python_eth) > self.threshold:
                comparison['match'] = False
                comparison['eth_differences'][addr] = {
                    'revm': revm_eth,
                    'python': python_eth,
                    'diff': revm_eth - python_eth
                }
            
            # Compare token changes
            revm_tokens = revm_changes.get('token_net', {})
            python_tokens = python_changes.get('token_net', {})
            
            # Check tokens in both
            all_tokens = set(revm_tokens.keys()) | set(python_tokens.keys())
            for token in all_tokens:
                revm_amount = float(revm_tokens.get(token, 0))
                python_amount = float(python_tokens.get(token, 0))
                
                if abs(revm_amount - python_amount) > 0.01:  # 0.01 token threshold
                    comparison['match'] = False
                    if addr not in comparison['token_differences']:
                        comparison['token_differences'][addr] = {}
                    comparison['token_differences'][addr][token] = {
                        'revm': revm_amount,
                        'python': python_amount,
                        'diff': revm_amount - python_amount
                    }
        
        return comparison
    
    def run_validation(self, num_transactions: int = 1000) -> Dict:
        """Run validation comparing REVM and Python implementations."""
        print(f"🚀 Starting state change validation for {num_transactions} transactions")
        print("=" * 80)
        
        # Get transactions
        transactions = self.get_recent_transactions(num_transactions)
        
        results = {
            'timestamp': time.time(),
            'total_transactions': len(transactions),
            'both_successful': 0,
            'revm_only_successful': 0,
            'python_only_successful': 0,
            'both_failed': 0,
            'matching_results': 0,
            'mismatched_results': 0,
            'detailed_mismatches': [],
            'summary': {}
        }
        
        # Test each transaction
        for i, tx_hash in enumerate(transactions):
            if i % 50 == 0:
                print(f"\n[{i}/{len(transactions)}] Progress: {i/len(transactions)*100:.1f}%")
            
            # Run both simulations
            revm_result = self.run_revm_simulation(tx_hash)
            python_result = self.run_python_simulation(tx_hash)
            
            # Categorize results
            if revm_result and python_result:
                results['both_successful'] += 1
                
                # Compare state changes
                comparison = self.compare_state_changes(revm_result, python_result)
                
                if comparison['match']:
                    results['matching_results'] += 1
                else:
                    results['mismatched_results'] += 1
                    
                    # Store detailed mismatch info (limit to first 10)
                    if len(results['detailed_mismatches']) < 10:
                        results['detailed_mismatches'].append({
                            'tx_hash': tx_hash,
                            'comparison': comparison
                        })
                
            elif revm_result and not python_result:
                results['revm_only_successful'] += 1
            elif python_result and not revm_result:
                results['python_only_successful'] += 1
            else:
                results['both_failed'] += 1
            
            # Show progress
            if (i + 1) % 100 == 0:
                print(f"\n  Processed: {i+1}")
                print(f"  Both successful: {results['both_successful']}")
                print(f"  Matching: {results['matching_results']}")
                print(f"  Mismatched: {results['mismatched_results']}")
        
        # Generate summary
        results['summary'] = {
            'success_rate_revm': (results['both_successful'] + results['revm_only_successful']) / results['total_transactions'],
            'success_rate_python': (results['both_successful'] + results['python_only_successful']) / results['total_transactions'],
            'match_rate': results['matching_results'] / max(results['both_successful'], 1),
            'common_mismatches': self.analyze_mismatches(results['detailed_mismatches'])
        }
        
        return results
    
    def analyze_mismatches(self, mismatches: List[Dict]) -> Dict:
        """Analyze common patterns in mismatches."""
        patterns = {
            'eth_precision_issues': 0,
            'missing_addresses_revm': 0,
            'missing_addresses_python': 0,
            'token_differences': 0,
            'gas_fee_differences': 0
        }
        
        for mismatch in mismatches:
            comp = mismatch['comparison']
            
            if comp.get('eth_differences'):
                # Check if differences are small (likely precision)
                small_diffs = all(abs(d['diff']) < 0.001 for d in comp['eth_differences'].values())
                if small_diffs:
                    patterns['eth_precision_issues'] += 1
                else:
                    patterns['gas_fee_differences'] += 1
            
            if comp.get('revm_only'):
                patterns['missing_addresses_python'] += len(comp['revm_only'])
            
            if comp.get('python_only'):
                patterns['missing_addresses_revm'] += len(comp['python_only'])
            
            if comp.get('token_differences'):
                patterns['token_differences'] += 1
        
        return patterns
    
    def print_summary(self, results: Dict):
        """Print validation summary."""
        print("\n" + "=" * 80)
        print("STATE CHANGE VALIDATION SUMMARY")
        print("=" * 80)
        
        print(f"\n📊 Transactions Tested: {results['total_transactions']}")
        print(f"✅ Both Successful: {results['both_successful']}")
        print(f"🟦 REVM Only Success: {results['revm_only_successful']}")
        print(f"🟨 Python Only Success: {results['python_only_successful']}")
        print(f"❌ Both Failed: {results['both_failed']}")
        
        print(f"\n🎯 Matching Results: {results['matching_results']}")
        print(f"⚠️  Mismatched Results: {results['mismatched_results']}")
        print(f"📈 Match Rate: {results['summary']['match_rate']*100:.1f}%")
        
        print(f"\n📋 Success Rates:")
        print(f"  REVM: {results['summary']['success_rate_revm']*100:.1f}%")
        print(f"  Python: {results['summary']['success_rate_python']*100:.1f}%")
        
        if results['detailed_mismatches']:
            print(f"\n🔍 Common Mismatch Patterns:")
            patterns = results['summary']['common_mismatches']
            for pattern, count in patterns.items():
                if count > 0:
                    print(f"  - {pattern}: {count}")
            
            print(f"\n📝 First Few Mismatches:")
            for i, mismatch in enumerate(results['detailed_mismatches'][:3]):
                print(f"\n  [{i+1}] Transaction: {mismatch['tx_hash']}")
                comp = mismatch['comparison']
                if comp.get('eth_differences'):
                    print(f"      ETH differences in {len(comp['eth_differences'])} addresses")
                if comp.get('revm_only'):
                    print(f"      REVM has {len(comp['revm_only'])} extra addresses")
                if comp.get('python_only'):
                    print(f"      Python has {len(comp['python_only'])} extra addresses")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="Validate REVM vs Python state changes")
    parser.add_argument("--transactions", "-n", type=int, default=1000, 
                       help="Number of transactions to test")
    parser.add_argument("--save", "-s", action="store_true",
                       help="Save detailed results to file")
    
    args = parser.parse_args()
    
    validator = StateChangeValidator()
    results = validator.run_validation(args.transactions)
    validator.print_summary(results)
    
    if args.save:
        filename = f"state_validation_results_{int(time.time())}.json"
        with open(filename, "w") as f:
            json.dump(results, f, indent=2, default=str)
        print(f"\n💾 Detailed results saved to: {filename}")