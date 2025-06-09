#!/usr/bin/env python3
"""Capture detailed failure information from REVM simulations."""

import subprocess
import json
import requests
import time
from typing import Dict, List, Optional, Tuple

class FailureCollector:
    def __init__(self):
        self.rpc_url = "http://127.0.0.1:8545"
        self.revm_cmd = ["cargo", "run", "--example", "json_state_validator_no_rpc", "--"]
        
    def get_transaction_details(self, tx_hash: str) -> Dict:
        """Get detailed transaction information including receipt."""
        # Get transaction
        tx_payload = {
            'jsonrpc': '2.0',
            'method': 'eth_getTransactionByHash',
            'params': [tx_hash],
            'id': 1
        }
        tx_response = requests.post(self.rpc_url, json=tx_payload)
        tx = tx_response.json().get('result')
        
        # Get receipt
        receipt_payload = {
            'jsonrpc': '2.0',
            'method': 'eth_getTransactionReceipt',
            'params': [tx_hash],
            'id': 1
        }
        receipt_response = requests.post(self.rpc_url, json=receipt_payload)
        receipt = receipt_response.json().get('result')
        
        if not tx or not receipt:
            return None
            
        return {
            'hash': tx_hash,
            'block_number': int(tx['blockNumber'], 16) if tx.get('blockNumber') else None,
            'from': tx.get('from'),
            'to': tx.get('to'),
            'value': int(tx['value'], 16) if tx.get('value') else 0,
            'gas_limit': int(tx['gas'], 16) if tx.get('gas') else 0,
            'gas_price': int(tx['gasPrice'], 16) if tx.get('gasPrice') else 0,
            'input_size': len(tx.get('input', '0x')) // 2 - 1,
            'nonce': int(tx['nonce'], 16) if tx.get('nonce') else 0,
            # Receipt details
            'status': receipt.get('status'),  # '0x1' = success, '0x0' = failed
            'gas_used': int(receipt['gasUsed'], 16) if receipt.get('gasUsed') else 0,
            'effective_gas_price': int(receipt['effectiveGasPrice'], 16) if receipt.get('effectiveGasPrice') else 0,
            'tx_type': receipt.get('type', '0x0'),
            'logs_count': len(receipt.get('logs', [])),
        }
    
    def run_simulation(self, tx_hash: str) -> Tuple[bool, Optional[str], Optional[str]]:
        """Run simulation and capture detailed error information."""
        try:
            result = subprocess.run(
                self.revm_cmd + [tx_hash],
                capture_output=True,
                text=True,
                timeout=45,  # Longer timeout
                cwd="/home/nima/code/crypto/rust/revm_tx_simulator"
            )
            
            success = result.returncode == 0
            stdout = result.stdout.strip() if result.stdout else ""
            stderr = result.stderr.strip() if result.stderr else ""
            
            if success and stdout:
                try:
                    json.loads(stdout)
                    return True, None, None
                except json.JSONDecodeError as e:
                    return False, f"JSON_PARSE_ERROR: {e}", stderr
            else:
                return False, "EXECUTION_FAILED", stderr
                
        except subprocess.TimeoutExpired:
            return False, "TIMEOUT", "Process timed out after 45 seconds"
        except Exception as e:
            return False, f"EXCEPTION: {e}", str(e)
    
    def get_recent_transactions(self, count: int = 500) -> List[str]:
        """Get recent transaction hashes using same logic as validation."""
        # Get latest block
        payload = {'jsonrpc': '2.0', 'method': 'eth_blockNumber', 'params': [], 'id': 1}
        response = requests.post(self.rpc_url, json=payload)
        latest_block = int(response.json()['result'], 16)
        
        transactions = []
        blocks_checked = 0
        
        print(f"Scanning blocks from {latest_block} backwards...")
        
        # Search back through blocks (same logic as original validator)
        for block_num in range(latest_block, latest_block - 200, -1):
            if len(transactions) >= count:
                break
                
            blocks_checked += 1
            if blocks_checked % 20 == 0:
                print(f"  Checked {blocks_checked} blocks, found {len(transactions)} transactions...")
            
            # Get block
            payload = {
                'jsonrpc': '2.0',
                'method': 'eth_getBlockByNumber',
                'params': [hex(block_num), True],
                'id': 1
            }
            
            try:
                response = requests.post(self.rpc_url, json=payload, timeout=5)
                block = response.json().get('result')
                
                if not block or not block.get('transactions'):
                    continue
                    
                # Add transactions with same filter as original validator
                for tx in block['transactions']:
                    if len(transactions) >= count:
                        break
                        
                    # Same filter logic: skip simple transfers
                    if (tx.get('to') and 
                        int(tx.get('gas', '0x0'), 16) > 21000):
                        transactions.append(tx['hash'])
                        
            except Exception as e:
                continue
        
        print(f"Found {len(transactions)} transactions in {blocks_checked} blocks")
        return transactions[:count]
    
    def analyze_failure_type(self, tx_details: Dict, error_msg: str, stderr: str) -> str:
        """Categorize the type of failure."""
        if not tx_details:
            return "TX_NOT_FOUND"
            
        # Check transaction status first
        tx_status = tx_details.get('status')
        if tx_status == '0x0':
            tx_failed_onchain = True
        elif tx_status == '0x1':
            tx_failed_onchain = False
        else:
            tx_failed_onchain = None  # Unknown
        
        # Analyze error patterns
        error_lower = error_msg.lower() if error_msg else ""
        stderr_lower = stderr.lower() if stderr else ""
        
        if "timeout" in error_lower:
            return "SIMULATION_TIMEOUT"
        elif "json" in error_lower and "parse" in error_lower:
            return "OUTPUT_FORMAT_ERROR"
        elif "nonce" in stderr_lower:
            return "NONCE_MISMATCH"
        elif "insufficient" in stderr_lower and "funds" in stderr_lower:
            return "INSUFFICIENT_FUNDS_SIM"
        elif "execution reverted" in stderr_lower or "revert" in stderr_lower:
            if tx_failed_onchain:
                return "TX_REVERTED_ONCHAIN"  # Expected - transaction failed on chain
            else:
                return "UNEXPECTED_REVERT"   # Simulation issue - tx succeeded on chain but reverts in sim
        elif "database" in stderr_lower or "rpc" in stderr_lower:
            return "RPC_DATABASE_ERROR"
        elif tx_details.get('to') is None:
            return "CONTRACT_CREATION_ISSUE"
        elif "invalid" in stderr_lower:
            return "INVALID_TRANSACTION"
        else:
            return "UNKNOWN_ERROR"
    
    def collect_failures(self, max_transactions: int = 500) -> Dict:
        """Collect and analyze simulation failures."""
        print(f"🔍 Collecting failure data from {max_transactions} transactions...")
        print("=" * 70)
        
        # Get transactions
        tx_hashes = self.get_recent_transactions(max_transactions)
        
        results = {
            'timestamp': time.time(),
            'total_tested': len(tx_hashes),
            'successes': 0,
            'failures': [],
            'failure_categories': {},
            'onchain_vs_simulation_status': {
                'both_success': 0,
                'both_failed': 0,
                'tx_failed_sim_succeeded': 0,
                'tx_succeeded_sim_failed': 0,
                'unknown_tx_status': 0
            }
        }
        
        print(f"Testing {len(tx_hashes)} transactions...")
        
        for i, tx_hash in enumerate(tx_hashes):
            if i % 50 == 0:
                print(f"\n[{i}/{len(tx_hashes)}] Progress: {i/len(tx_hashes)*100:.1f}%")
                if results['failures']:
                    print(f"  Failures so far: {len(results['failures'])} ({len(results['failures'])/max(i,1)*100:.1f}%)")
            
            # Get transaction details
            tx_details = self.get_transaction_details(tx_hash)
            
            # Run simulation
            success, error_msg, stderr = self.run_simulation(tx_hash)
            
            if success:
                results['successes'] += 1
                
                # Track correlation with on-chain status
                if tx_details and tx_details.get('status') == '0x1':
                    results['onchain_vs_simulation_status']['both_success'] += 1
                elif tx_details and tx_details.get('status') == '0x0':
                    results['onchain_vs_simulation_status']['tx_failed_sim_succeeded'] += 1
                else:
                    results['onchain_vs_simulation_status']['unknown_tx_status'] += 1
            else:
                # Analyze failure
                failure_type = self.analyze_failure_type(tx_details, error_msg, stderr)
                
                failure_detail = {
                    'tx_hash': tx_hash,
                    'tx_details': tx_details,
                    'error_msg': error_msg,
                    'stderr': stderr[:500] if stderr else None,  # Limit stderr length
                    'failure_type': failure_type
                }
                
                results['failures'].append(failure_detail)
                
                # Count failure categories
                if failure_type not in results['failure_categories']:
                    results['failure_categories'][failure_type] = 0
                results['failure_categories'][failure_type] += 1
                
                # Track correlation with on-chain status
                if tx_details:
                    if tx_details.get('status') == '0x1':
                        results['onchain_vs_simulation_status']['tx_succeeded_sim_failed'] += 1
                    elif tx_details.get('status') == '0x0':
                        results['onchain_vs_simulation_status']['both_failed'] += 1
                    else:
                        results['onchain_vs_simulation_status']['unknown_tx_status'] += 1
        
        return results
    
    def print_failure_analysis(self, results: Dict):
        """Print detailed failure analysis."""
        print("\n" + "=" * 70)
        print("DETAILED FAILURE ANALYSIS")
        print("=" * 70)
        
        total = results['total_tested']
        successes = results['successes'] 
        failures = len(results['failures'])
        
        print(f"\n📊 Overall Results:")
        print(f"  Total Tested: {total}")
        print(f"  Simulation Successes: {successes} ({successes/total*100:.1f}%)")
        print(f"  Simulation Failures: {failures} ({failures/total*100:.1f}%)")
        
        print(f"\n🔍 On-Chain vs Simulation Status:")
        status_counts = results['onchain_vs_simulation_status']
        print(f"  Both Success: {status_counts['both_success']}")
        print(f"  Both Failed: {status_counts['both_failed']}")
        print(f"  TX Failed, Sim Succeeded: {status_counts['tx_failed_sim_succeeded']}")
        print(f"  TX Succeeded, Sim Failed: {status_counts['tx_succeeded_sim_failed']} ⚠️")
        print(f"  Unknown TX Status: {status_counts['unknown_tx_status']}")
        
        print(f"\n📋 Failure Categories:")
        for failure_type, count in sorted(results['failure_categories'].items(), key=lambda x: x[1], reverse=True):
            percentage = count / failures * 100 if failures > 0 else 0
            print(f"  {failure_type}: {count} ({percentage:.1f}%)")
        
        print(f"\n📝 Sample Failures:")
        legitimate_failures = ["TX_REVERTED_ONCHAIN", "INSUFFICIENT_FUNDS_SIM"]
        simulation_bugs = []
        
        for failure in results['failures'][:10]:
            tx_status = failure['tx_details']['status'] if failure['tx_details'] else 'unknown'
            tx_status_str = "SUCCESS" if tx_status == '0x1' else "FAILED" if tx_status == '0x0' else "UNKNOWN"
            
            print(f"\n  {failure['tx_hash']}:")
            print(f"    On-chain Status: {tx_status_str}")
            print(f"    Failure Type: {failure['failure_type']}")
            
            if failure['failure_type'] not in legitimate_failures:
                simulation_bugs.append(failure)
            
            if failure['stderr']:
                stderr_preview = failure['stderr'][:150].replace('\n', ' ')
                print(f"    Error: {stderr_preview}...")
        
        # Summary
        simulation_bug_count = len(simulation_bugs)
        legitimate_failure_count = failures - simulation_bug_count
        
        print(f"\n🎯 CRITICAL ASSESSMENT:")
        print(f"  Legitimate Failures (TX failed on-chain): {legitimate_failure_count}")
        print(f"  Simulation Bugs (Our code failed): {simulation_bug_count}")
        
        if simulation_bug_count > 0:
            print(f"  ⚠️  {simulation_bug_count} failures need to be fixed in our simulator!")
        else:
            print(f"  ✅ All failures appear to be legitimate transaction failures!")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="Capture and analyze simulation failures")
    parser.add_argument("--transactions", "-n", type=int, default=500, 
                       help="Number of transactions to test")
    parser.add_argument("--save", "-s", action="store_true",
                       help="Save detailed results to file")
    
    args = parser.parse_args()
    
    collector = FailureCollector()
    results = collector.collect_failures(args.transactions)
    collector.print_failure_analysis(results)
    
    if args.save:
        filename = f"detailed_failures_{int(time.time())}.json"
        with open(filename, "w") as f:
            json.dump(results, f, indent=2, default=str)
        print(f"\n💾 Detailed results saved to: {filename}")