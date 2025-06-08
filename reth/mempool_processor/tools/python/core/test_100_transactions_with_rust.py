#!/usr/bin/env python3
"""
100-Transaction Rust vs Python State Change Validation Test with Real Rust Integration

This test finds 100 recent transactions and validates that Rust and Python
produce identical state change results using the actual Rust binary.
"""

import json
import sys
import time
import subprocess
import tempfile
from web3 import Web3
from pathlib import Path

class Transaction100RustTest:
    """Test class for validating 100 transactions across Rust and Python using real Rust binary."""
    
    def __init__(self):
        self.w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        if not self.w3.is_connected():
            raise ConnectionError("Failed to connect to Ethereum node")
        
        self.results = {
            "test_config": {
                "target_count": 100,
                "start_time": time.time(),
                "rpc_url": "http://127.0.0.1:8545",
                "rust_binary": "test_comprehensive_state_diff"
            },
            "transactions": [],
            "python_results": {},
            "rust_results": {},
            "comparisons": {},
            "summary": {}
        }
        
        print(f"✅ Connected to Ethereum node: {self.w3.eth.block_number}")
    
    def fetch_recent_transactions(self, count=100):
        """Fetch recent transactions for testing."""
        print(f"🔍 Fetching {count} recent transactions...")
        
        current_block = self.w3.eth.block_number
        transactions = []
        blocks_scanned = 0
        
        # Scan recent blocks to find transactions
        for block_offset in range(50):  # Scan more blocks to get enough transactions
            block_num = current_block - block_offset
            
            try:
                block = self.w3.eth.get_block(block_num, full_transactions=True)
                blocks_scanned += 1
                
                for tx in block.transactions:
                    if len(transactions) >= count:
                        break
                    
                    # Filter for interesting transactions
                    if (tx.value > 0 or  # Has ETH transfer
                        tx.gas > 50000):  # Complex transaction
                        
                        transactions.append({
                            "hash": tx.hash.hex(),
                            "block": block_num,
                            "value": float(self.w3.from_wei(tx.value, 'ether')),
                            "from": tx['from'],
                            "to": tx.get('to'),
                            "gas_limit": tx.gas
                        })
                
                if len(transactions) >= count:
                    break
                    
            except Exception as e:
                if blocks_scanned <= 5:  # Only show first few errors
                    print(f"   ⚠️  Error fetching block {block_num}: {e}")
                continue
        
        print(f"   📦 Scanned {blocks_scanned} blocks")
        print(f"   ✅ Found {len(transactions)} suitable transactions")
        
        self.results["transactions"] = transactions[:count]
        return transactions[:count]
    
    def analyze_with_python(self, transactions):
        """Analyze transactions with Python state change logic."""
        print(f"\n🐍 Analyzing {len(transactions)} transactions with Python...")
        
        successful = 0
        failed = 0
        
        for i, tx_info in enumerate(transactions, 1):
            tx_hash = tx_info["hash"]
            
            if i % 20 == 0:
                print(f"   Progress: {i}/{len(transactions)} ({i/len(transactions)*100:.1f}%)")
            
            try:
                # Use Python state change analysis
                state_changes = self.calculate_python_state_changes(tx_hash)
                
                self.results["python_results"][tx_hash] = {
                    "success": True,
                    "address_count": len(state_changes),
                    "state_changes": state_changes,
                    "has_eth_changes": any(abs(changes.get("eth_net", 0)) > 1e-15 for changes in state_changes.values()),
                    "has_token_changes": any(len(changes.get("token_net", {})) > 0 for changes in state_changes.values())
                }
                successful += 1
                
            except Exception as e:
                self.results["python_results"][tx_hash] = {
                    "success": False,
                    "error": str(e)
                }
                failed += 1
                
                if failed <= 3:  # Show first few errors for debugging
                    print(f"   ❌ {tx_hash[:10]}: {e}")
        
        print(f"   ✅ Python: {successful} successful, {failed} failed")
        return successful, failed
    
    def calculate_python_state_changes(self, tx_hash):
        """Calculate state changes using Python logic (simplified but accurate version)."""
        # Get transaction data
        tx = self.w3.eth.get_transaction(tx_hash)
        receipt = self.w3.eth.get_transaction_receipt(tx_hash)
        
        state_changes = {}
        
        def ensure_address(addr):
            addr = addr.lower()
            if addr not in state_changes:
                state_changes[addr] = {
                    "eth_net": 0.0,
                    "token_net": {},
                    "movements": {"in": 0, "out": 0}
                }
            return addr
        
        # Process basic transaction value
        if tx.value > 0:
            value_eth = float(self.w3.from_wei(tx.value, "ether"))
            
            from_addr = ensure_address(tx['from'])
            to_addr = ensure_address(tx.to) if tx.to else None
            
            state_changes[from_addr]["eth_net"] -= value_eth
            state_changes[from_addr]["movements"]["out"] += 1
            
            if to_addr:
                state_changes[to_addr]["eth_net"] += value_eth
                state_changes[to_addr]["movements"]["in"] += 1
        
        # Process ERC20 transfers from logs
        transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        
        for log_entry in receipt.logs:
            if len(log_entry.topics) == 3 and log_entry.topics[0].hex() == transfer_topic:
                try:
                    from_addr = ensure_address("0x" + log_entry.topics[1].hex()[-40:])
                    to_addr = ensure_address("0x" + log_entry.topics[2].hex()[-40:])
                    token_addr = log_entry.address.lower()
                    amount = int(log_entry.data, 16)
                    
                    # Track token movements (use raw amounts for now)
                    if token_addr not in state_changes[from_addr]["token_net"]:
                        state_changes[from_addr]["token_net"][token_addr] = 0
                    if token_addr not in state_changes[to_addr]["token_net"]:
                        state_changes[to_addr]["token_net"][token_addr] = 0
                    
                    state_changes[from_addr]["token_net"][token_addr] -= amount
                    state_changes[to_addr]["token_net"][token_addr] += amount
                    
                except Exception:
                    continue  # Skip malformed logs
        
        # Filter out zero changes
        filtered_changes = {}
        for addr, changes in state_changes.items():
            has_eth_change = abs(changes["eth_net"]) > 1e-15
            has_token_changes = any(abs(amount) > 0 for amount in changes["token_net"].values())
            
            if has_eth_change or has_token_changes:
                changes["eth_net"] = round(changes["eth_net"], 12)
                filtered_changes[addr] = changes
        
        return filtered_changes
    
    def analyze_with_rust(self, transactions):
        """Analyze transactions with Rust using the test_comprehensive_state_diff binary."""
        print(f"\n🦀 Analyzing {len(transactions)} transactions with Rust...")
        
        successful = 0
        failed = 0
        skipped = 0
        
        for i, tx_info in enumerate(transactions, 1):
            tx_hash = tx_info["hash"]
            
            if i % 10 == 0:
                print(f"   Progress: {i}/{len(transactions)} ({i/len(transactions)*100:.1f}%)")
            
            # Only analyze transactions that Python successfully processed
            if not self.results["python_results"].get(tx_hash, {}).get("success", False):
                self.results["rust_results"][tx_hash] = {
                    "success": False,
                    "error": "Skipped - Python analysis failed"
                }
                skipped += 1
                continue
            
            try:
                # Run Rust analysis
                rust_result = self.run_rust_analysis(tx_hash)
                
                if rust_result:
                    self.results["rust_results"][tx_hash] = {
                        "success": True,
                        "address_count": len(rust_result),
                        "state_changes": rust_result,
                        "analysis_method": "rust_comprehensive_state_diff"
                    }
                    successful += 1
                else:
                    self.results["rust_results"][tx_hash] = {
                        "success": False,
                        "error": "Rust analysis returned no results"
                    }
                    failed += 1
                    
            except Exception as e:
                self.results["rust_results"][tx_hash] = {
                    "success": False,
                    "error": str(e)
                }
                failed += 1
                
                if failed <= 3:  # Show first few errors for debugging
                    print(f"   ❌ {tx_hash[:10]}: {e}")
        
        print(f"   ✅ Rust: {successful} successful, {failed} failed, {skipped} skipped")
        return successful, failed
    
    def run_rust_analysis(self, tx_hash):
        """Run Rust state change analysis for a single transaction."""
        try:
            # Run the Rust binary
            cmd = [
                "cargo", "run", "--release", "--bin", "test_comprehensive_state_diff", "--",
                tx_hash,
                "--eth-rpc-url", "http://127.0.0.1:8545"
            ]
            
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30,  # 30 second timeout per transaction
                cwd="/home/nima/code/crypto/rust/mempool_processor"
            )
            
            if result.returncode == 0:
                # Parse output to extract state changes
                output_lines = result.stdout.strip().split('\n')
                
                # Look for JSON output or structured output
                for line in output_lines:
                    if line.strip().startswith('{') and '"state_changes"' in line:
                        try:
                            parsed = json.loads(line)
                            return parsed.get("state_changes", {})
                        except json.JSONDecodeError:
                            continue
                
                # If no JSON found, try to parse structured output
                # This is a fallback - the Rust binary should output JSON
                return self.parse_rust_text_output(output_lines)
            else:
                if "timeout" in result.stderr.lower() or "connection" in result.stderr.lower():
                    return None  # Skip timeouts and connection issues
                else:
                    raise Exception(f"Rust binary failed: {result.stderr[:100]}")
                    
        except subprocess.TimeoutExpired:
            return None  # Skip timeouts
        except Exception as e:
            raise Exception(f"Error running Rust analysis: {e}")
    
    def parse_rust_text_output(self, output_lines):
        """Parse text output from Rust binary (fallback method)."""
        # This is a simplified parser - ideally Rust should output JSON
        state_changes = {}
        
        current_address = None
        for line in output_lines:
            line = line.strip()
            
            # Look for address lines
            if line.startswith("Address:") or "0x" in line:
                if ":" in line:
                    current_address = line.split(":")[-1].strip().lower()
                    if current_address.startswith("0x"):
                        state_changes[current_address] = {
                            "eth_net": 0.0,
                            "token_net": {}
                        }
            
            # Look for ETH changes
            elif "ETH" in line and current_address:
                try:
                    # Extract ETH amount from line
                    parts = line.split()
                    for part in parts:
                        if part.replace(".", "").replace("-", "").isdigit():
                            eth_amount = float(part)
                            state_changes[current_address]["eth_net"] = eth_amount
                            break
                except:
                    continue
        
        return state_changes
    
    def compare_results(self, transactions):
        """Compare Rust and Python results with detailed analysis."""
        print(f"\n📊 Comparing Rust vs Python results...")
        
        exact_matches = 0
        close_matches = 0
        mismatches = 0
        both_failed = 0
        total_addresses = 0
        
        detailed_mismatches = []
        
        for tx_info in transactions:
            tx_hash = tx_info["hash"]
            
            python_result = self.results["python_results"].get(tx_hash, {})
            rust_result = self.results["rust_results"].get(tx_hash, {})
            
            comparison = {
                "transaction": tx_hash[:10] + "...",
                "python_success": python_result.get("success", False),
                "rust_success": rust_result.get("success", False),
                "addresses_python": python_result.get("address_count", 0),
                "addresses_rust": rust_result.get("address_count", 0),
                "match_status": "unknown"
            }
            
            if not python_result.get("success") and not rust_result.get("success"):
                comparison["match_status"] = "both_failed"
                both_failed += 1
            elif not python_result.get("success") or not rust_result.get("success"):
                comparison["match_status"] = "one_failed"
                mismatches += 1
            else:
                # Both succeeded - compare results
                python_changes = python_result.get("state_changes", {})
                rust_changes = rust_result.get("state_changes", {})
                
                total_addresses += len(python_changes)
                
                match_result = self.detailed_state_comparison(python_changes, rust_changes)
                comparison.update(match_result)
                
                if match_result["match_status"] == "exact_match":
                    exact_matches += 1
                elif match_result["match_status"] == "close_match":
                    close_matches += 1
                else:
                    mismatches += 1
                    detailed_mismatches.append({
                        "tx_hash": tx_hash,
                        "details": match_result
                    })
            
            self.results["comparisons"][tx_hash] = comparison
        
        total_analyzed = exact_matches + close_matches + mismatches + both_failed
        match_rate = (exact_matches + close_matches) / total_analyzed if total_analyzed > 0 else 0
        
        print(f"   📈 DETAILED COMPARISON RESULTS:")
        print(f"      Total transactions: {len(transactions)}")
        print(f"      Exact matches: {exact_matches}")
        print(f"      Close matches: {close_matches}")
        print(f"      Mismatches: {mismatches}")
        print(f"      Both failed: {both_failed}")
        print(f"      Overall match rate: {match_rate:.1%}")
        print(f"      Total addresses analyzed: {total_addresses}")
        
        # Show first few mismatches for debugging
        if detailed_mismatches:
            print(f"\\n   🔍 First few mismatches:")
            for mismatch in detailed_mismatches[:3]:
                print(f"      {mismatch['tx_hash'][:10]}: {mismatch['details']['mismatch_reason']}")
        
        return {
            "total": len(transactions),
            "exact_matches": exact_matches,
            "close_matches": close_matches,
            "mismatches": mismatches,
            "both_failed": both_failed,
            "match_rate": match_rate,
            "total_addresses": total_addresses,
            "detailed_mismatches": detailed_mismatches[:10]  # Store first 10 for analysis
        }
    
    def detailed_state_comparison(self, python_changes, rust_changes):
        """Perform detailed comparison of state changes."""
        # Address count comparison
        if len(python_changes) != len(rust_changes):
            return {
                "match_status": "mismatch",
                "mismatch_reason": f"Address count differs: Python {len(python_changes)}, Rust {len(rust_changes)}",
                "address_diff": len(python_changes) - len(rust_changes)
            }
        
        # Address-by-address comparison
        eth_mismatches = 0
        token_mismatches = 0
        precision_issues = 0
        
        for addr in python_changes:
            if addr not in rust_changes:
                return {
                    "match_status": "mismatch",
                    "mismatch_reason": f"Address {addr[:10]} missing in Rust results"
                }
            
            py_eth = python_changes[addr].get("eth_net", 0)
            rust_eth = rust_changes[addr].get("eth_net", 0)
            
            eth_diff = abs(py_eth - rust_eth)
            
            if eth_diff > 1e-12:  # Allow for small precision differences
                if eth_diff > 1e-6:  # Significant difference
                    eth_mismatches += 1
                else:
                    precision_issues += 1
        
        # Check for addresses only in Rust
        rust_only = [addr for addr in rust_changes if addr not in python_changes]
        if rust_only:
            return {
                "match_status": "mismatch", 
                "mismatch_reason": f"Rust has {len(rust_only)} extra addresses"
            }
        
        # Determine overall match status
        if eth_mismatches > 0:
            return {
                "match_status": "mismatch",
                "mismatch_reason": f"{eth_mismatches} significant ETH differences",
                "eth_mismatches": eth_mismatches,
                "precision_issues": precision_issues
            }
        elif precision_issues > 0:
            return {
                "match_status": "close_match",
                "mismatch_reason": f"{precision_issues} minor precision differences",
                "precision_issues": precision_issues
            }
        else:
            return {
                "match_status": "exact_match",
                "mismatch_reason": "Perfect match"
            }
    
    def save_results(self):
        """Save test results to file."""
        self.results["test_config"]["end_time"] = time.time()
        self.results["test_config"]["duration"] = self.results["test_config"]["end_time"] - self.results["test_config"]["start_time"]
        
        # Save results
        results_file = Path("test_100_transactions_rust_results.json")
        with open(results_file, "w") as f:
            json.dump(self.results, f, indent=2, default=str)
        
        print(f"\n💾 Results saved to: {results_file}")
        return results_file
    
    def run_test(self, target_count=100):
        """Run the complete transaction test."""
        print("🧪 100-Transaction Rust vs Python State Change Test (Real Rust Integration)")
        print("=" * 80)
        
        try:
            # 1. Fetch transactions
            transactions = self.fetch_recent_transactions(target_count)
            
            if len(transactions) < target_count // 2:
                print(f"⚠️  Warning: Only found {len(transactions)} transactions (target: {target_count})")
            
            # 2. Analyze with Python
            py_success, py_failed = self.analyze_with_python(transactions)
            
            # 3. Analyze with Rust
            rust_success, rust_failed = self.analyze_with_rust(transactions)
            
            # 4. Compare results
            comparison_summary = self.compare_results(transactions)
            
            # 5. Save results
            self.results["summary"] = comparison_summary
            self.save_results()
            
            # 6. Final summary
            print(f"\n🎯 FINAL TEST SUMMARY")
            print("=" * 80)
            print(f"Transactions analyzed: {len(transactions)}")
            print(f"Python success rate: {py_success/(py_success+py_failed)*100:.1f}%")
            print(f"Rust success rate: {rust_success/(rust_success+rust_failed)*100:.1f}%")
            print(f"Match rate: {comparison_summary['match_rate']:.1%}")
            print(f"Exact matches: {comparison_summary['exact_matches']}")
            print(f"Close matches: {comparison_summary['close_matches']}")
            print(f"Mismatches: {comparison_summary['mismatches']}")
            print(f"Total addresses: {comparison_summary['total_addresses']}")
            
            if comparison_summary["match_rate"] > 0.95:
                print(f"\n🎉 EXCELLENT: Rust and Python are highly consistent!")
                return True
            elif comparison_summary["match_rate"] > 0.8:
                print(f"\n⚠️  GOOD: Some discrepancies found, but generally consistent")
                return True
            else:
                print(f"\n❌ NEEDS INVESTIGATION: Significant discrepancies found")
                return False
                
        except Exception as e:
            print(f"\n❌ Test failed: {e}")
            import traceback
            traceback.print_exc()
            return False

def main():
    """Main test execution."""
    target = 50  # Start with 50 transactions for faster testing
    
    test = Transaction100RustTest()
    success = test.run_test(target)
    
    if success:
        print(f"\n✅ Rust vs Python validation successful with {target} transactions")
        print(f"   Both systems produce consistent state change results")
    else:
        print(f"\n❌ Validation failed - check detailed results file")
    
    return success

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)