#!/usr/bin/env python3
"""
Example: Batch operations for efficient querying
"""

from pyreth import chain_query as pyreth_chain_query
import time

def main():
    # Create PyReth instance
    query = pyreth_chain_query()
    
    print("=== Chain Query: Batch Operations ===\n")
    
    # Test addresses
    addresses = [
        ("Vitalik", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
        ("Ethereum Foundation", "0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe"),
        ("Binance", "0xF977814e90dA44bFA03b6295A0616a897441aceC"),
    ]
    
    # Token addresses
    usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    usdt = "0xdAC17F958D2ee523a2206206994597C13D831ec7"
    weth = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
    
    # 1. Batch ETH balance query
    print("1. Batch ETH Balance Query")
    print("-" * 40)
    
    start_time = time.time()
    addr_list = [addr for _, addr in addresses]
    balances = query.batch_get_eth_balances(addr_list)
    batch_time = time.time() - start_time
    
    for (name, addr), balance in zip(addresses, balances):
        eth_amount = int(balance) / 10**18
        print(f"{name:20} {eth_amount:.4f} ETH")
    
    print(f"\nBatch query time: {batch_time:.3f}s")
    print()
    
    # 2. Batch token balance query
    print("2. Batch Token Balance Query")
    print("-" * 40)
    
    # Create (token, holder) pairs
    token_requests = []
    request_labels = []
    
    for name, addr in addresses:
        for token_name, token_addr in [("USDC", usdc), ("USDT", usdt)]:
            token_requests.append((token_addr, addr))
            request_labels.append(f"{name} - {token_name}")
    
    start_time = time.time()
    token_balances = query.batch_get_token_balances(token_requests)
    batch_time = time.time() - start_time
    
    for label, balance in zip(request_labels, token_balances):
        if int(balance) > 0:
            # Assuming 6 decimals for USDC/USDT
            amount = int(balance) / 10**6
            print(f"{label:30} ${amount:,.2f}")
    
    print(f"\nBatch query time: {batch_time:.3f}s")
    print()
    
    # 3. Batch contract check
    print("3. Batch Contract Detection")
    print("-" * 40)
    
    check_addresses = [
        vitalik := "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
        usdc,
        usdt,
        weth,
        "0xE592427A0AEce92De3Edee1F18E0157C05861564",  # Uniswap V3 Router
    ]
    
    check_labels = [
        "Vitalik (EOA)",
        "USDC",
        "USDT", 
        "WETH",
        "Uniswap V3 Router",
    ]
    
    start_time = time.time()
    contract_results = query.batch_is_contract(check_addresses)
    batch_time = time.time() - start_time
    
    for label, addr in zip(check_labels, check_addresses):
        # Try both original and lowercase versions
        is_contract = contract_results.get(addr, contract_results.get(addr.lower(), False))
        print(f"{label:20} {'Contract' if is_contract else 'EOA'}")
    
    print(f"\nBatch query time: {batch_time:.3f}s")
    print()
    
    # 4. Compare individual vs batch performance
    print("4. Performance Comparison")
    print("-" * 40)
    
    # Individual queries
    start_time = time.time()
    for _, addr in addresses[:3]:
        _ = query.get_eth_balance(addr)
    individual_time = time.time() - start_time
    
    # Batch query
    start_time = time.time()
    _ = query.batch_get_eth_balances([addr for _, addr in addresses[:3]])
    batch_time = time.time() - start_time
    
    print(f"Individual queries (3 addresses): {individual_time:.3f}s")
    print(f"Batch query (3 addresses): {batch_time:.3f}s")
    print(f"Speedup: {individual_time/batch_time:.1f}x")
    print()
    
    # 5. Get complete balances for multiple tokens
    print("5. Complete Balance Snapshot")
    print("-" * 40)
    
    vitalik_addr = addresses[0][1]
    tokens = [usdc, usdt, weth]
    
    complete = query.get_complete_balances(vitalik_addr, tokens)
    
    print(f"Complete balances for {complete.address}")
    print(f"  Block: {complete.block_number}")
    print(f"  ETH: {int(complete.eth_balance) / 10**18:.4f} ETH")
    print(f"  Token balances: {complete.token_balances()}")
    
    print("\n✅ Batch operations examples complete!")

if __name__ == "__main__":
    main()