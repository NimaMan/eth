#!/usr/bin/env python3
"""
Test Runner for Fund Flow Network Analysis
Runs tests for transaction 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae
"""
import sys
import os
import unittest

# Add the project root to the Python path
project_root = os.path.join(os.path.dirname(__file__), '..', '..')
sys.path.insert(0, project_root)

def run_network_tests():
    """Run all network-related tests"""
    
    print("=" * 80)
    print("🔍 FUND FLOW NETWORK ANALYSIS TESTS")
    print("=" * 80)
    print(f"Testing transaction: 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")
    print("Expected network: 5 nodes, 4 edges (excluding intermediaries)")
    print("=" * 80)
    
    # Discover and run tests
    loader = unittest.TestLoader()
    start_dir = os.path.dirname(__file__)
    suite = loader.discover(start_dir, pattern='test_*.py')
    
    # Run tests with detailed output
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    print("\n" + "=" * 80)
    if result.wasSuccessful():
        print("✅ ALL NETWORK TESTS PASSED!")
        print("The fund flow network implementation is working correctly.")
    else:
        print("❌ SOME TESTS FAILED!")
        print(f"Failures: {len(result.failures)}")
        print(f"Errors: {len(result.errors)}")
        
        if result.failures:
            print("\n🔥 FAILURES:")
            for test, traceback in result.failures:
                print(f"- {test}: {traceback}")
                
        if result.errors:
            print("\n💥 ERRORS:")
            for test, traceback in result.errors:
                print(f"- {test}: {traceback}")
    
    print("=" * 80)
    return result.wasSuccessful()

if __name__ == '__main__':
    success = run_network_tests()
    sys.exit(0 if success else 1)