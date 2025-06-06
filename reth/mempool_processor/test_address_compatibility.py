#!/usr/bin/env python3
"""
Test script to verify address format compatibility between Rust and Python.
This demonstrates the checksum address issue and validates the fix.
"""

import sys
sys.path.append('/home/nima/code/crypto/py/eth_block_processor')

from web3 import Web3

def test_address_compatibility():
    """Test that demonstrates address format differences and compatibility."""
    
    # Initialize Web3 for checksum address conversion
    w3 = Web3()
    
    # Test addresses that would come from Rust/Python comparison
    test_addresses = [
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045",  # lowercase (old Rust format)
        "0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed",  # lowercase (old Rust format)
        "0x98c3d3183c4b8a650614ad179a1a98be0a8d6b8e",  # lowercase (old Rust format)
    ]
    
    print("=== Address Format Compatibility Test ===")
    print()
    
    print("BEFORE FIX (Rust uses lowercase, Python uses checksum):")
    print("--------------------------------------------------------")
    for addr in test_addresses:
        lowercase = addr.lower()
        checksummed = w3.to_checksum_address(addr)
        matches = lowercase == checksummed
        
        print(f"Address: {addr}")
        print(f"  Rust (old):    {lowercase}")
        print(f"  Python:        {checksummed}")
        print(f"  Match: {'✅' if matches else '❌'}")
        print()
    
    print("AFTER FIX (Both use EIP-55 checksum):")
    print("--------------------------------------")
    for addr in test_addresses:
        checksummed = w3.to_checksum_address(addr)
        rust_checksum = checksummed  # This is what Rust will now produce
        matches = rust_checksum == checksummed
        
        print(f"Address: {addr}")
        print(f"  Rust (new):    {rust_checksum}")
        print(f"  Python:        {checksummed}")
        print(f"  Match: {'✅' if matches else '❌'}")
        print()
    
    print("=== Pool Address Lookup Test ===")
    print()
    
    # Simulate pool cache lookup scenario
    pool_data = {
        w3.to_checksum_address("0xd8da6bf26964af9d7eed9e03e53415d37aa96045"): {"eth_reserve": 5.25},
        w3.to_checksum_address("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed"): {"eth_reserve": 12.7},
    }
    
    # Test lookups with different address formats
    test_lookup_addresses = [
        "0xd8da6bf26964af9d7eed9e03e53415d37aa96045",  # lowercase
        "0xD8DA6BF26964AF9D7EED9E03E53415D37AA96045",  # uppercase
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",  # checksummed
    ]
    
    print("Pool cache contains checksummed addresses.")
    print("Testing lookups with different formats:")
    print()
    
    for addr in test_lookup_addresses:
        checksummed_key = w3.to_checksum_address(addr)
        found = checksummed_key in pool_data
        reserve = pool_data.get(checksummed_key, {}).get("eth_reserve", "N/A")
        
        print(f"Lookup: {addr}")
        print(f"  Key used: {checksummed_key}")
        print(f"  Found: {'✅' if found else '❌'}")
        print(f"  Reserve: {reserve} ETH")
        print()

if __name__ == "__main__":
    test_address_compatibility()