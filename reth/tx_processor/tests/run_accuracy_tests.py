#!/usr/bin/env python3
"""
Comprehensive accuracy validation test runner for REVM vs Python implementation.
This script can be run regularly to ensure ongoing accuracy between implementations.
"""

import subprocess
import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Tuple, Optional
import argparse

# Add the necessary Python paths for imports
sys.path.append('/home/nima/code/crypto/rust/mempool_processor/python')
sys.path.append('/home/nima/code/crypto/py')

# Import the validation functionality
try:
    from core.validate_state_changes import get_transaction_state_changes_for_validation
except ImportError:
    print("❌ Failed to import validation functions from mempool_processor")
    print("   Make sure the mempool_processor Python package is available")
    sys.exit(1)

class AccuracyTestRunner:
    def __init__(self, revm_dir: str = "/home/nima/code/crypto/rust/revm_tx_simulator"):
        self.revm_dir = Path(revm_dir)
        
    def run_comprehensive_accuracy_test(self, num_transactions: int = 100) -> Dict:
        """Run comprehensive accuracy tests comparing REVM vs Python."""
        print(f"🧪 Running comprehensive accuracy test with {num_transactions} transactions...")
        
        results = {
            "timestamp": time.time(),
            "total_transactions": 0,
            "successful_matches": 0,
            "perfect_matches": 0,
            "minor_differences": 0,
            "significant_differences": 0,
            "failed_transactions": 0,
            "performance_metrics": {},
            "detailed_results": [],
            "summary": {}
        }
        
        start_time = time.time()
        
        # Get recent transactions for testing
        transactions = self._get_recent_transactions(num_transactions)
        results["total_transactions"] = len(transactions)
        
        print(f"📊 Testing {len(transactions)} transactions...")
        
        for i, tx_hash in enumerate(transactions):
            if i % 10 == 0:
                print(f"  Progress: {i}/{len(transactions)} ({i/len(transactions)*100:.1f}%)")
            
            try:
                comparison = self._compare_transaction(tx_hash)
                results["detailed_results"].append(comparison)
                
                if comparison["status"] == "perfect_match":
                    results["perfect_matches"] += 1
                    results["successful_matches"] += 1
                elif comparison["status"] == "minor_differences":
                    results["minor_differences"] += 1
                    results["successful_matches"] += 1
                elif comparison["status"] == "significant_differences":
                    results["significant_differences"] += 1
                else:  # failed
                    results["failed_transactions"] += 1
                    
            except Exception as e:
                results["failed_transactions"] += 1
                results["detailed_results"].append({
                    "tx_hash": tx_hash,
                    "status": "failed",
                    "error": str(e)
                })
        
        # Calculate performance metrics
        elapsed_time = time.time() - start_time
        results["performance_metrics"] = {
            "total_time_seconds": elapsed_time,
            "average_time_per_transaction": elapsed_time / len(transactions) if transactions else 0,
            "throughput_tx_per_second": len(transactions) / elapsed_time if elapsed_time > 0 else 0
        }
        
        # Generate summary
        total = results["total_transactions"]
        results["summary"] = {
            "success_rate": results["successful_matches"] / total if total > 0 else 0,
            "perfect_match_rate": results["perfect_matches"] / total if total > 0 else 0,
            "failure_rate": results["failed_transactions"] / total if total > 0 else 0,
            "accuracy_assessment": self._assess_accuracy(results)
        }
        
        return results
    
    def _get_recent_transactions(self, count: int) -> List[str]:
        """Get recent transactions using the REVM validator."""
        try:
            cmd = [
                "cargo", "run", "--example", "json_state_validator_no_rpc", 
                "--", "--count", str(count)
            ]
            
            result = subprocess.run(
                cmd,
                cwd=self.revm_dir,
                capture_output=True,
                text=True,
                timeout=300  # 5 minute timeout
            )
            
            if result.returncode != 0:
                print(f"⚠️  Warning: Failed to get transactions via REVM: {result.stderr}")
                return []
            
            # Extract transaction hashes from output
            transactions = []
            for line in result.stdout.split('\n'):
                if 'Processing transaction:' in line:
                    # Extract hash from line like "Processing transaction: 0x..."
                    parts = line.split('0x')
                    if len(parts) > 1:
                        hash_part = '0x' + parts[1].split()[0]
                        if len(hash_part) == 66:  # Valid transaction hash length
                            transactions.append(hash_part)
            
            return transactions[:count]
            
        except Exception as e:
            print(f"⚠️  Failed to get recent transactions: {e}")
            return []
    
    def _compare_transaction(self, tx_hash: str) -> Dict:
        """Compare a single transaction between Python and REVM implementations."""
        comparison = {
            "tx_hash": tx_hash,
            "status": "unknown",
            "python_result": None,
            "rust_result": None,
            "differences": [],
            "processing_time": 0
        }
        
        start_time = time.time()
        
        try:
            # Get Python result
            python_result = self._get_python_state_changes(tx_hash)
            comparison["python_result"] = python_result
            
            # Get Rust result
            rust_result = self._get_rust_state_changes(tx_hash)
            comparison["rust_result"] = rust_result
            
            # Compare results
            comparison_result = self._compare_state_changes(python_result, rust_result)
            comparison.update(comparison_result)
            
            comparison["processing_time"] = time.time() - start_time
            
        except Exception as e:
            comparison["status"] = "failed"
            comparison["error"] = str(e)
            comparison["processing_time"] = time.time() - start_time
        
        return comparison
    
    def _get_python_state_changes(self, tx_hash: str) -> Dict:
        """Get state changes from Python implementation."""
        try:
            # Use the validation function directly
            state_changes = get_transaction_state_changes_for_validation(tx_hash)
            return {"state_changes": state_changes}
        except Exception as e:
            raise Exception(f"Python processing failed: {e}")
    
    def _get_rust_state_changes(self, tx_hash: str) -> Dict:
        """Get state changes from Rust REVM implementation."""
        try:
            cmd = [
                "cargo", "run", "--example", "json_state_validator_no_rpc",
                "--", "--tx-hash", tx_hash
            ]
            
            result = subprocess.run(
                cmd,
                cwd=self.revm_dir,
                capture_output=True,
                text=True,
                timeout=60
            )
            
            if result.returncode != 0:
                raise Exception(f"Rust processing failed: {result.stderr}")
            
            # Extract JSON from output
            for line in result.stdout.split('\n'):
                line = line.strip()
                if line.startswith('{') and line.endswith('}'):
                    return json.loads(line)
            
            raise Exception("No JSON output found in Rust result")
            
        except subprocess.TimeoutExpired:
            raise Exception("Rust processing timed out")
        except Exception as e:
            raise Exception(f"Rust processing error: {e}")
    
    def _compare_state_changes(self, python_result: Dict, rust_result: Dict) -> Dict:
        """Compare state changes between Python and Rust results."""
        comparison = {
            "status": "unknown",
            "differences": [],
            "stats": {
                "addresses_compared": 0,
                "perfect_matches": 0,
                "minor_differences": 0,
                "significant_differences": 0
            }
        }
        
        # Extract state changes
        python_changes = python_result.get("state_changes", {})
        rust_changes = rust_result.get("state_changes", {})
        
        # Get all addresses that have changes in either implementation
        all_addresses = set(python_changes.keys()) | set(rust_changes.keys())
        comparison["stats"]["addresses_compared"] = len(all_addresses)
        
        significant_diffs = []
        minor_diffs = []
        
        for address in all_addresses:
            python_change = python_changes.get(address, {}).get("eth_change", 0.0)
            rust_change = rust_changes.get(address, {}).get("eth_change", 0.0)
            
            difference = abs(python_change - rust_change)
            
            if difference == 0.0:
                comparison["stats"]["perfect_matches"] += 1
            elif difference > 0.005:  # Significant difference threshold
                comparison["stats"]["significant_differences"] += 1
                significant_diffs.append({
                    "address": address,
                    "python_change": python_change,
                    "rust_change": rust_change,
                    "difference": difference
                })
            else:  # Minor difference
                comparison["stats"]["minor_differences"] += 1
                minor_diffs.append({
                    "address": address,
                    "python_change": python_change,
                    "rust_change": rust_change,
                    "difference": difference
                })
        
        # Determine overall status
        if significant_diffs:
            comparison["status"] = "significant_differences"
            comparison["differences"] = significant_diffs
        elif minor_diffs:
            comparison["status"] = "minor_differences"
            comparison["differences"] = minor_diffs
        else:
            comparison["status"] = "perfect_match"
        
        return comparison
    
    def _assess_accuracy(self, results: Dict) -> str:
        """Assess overall accuracy based on test results."""
        success_rate = results["summary"]["success_rate"]
        perfect_rate = results["summary"]["perfect_match_rate"]
        failure_rate = results["summary"]["failure_rate"]
        
        if failure_rate > 0.05:  # More than 5% failures
            return "POOR - High failure rate"
        elif success_rate < 0.90:  # Less than 90% success
            return "POOR - Low success rate"
        elif success_rate < 0.95:  # Less than 95% success
            return "FAIR - Moderate accuracy"
        elif perfect_rate > 0.80:  # More than 80% perfect matches
            return "EXCELLENT - High precision"
        else:
            return "GOOD - Acceptable accuracy"
    
    def print_results(self, results: Dict):
        """Print formatted test results."""
        print("\n" + "="*80)
        print("🧪 REVM vs Python Accuracy Test Results")
        print("="*80)
        
        # Summary stats
        summary = results["summary"]
        print(f"\n📊 SUMMARY:")
        print(f"  • Total Transactions: {results['total_transactions']}")
        print(f"  • Success Rate: {summary['success_rate']:.1%}")
        print(f"  • Perfect Matches: {results['perfect_matches']} ({summary['perfect_match_rate']:.1%})")
        print(f"  • Minor Differences: {results['minor_differences']}")
        print(f"  • Significant Differences: {results['significant_differences']}")
        print(f"  • Failed Transactions: {results['failed_transactions']}")
        print(f"  • Overall Assessment: {summary['accuracy_assessment']}")
        
        # Performance metrics
        perf = results["performance_metrics"]
        print(f"\n⏱️  PERFORMANCE:")
        print(f"  • Total Time: {perf['total_time_seconds']:.2f}s")
        print(f"  • Avg Time/Transaction: {perf['average_time_per_transaction']:.3f}s")
        print(f"  • Throughput: {perf['throughput_tx_per_second']:.1f} tx/s")
        
        # Detailed issues
        if results['significant_differences'] > 0:
            print(f"\n⚠️  SIGNIFICANT DIFFERENCES FOUND:")
            sig_diff_transactions = [r for r in results['detailed_results'] if r['status'] == 'significant_differences']
            for tx_result in sig_diff_transactions[:5]:  # Show first 5
                print(f"  • Transaction: {tx_result['tx_hash']}")
                for diff in tx_result.get('differences', [])[:3]:  # Show first 3 differences
                    print(f"    - Address {diff['address']}: Python={diff['python_change']:.6f}, Rust={diff['rust_change']:.6f}")
        
        if results['failed_transactions'] > 0:
            print(f"\n❌ FAILED TRANSACTIONS:")
            failed_transactions = [r for r in results['detailed_results'] if r['status'] == 'failed']
            for tx_result in failed_transactions[:3]:  # Show first 3
                print(f"  • {tx_result['tx_hash']}: {tx_result.get('error', 'Unknown error')}")
        
        # Final verdict
        if summary['success_rate'] >= 0.95 and results['significant_differences'] == 0:
            print(f"\n✅ VERDICT: READY FOR PRODUCTION")
            print(f"   High accuracy maintained with {summary['success_rate']:.1%} success rate")
        elif summary['success_rate'] >= 0.90:
            print(f"\n⚠️  VERDICT: NEEDS ATTENTION")
            print(f"   Moderate accuracy with {results['significant_differences']} significant differences")
        else:
            print(f"\n❌ VERDICT: CRITICAL ISSUES")
            print(f"   Low accuracy requiring immediate investigation")

def main():
    parser = argparse.ArgumentParser(description="Run REVM vs Python accuracy validation tests")
    parser.add_argument("--transactions", "-t", type=int, default=100, 
                       help="Number of transactions to test (default: 100)")
    parser.add_argument("--output", "-o", type=str, 
                       help="Output file for detailed results (JSON format)")
    parser.add_argument("--quick", "-q", action="store_true",
                       help="Run quick test with 25 transactions")
    parser.add_argument("--comprehensive", "-c", action="store_true",
                       help="Run comprehensive test with 500 transactions")
    
    args = parser.parse_args()
    
    # Determine number of transactions
    if args.quick:
        num_transactions = 25
    elif args.comprehensive:
        num_transactions = 500
    else:
        num_transactions = args.transactions
    
    # Run the test
    runner = AccuracyTestRunner()
    results = runner.run_comprehensive_accuracy_test(num_transactions)
    
    # Print results
    runner.print_results(results)
    
    # Save detailed results if requested
    if args.output:
        with open(args.output, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"\n💾 Detailed results saved to: {args.output}")
    
    # Exit with appropriate code
    if results["summary"]["success_rate"] >= 0.95 and results["significant_differences"] == 0:
        sys.exit(0)  # Success
    else:
        sys.exit(1)  # Issues found

if __name__ == "__main__":
    main()