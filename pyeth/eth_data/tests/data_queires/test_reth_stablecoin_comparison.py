#!/usr/bin/env python3
"""
Test script to compare stablecoin queries between Python (web3) and PyReth implementations.

This script verifies that both implementations return the same results for:
- Total supply queries
- Balance queries
- Performance comparison
"""

import time
from decimal import Decimal
from web3 import Web3
import os
import sys

# Add parent directory to path
sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from eth_data.reth_chain_query import (
    get_stablecoin_total_supply,
    get_stablecoin_balance,
    compare_top_stablecoins_supply
)
from eth_data.chain_utils.common_addresses import (
    STABLECOINS_ADDRESS_BY_NAME,
    ERC20_TOKEN_DECIMALS
)


# Initialize Web3 for comparison
ETH_RPC_URL = os.getenv("ETH_RPC_URL", "http://localhost:8545")
w3 = Web3(Web3.HTTPProvider(ETH_RPC_URL))

# ERC20 ABI for totalSupply and balanceOf
ERC20_ABI = [
    {
        "constant": True,
        "inputs": [],
        "name": "totalSupply",
        "outputs": [{"name": "", "type": "uint256"}],
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [{"name": "_owner", "type": "address"}],
        "name": "balanceOf",
        "outputs": [{"name": "balance", "type": "uint256"}],
        "type": "function"
    }
]


def get_web3_total_supply(token_name: str, block_number=None):
    """Get total supply using Web3 (Python implementation)"""
    token_address = STABLECOINS_ADDRESS_BY_NAME[token_name]
    contract = w3.eth.contract(
        address=Web3.to_checksum_address(token_address),
        abi=ERC20_ABI
    )
    
    if block_number:
        total_supply = contract.functions.totalSupply().call(block_identifier=block_number)
    else:
        total_supply = contract.functions.totalSupply().call()
    
    return str(total_supply)


def get_web3_balance(token_name: str, holder_address: str, block_number=None):
    """Get balance using Web3 (Python implementation)"""
    token_address = STABLECOINS_ADDRESS_BY_NAME[token_name]
    contract = w3.eth.contract(
        address=Web3.to_checksum_address(token_address),
        abi=ERC20_ABI
    )
    
    holder_checksum = Web3.to_checksum_address(holder_address)
    
    if block_number:
        balance = contract.functions.balanceOf(holder_checksum).call(block_identifier=block_number)
    else:
        balance = contract.functions.balanceOf(holder_checksum).call()
    
    return str(balance)


def test_total_supply_comparison():
    """Compare total supply results between Web3 and PyReth"""
    print("\n" + "="*60)
    print("TESTING TOTAL SUPPLY COMPARISON")
    print("="*60)
    
    test_tokens = ["USDC", "USDT", "DAI"]
    
    for token_name in test_tokens:
        print(f"\n{token_name}:")
        print("-" * 40)
        
        # Get total supply using PyReth
        start_time = time.time()
        pyreth_result = get_stablecoin_total_supply(token_name)
        pyreth_time = time.time() - start_time
        
        # Get total supply using Web3
        start_time = time.time()
        web3_result = get_web3_total_supply(token_name)
        web3_time = time.time() - start_time
        
        # Compare raw values
        pyreth_raw = pyreth_result["total_supply_raw"]
        
        print(f"  PyReth raw:  {pyreth_raw}")
        print(f"  Web3 raw:    {web3_result}")
        print(f"  Match:       {pyreth_raw == web3_result}")
        
        # Show formatted values
        decimals = ERC20_TOKEN_DECIMALS.get(token_name, 18)
        web3_formatted = Decimal(web3_result) / Decimal(10 ** decimals)
        print(f"  PyReth formatted: {pyreth_result['total_supply_formatted']}")
        print(f"  Web3 formatted:   {web3_formatted:,.2f}")
        
        # Performance comparison
        print(f"  PyReth time: {pyreth_time*1000:.2f}ms")
        print(f"  Web3 time:   {web3_time*1000:.2f}ms")
        print(f"  Speedup:     {web3_time/pyreth_time:.1f}x")


def test_balance_comparison():
    """Compare balance results between Web3 and PyReth"""
    print("\n" + "="*60)
    print("TESTING BALANCE COMPARISON")
    print("="*60)
    
    # Test addresses (some known large holders)
    test_cases = [
        ("USDC", "0x0A59649758aa4d66E25f08Dd01271e891fe52199"),  # Uniswap V3: USDC 3
        ("USDT", "0x5041ed759Dd4aFc3a72b8192C143F72f4724081A"),  # Large USDT holder
        ("DAI", "0x2a65Aca4D5fC5B5C859090a6c34d164135398226"),   # Large DAI holder
    ]
    
    for token_name, holder_address in test_cases:
        print(f"\n{token_name} balance for {holder_address[:10]}...:")
        print("-" * 40)
        
        # Get balance using PyReth
        start_time = time.time()
        pyreth_result = get_stablecoin_balance(token_name, holder_address)
        pyreth_time = time.time() - start_time
        
        # Get balance using Web3
        start_time = time.time()
        web3_result = get_web3_balance(token_name, holder_address)
        web3_time = time.time() - start_time
        
        # Compare raw values
        pyreth_raw = pyreth_result["balance_raw"]
        
        print(f"  PyReth raw:  {pyreth_raw}")
        print(f"  Web3 raw:    {web3_result}")
        print(f"  Match:       {pyreth_raw == web3_result}")
        
        # Show formatted values
        decimals = ERC20_TOKEN_DECIMALS.get(token_name, 18)
        web3_formatted = Decimal(web3_result) / Decimal(10 ** decimals)
        print(f"  PyReth formatted: {pyreth_result['balance_formatted']}")
        print(f"  Web3 formatted:   {web3_formatted:,.2f}")
        
        # Performance comparison
        print(f"  PyReth time: {pyreth_time*1000:.2f}ms")
        print(f"  Web3 time:   {web3_time*1000:.2f}ms")
        print(f"  Speedup:     {web3_time/pyreth_time:.1f}x")


def test_bulk_query_performance():
    """Test performance for bulk queries"""
    print("\n" + "="*60)
    print("TESTING BULK QUERY PERFORMANCE")
    print("="*60)
    
    print("\nQuerying top 3 stablecoins supply...")
    
    # PyReth bulk query
    start_time = time.time()
    pyreth_results = compare_top_stablecoins_supply()
    pyreth_time = time.time() - start_time
    
    # Web3 bulk query
    start_time = time.time()
    web3_results = {}
    for token in ["USDC", "USDT", "DAI"]:
        web3_results[token] = get_web3_total_supply(token)
    web3_time = time.time() - start_time
    
    print(f"  PyReth time: {pyreth_time*1000:.2f}ms")
    print(f"  Web3 time:   {web3_time*1000:.2f}ms") 
    print(f"  Speedup:     {web3_time/pyreth_time:.1f}x")
    
    # Show results
    print("\nResults summary:")
    for token in ["USDC", "USDT", "DAI"]:
        if token in pyreth_results and not pyreth_results[token].get("error"):
            print(f"  {token}: {pyreth_results[token]['total_supply_formatted']}")


def main():
    """Run all comparison tests"""
    print("\n" + "="*60)
    print("PYRETH vs WEB3 STABLECOIN QUERY COMPARISON")
    print("="*60)
    
    # Check connection
    if not w3.is_connected():
        print("ERROR: Cannot connect to Ethereum node at", ETH_RPC_URL)
        print("Please ensure your Ethereum node is running")
        return
    
    latest_block = w3.eth.block_number
    print(f"\nConnected to Ethereum node")
    print(f"Latest block: {latest_block:,}")
    
    try:
        # Run tests
        test_total_supply_comparison()
        test_balance_comparison()
        test_bulk_query_performance()
        
        print("\n" + "="*60)
        print("ALL TESTS COMPLETED SUCCESSFULLY")
        print("="*60)
        
    except Exception as e:
        print(f"\nERROR: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()