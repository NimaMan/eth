#!/usr/bin/env python3
"""
Comprehensive Integration Test: Rust vs Python State Change Detection

This test validates that the Rust mempool processor and Python state change 
detection systems produce identical results for complex transactions with:
- Internal transfers
- ERC20 token movements
- Multi-step DeFi interactions
- Complex MEV transactions

Test transactions selected based on complexity score (>20) including:
1. Multiple internal ETH transfers
2. Contract interactions with logs
3. High gas usage indicating complex computation
4. Real-world DeFi scenarios
"""

import json
import sys
import os
from pathlib import Path

# Add the core directory to path for imports
sys.path.append(str(Path(__file__).parent.parent / "python" / "core"))

try:
    from python_state_analyzer import PythonStateAnalyzer
except ImportError as e:
    print(f"❌ Failed to import PythonStateAnalyzer: {e}")
    print("   Make sure you're in the correct conda environment (qw)")
    sys.exit(1)

class RustPythonComparisonTest:
    """Test class for comparing Rust and Python state change detection."""
    
    def __init__(self):
        self.test_transactions = [
            {
                "hash": "0x290fa323da94b338ade0690abaf313d4a5307e4a274ffd742d12cd50617f33e8",
                "description": "Complex DeFi swap with 1 ETH value transfer, 14 internal calls, 7 logs",
                "complexity_score": 43,
                "expected_features": ["value_transfer", "internal_calls", "contract_logs"]
            },
            {
                "hash": "0x01e025cc593d597c9b8b83f7c3a64d1d2399ae6f485511a69729673ef8061761", 
                "description": "Multi-step transaction with 0.051 ETH, 11 internal calls, 5 logs",
                "complexity_score": 35,
                "expected_features": ["value_transfer", "internal_calls", "contract_logs"]
            },
            {
                "hash": "0x799252ab72588c6f7e414cde250ca2a1bb030979f36ff16918bf3ce2e4963386",
                "description": "MEV transaction with minimal ETH value, 5 internal calls, 4 logs",
                "complexity_score": 22,
                "expected_features": ["minimal_value", "internal_calls", "contract_logs"]
            }
        ]
        
        self.python_analyzer = None
        self.results = {
            "python_analysis": {},
            "rust_analysis": {},  # Will be populated when Rust system is available
            "comparisons": {},
            "test_summary": {}
        }
    
    def setup(self):
        """Initialize the Python analyzer."""
        try:
            self.python_analyzer = PythonStateAnalyzer()
            print("✅ Python analyzer initialized successfully")
            return True
        except Exception as e:
            print(f"❌ Failed to initialize Python analyzer: {e}")
            return False
    
    def analyze_with_python(self):
        """Analyze all test transactions with Python."""
        print("\n🐍 Running Python State Change Analysis")
        print("=" * 60)
        
        for i, tx_info in enumerate(self.test_transactions, 1):
            tx_hash = tx_info["hash"]
            description = tx_info["description"]
            
            print(f"\n{i}. {description}")
            print(f"   Hash: {tx_hash}")
            
            try:
                state_changes = self.python_analyzer.calculate_state_changes(tx_hash)
                formatted = self.python_analyzer.format_state_changes_for_comparison(state_changes)
                
                self.results["python_analysis"][tx_hash] = {
                    "raw_state_changes": state_changes,
                    "formatted_for_comparison": formatted,
                    "success": True,
                    "address_count": len(formatted),
                    "has_eth_changes": any(abs(data["eth_net"]) > 1e-15 for data in formatted.values()),
                    "has_token_changes": any(data["token_count"] > 0 for data in formatted.values())
                }
                
                print(f"   ✅ Python Success: {len(formatted)} addresses with state changes")
                
                # Validate expected features
                result = self.results["python_analysis"][tx_hash]
                features_found = []
                if result["has_eth_changes"]:
                    features_found.append("eth_changes")
                if result["has_token_changes"]:
                    features_found.append("token_changes")
                if len(formatted) > 2:
                    features_found.append("multi_address")
                
                print(f"   📋 Features detected: {', '.join(features_found)}")
                
            except Exception as e:
                print(f"   ❌ Python Error: {e}")
                self.results["python_analysis"][tx_hash] = {
                    "success": False,
                    "error": str(e)
                }
    
    def analyze_with_rust(self):
        """Analyze all test transactions with Rust (placeholder for when system is fixed)."""
        print("\n🦀 Rust State Change Analysis")
        print("=" * 60)
        print("⚠️  Rust analysis not yet available due to dependency issues")
        print("   When Rust system is fixed, this will:")
        print("   1. Run cargo run --bin test_comprehensive_state_diff for each transaction")
        print("   2. Parse JSON output from Rust state diff calculator")
        print("   3. Store results in self.results['rust_analysis']")
        print("   4. Compare address-by-address ETH and token changes")
        
        # Placeholder structure for future Rust integration
        for tx_info in self.test_transactions:
            tx_hash = tx_info["hash"]
            self.results["rust_analysis"][tx_hash] = {
                "status": "not_implemented",
                "note": "Will be implemented when Rust compilation issues are resolved"
            }
    
    def compare_results(self):
        """Compare Python and Rust results (placeholder for when Rust is available)."""
        print("\n📊 Comparison Analysis")
        print("=" * 60)
        
        python_results = self.results["python_analysis"]
        rust_results = self.results["rust_analysis"]
        
        successful_python = sum(1 for r in python_results.values() if r.get("success", False))
        total_addresses = sum(r.get("address_count", 0) for r in python_results.values() if r.get("success", False))
        
        print(f"Python Analysis Results:")
        print(f"   Successful transactions: {successful_python}/{len(self.test_transactions)}")
        print(f"   Total addresses analyzed: {total_addresses}")
        print(f"   Average addresses per transaction: {total_addresses/successful_python if successful_python > 0 else 0:.1f}")
        
        # Detailed transaction results
        for i, tx_info in enumerate(self.test_transactions, 1):
            tx_hash = tx_info["hash"]
            python_result = python_results.get(tx_hash, {})
            
            print(f"\n   Transaction {i}: {tx_hash[:10]}...")
            print(f"      Description: {tx_info['description']}")
            print(f"      Complexity Score: {tx_info['complexity_score']}")
            
            if python_result.get("success"):
                print(f"      Python: ✅ {python_result['address_count']} addresses")
                print(f"      ETH changes: {'Yes' if python_result['has_eth_changes'] else 'No'}")
                print(f"      Token changes: {'Yes' if python_result['has_token_changes'] else 'No'}")
            else:
                print(f"      Python: ❌ {python_result.get('error', 'Unknown error')}")
            
            print(f"      Rust: ⏳ Pending implementation")
        
        # Future comparison logic
        print(f"\n🔮 Future Rust vs Python Comparison:")
        print(f"   When Rust analysis is complete, this will compare:")
        print(f"   - Address-by-address ETH net changes (precision: 1e-15)")
        print(f"   - Token balance changes by contract address")
        print(f"   - Movement counts and directions")
        print(f"   - Overall consistency metrics")
        
        return {
            "python_success_rate": successful_python / len(self.test_transactions),
            "total_addresses": total_addresses,
            "rust_implemented": False
        }
    
    def generate_test_expectations(self):
        """Generate expected test results for future Rust validation."""
        print("\n📋 Generating Test Expectations for Rust Implementation")
        print("=" * 60)
        
        expectations = {}
        
        for tx_info in self.test_transactions:
            tx_hash = tx_info["hash"]
            python_result = self.results["python_analysis"].get(tx_hash, {})
            
            if python_result.get("success"):
                formatted = python_result["formatted_for_comparison"]
                
                # Create expected results for Rust to match
                expected = {
                    "transaction_hash": tx_hash,
                    "description": tx_info["description"],
                    "complexity_score": tx_info["complexity_score"],
                    "expected_address_count": len(formatted),
                    "expected_addresses": {},
                    "validation_criteria": {
                        "min_addresses": len(formatted),
                        "eth_precision_tolerance": 1e-15,
                        "must_have_eth_changes": python_result["has_eth_changes"],
                        "must_have_token_changes": python_result["has_token_changes"]
                    }
                }
                
                # Store expected state changes for each address
                for address, changes in formatted.items():
                    expected["expected_addresses"][address] = {
                        "eth_net": changes["eth_net"],
                        "token_count": changes["token_count"],
                        "eth_movements": changes["eth_movements"]
                    }
                
                expectations[tx_hash] = expected
        
        # Save expectations for future Rust testing
        expectations_file = Path(__file__).parent / "rust_validation_expectations.json"
        with open(expectations_file, "w") as f:
            json.dump(expectations, f, indent=2)
        
        print(f"✅ Saved test expectations to: {expectations_file}")
        print(f"   Expectations for {len(expectations)} transactions")
        print(f"   Total expected addresses: {sum(e['expected_address_count'] for e in expectations.values())}")
        
        return expectations
    
    def run_full_test(self):
        """Run the complete test suite."""
        print("🧪 Rust vs Python State Change Detection Test")
        print("=" * 60)
        print("Testing complex transactions with internal transfers and DeFi interactions")
        print(f"Test Transactions: {len(self.test_transactions)}")
        
        # Setup
        if not self.setup():
            return False
        
        # Run Python analysis
        self.analyze_with_python()
        
        # Run Rust analysis (placeholder)
        self.analyze_with_rust()
        
        # Compare results
        comparison_summary = self.compare_results()
        
        # Generate expectations for future testing
        expectations = self.generate_test_expectations()
        
        # Final summary
        print(f"\n🎯 TEST SUMMARY")
        print("=" * 60)
        print(f"Python Analysis Success Rate: {comparison_summary['python_success_rate']:.1%}")
        print(f"Total Addresses Analyzed: {comparison_summary['total_addresses']}")
        print(f"Rust Implementation: {'❌ Not yet available' if not comparison_summary['rust_implemented'] else '✅ Complete'}")
        
        if comparison_summary['python_success_rate'] == 1.0:
            print(f"\n✅ PYTHON ANALYSIS: All complex transactions processed successfully")
            print(f"   Ready for Rust comparison when dependencies are fixed")
            return True
        else:
            print(f"\n⚠️  PYTHON ANALYSIS: Some transactions failed")
            return False

def main():
    """Main test execution."""
    test = RustPythonComparisonTest()
    success = test.run_full_test()
    
    if success:
        print(f"\n🎉 TEST FRAMEWORK READY")
        print(f"   Python baseline established for 3 complex transactions")
        print(f"   Expectations generated for future Rust validation")
        print(f"   Run this test again when Rust compilation issues are resolved")
    else:
        print(f"\n❌ TEST SETUP FAILED")
        print(f"   Check Python analysis errors above")
    
    return success

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)