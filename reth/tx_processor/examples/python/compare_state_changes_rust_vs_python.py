#!/usr/bin/env python3
"""Compare Rust and Python state changes for transaction validation."""

import json
import sys
from typing import Dict, Any, Tuple

def compare_eth_changes(rust_data: Dict, python_data: Dict) -> Tuple[bool, str]:
    """Compare ETH net changes between Rust and Python."""
    differences = []
    
    # Get all unique addresses from both (addresses should already be checksum)
    rust_addresses = set(rust_data.keys())
    python_addresses = set(python_data.get('state_changes', {}).keys())
    all_addresses = rust_addresses | python_addresses
    
    for addr in sorted(all_addresses):
        rust_addr = addr if addr in rust_data else None
        python_addr = addr if addr in python_data.get('state_changes', {}) else None
        
        rust_eth = rust_data.get(rust_addr, {}).get('eth_net', 0)
        python_eth = python_data.get('state_changes', {}).get(python_addr, {}).get('eth_net', 0)
        
        # Skip if both are 0 or very small
        if abs(rust_eth) < 1e-10 and abs(python_eth) < 1e-10:
            continue
            
        # Compare with small tolerance for floating point
        if abs(rust_eth - python_eth) > 1e-9:
            differences.append(f"  {addr}: Rust={rust_eth:.18f}, Python={python_eth:.18f}, Diff={rust_eth - python_eth:.18f}")
    
    return len(differences) == 0, "\n".join(differences)

def compare_token_changes(rust_data: Dict, python_data: Dict) -> Tuple[bool, str]:
    """Compare token net changes between Rust and Python."""
    differences = []
    
    # Token addresses to names (using checksum addresses)
    token_map = {
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": "USDC",
        "0xdAC17F958D2ee523a2206206994597C13D831ec7": "USDT",
        "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2": "WETH"
    }
    
    # Get all addresses (addresses should already be checksum)
    all_addresses = set()
    all_addresses.update(rust_data.keys())
    all_addresses.update(python_data.get('state_changes', {}).keys())
    
    for addr in sorted(all_addresses):
        rust_addr = addr if addr in rust_data else None
        python_addr = addr if addr in python_data.get('state_changes', {}) else None
        
        rust_tokens = rust_data.get(rust_addr, {}).get('token_net', {})
        python_tokens = python_data.get('state_changes', {}).get(python_addr, {}).get('token_net', {})
        
        # Get all token symbols
        all_tokens = set(rust_tokens.keys()) | set(python_tokens.keys())
        
        for token in sorted(all_tokens):
            rust_amount = rust_tokens.get(token, 0)
            python_amount = python_tokens.get(token, 0)
            
            # Skip if both are 0
            if rust_amount == 0 and python_amount == 0:
                continue
                
            # Compare with small tolerance
            if abs(rust_amount - python_amount) > 0.000001:
                differences.append(f"  {addr} - {token}: Rust={rust_amount}, Python={python_amount}, Diff={rust_amount - python_amount}")
    
    return len(differences) == 0, "\n".join(differences)

def main():
    # Load Rust state changes
    with open('rust_state_changes_clean.json', 'r') as f:
        rust_data = json.load(f)
    
    # Load Python response
    with open('python_full_response.json', 'r') as f:
        python_response = json.load(f)
    
    python_data = python_response['processed_transaction']
    
    print("🔍 Comparing Rust and Python State Changes")
    print("=" * 60)
    print(f"Transaction: 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")
    print(f"Block: {python_data['block_number']}")
    print(f"Gas Used - Rust: {rust_data.get('gas_used', 'N/A')}, Python: {python_data['fees']['gas_used']}")
    print()
    
    # Compare ETH changes
    print("📊 ETH Net Changes Comparison:")
    eth_match, eth_diff = compare_eth_changes(rust_data, python_data)
    if eth_match:
        print("✅ ETH changes match!")
    else:
        print("❌ ETH changes differ:")
        print(eth_diff)
    print()
    
    # Compare token changes
    print("🪙 Token Net Changes Comparison:")
    token_match, token_diff = compare_token_changes(rust_data, python_data)
    if token_match:
        print("✅ Token changes match!")
    else:
        print("❌ Token changes differ:")
        print(token_diff)
    print()
    
    # Summary of internal transfers
    rust_internal_count = 5  # From the fast_tx_hash_simulator output
    python_internal_count = len(python_data.get('internal_transactions', []))
    print(f"🔄 Internal Transfers - Rust: {rust_internal_count}, Python: {python_internal_count}")
    
    # Summary of token transfers
    rust_token_count = 8  # ERC20 transfers from state changes
    python_token_count = len(python_data.get('erc20_transfers', []))
    print(f"💰 ERC20 Transfers - Rust: {rust_token_count}, Python: {python_token_count}")
    
    # Overall verdict
    print()
    if eth_match and token_match:
        print("✅ VALIDATION PASSED: Rust and Python implementations match!")
    else:
        print("❌ VALIDATION FAILED: Differences found between implementations")
        
    # Debug: Print raw data for manual inspection
    print("\n" + "=" * 60)
    print("🔍 Raw Data for Manual Inspection:")
    print("\nPython State Changes:")
    for addr, changes in python_data.get('state_changes', {}).items():
        print(f"\n{addr}:")
        print(f"  eth_net: {changes.get('eth_net', 0)}")
        print(f"  token_net: {changes.get('token_net', {})}")

if __name__ == "__main__":
    main()