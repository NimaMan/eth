#!/usr/bin/env python3
"""
ChainQuery Demo

Demonstrates using ChainQuery for direct database queries without simulation.
ChainQuery provides efficient access to blockchain state.
"""

import ethtx
import time

def format_wei(wei_str, decimals=18):
    """Format wei to human readable format"""
    wei = int(wei_str)
    if wei == 0:
        return "0"
    value = wei / (10 ** decimals)
    if value < 0.01:
        return f"{value:.8f}"
    elif value < 1:
        return f"{value:.4f}"
    else:
        return f"{value:,.2f}"

def main():
    print("="*60)
    print("ChainQuery Demo - Direct Database Access")
    print("="*60)
    
    # Initialize ChainQuery
    print("\nInitializing ChainQuery...")
    query = ethtx.ChainQuery()
    print(f"Connected to: {query}")
    
    # Get latest block
    latest_block = query.get_latest_block()
    print(f"\nLatest block: {latest_block:,}")
    
    # Test addresses
    addresses = {
        "Vitalik": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
        "Binance 14": "0x28C6c06298d514Db089934071355E5743bf21d60",
        "Uniswap V2 Router": "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
    }
    
    # Well-known tokens
    tokens = {
        "USDC": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "USDT": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        "WETH": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    }
    
    print("\n" + "="*60)
    print("ACCOUNT QUERIES")
    print("="*60)
    
    for name, address in addresses.items():
        print(f"\n{name} ({address[:10]}...)")
        
        # Get account info
        start = time.time()
        info = query.get_account_info(address)
        query_time = (time.time() - start) * 1000
        
        print(f"  Balance: {format_wei(info['balance'])} ETH")
        print(f"  Nonce: {info['nonce']}")
        print(f"  Is Contract: {info['has_code']}")
        print(f"  Query time: {query_time:.2f}ms")
    
    print("\n" + "="*60)
    print("TOKEN BALANCES")
    print("="*60)
    
    # Check Vitalik's token balances
    vitalik = addresses["Vitalik"]
    print(f"\nToken balances for Vitalik:")
    
    for token_name, token_addr in tokens.items():
        start = time.time()
        balance = query.get_token_balance(token_addr, vitalik)
        query_time = (time.time() - start) * 1000
        
        # Get decimals (USDC/USDT have 6, others 18)
        decimals = 6 if token_name in ["USDC", "USDT"] else 18
        
        formatted = format_wei(balance, decimals)
        if formatted != "0":
            print(f"  {token_name}: {formatted} ({query_time:.2f}ms)")
        else:
            print(f"  {token_name}: 0 ({query_time:.2f}ms)")
    
    print("\n" + "="*60)
    print("TOKEN INFORMATION")
    print("="*60)
    
    # Get token info
    for token_name, token_addr in tokens.items():
        print(f"\n{token_name}:")
        
        # Total supply
        start = time.time()
        supply = query.get_total_supply(token_addr)
        query_time = (time.time() - start) * 1000
        
        decimals = query.get_decimals(token_addr)
        formatted_supply = format_wei(supply, decimals)
        
        print(f"  Total Supply: {formatted_supply}")
        print(f"  Decimals: {decimals}")
        print(f"  Query time: {query_time:.2f}ms")
    
    print("\n" + "="*60)
    print("STORAGE QUERIES")
    print("="*60)
    
    # Read storage slots
    print("\nReading USDC storage slots:")
    usdc = tokens["USDC"]
    
    # Slot 0 - usually balanceOf mapping for ERC20
    slot0 = query.get_storage_at(usdc, "0")
    print(f"  Slot 0: {slot0}")
    
    # Slot 2 - often total supply
    slot2 = query.get_storage_at(usdc, "2")
    print(f"  Slot 2 (total supply): {format_wei(slot2, 6)} USDC")
    
    print("\n" + "="*60)
    print("ALLOWANCE QUERIES")
    print("="*60)
    
    # Check allowances (example)
    owner = vitalik
    spender = addresses["Uniswap V2 Router"]
    
    print(f"\nChecking allowances from Vitalik to Uniswap Router:")
    for token_name, token_addr in tokens.items():
        allowance = query.get_allowance(token_addr, owner, spender)
        if int(allowance) > 0:
            decimals = 6 if token_name in ["USDC", "USDT"] else 18
            print(f"  {token_name}: {format_wei(allowance, decimals)}")
    
    print("\n" + "="*60)
    print("HISTORICAL QUERIES")
    print("="*60)
    
    # Query at different blocks
    blocks_back = [10, 100, 1000]
    test_address = addresses["Binance 14"]
    
    print(f"\nBalance history for Binance 14:")
    for back in blocks_back:
        block = latest_block - back
        balance = query.get_balance(test_address, block)
        print(f"  Block {block:,} (-{back:4}): {format_wei(balance)} ETH")
    
    print("\n" + "="*60)
    print("PERFORMANCE SUMMARY")
    print("="*60)
    
    # Batch query test
    print("\nBatch query performance test:")
    
    start = time.time()
    for _ in range(10):
        query.get_balance(vitalik)
    balance_time = (time.time() - start) / 10 * 1000
    
    start = time.time()
    for _ in range(10):
        query.get_nonce(vitalik)
    nonce_time = (time.time() - start) / 10 * 1000
    
    start = time.time()
    for _ in range(10):
        query.has_code(vitalik)
    code_time = (time.time() - start) / 10 * 1000
    
    print(f"  Average balance query: {balance_time:.2f}ms")
    print(f"  Average nonce query: {nonce_time:.2f}ms")
    print(f"  Average code check: {code_time:.2f}ms")
    
    print("\n✅ ChainQuery demo completed successfully!")

if __name__ == "__main__":
    main()