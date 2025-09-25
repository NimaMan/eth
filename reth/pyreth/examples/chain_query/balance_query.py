#!/usr/bin/env python3
"""
Example: Query ETH and token balances using ChainQuery
"""

import pyreth

def main():
    # Create PyReth instance with shared database
    reth = pyreth.PyReth()
    
    # Get chain query interface
    query = reth.chain_query()
    
    print("=== Chain Query: Balance Operations ===\n")
    
    # Get latest block
    latest_block = query.get_latest_block()
    print(f"Latest block: {latest_block}")
    print()
    
    # Test addresses
    vitalik = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
    usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    usdt = "0xdAC17F958D2ee523a2206206994597C13D831ec7"
    
    # 1. Get ETH balance
    print("1. ETH Balance Query")
    print("-" * 40)
    eth_balance = query.get_eth_balance(vitalik)
    print(f"Vitalik's ETH balance: {eth_balance} wei")
    print(f"  = {int(eth_balance) / 10**18:.4f} ETH")
    print()
    
    # 2. Get account info
    print("2. Account Information")
    print("-" * 40)
    account = query.get_account(vitalik)
    print(f"Address: {account.address}")
    print(f"Nonce: {account.nonce}")
    print(f"Balance: {account.balance} wei")
    print(f"Is contract: {account.is_contract}")
    print()
    
    # 3. Get token balances
    print("3. Token Balances")
    print("-" * 40)
    usdc_balance = query.get_token_balance(usdc, vitalik)
    usdt_balance = query.get_token_balance(usdt, vitalik)
    
    print(f"USDC balance: {usdc_balance} (smallest unit)")
    if int(usdc_balance) > 0:
        print(f"  = ${int(usdc_balance) / 10**6:.2f}")
    
    print(f"USDT balance: {usdt_balance} (smallest unit)")
    if int(usdt_balance) > 0:
        print(f"  = ${int(usdt_balance) / 10**6:.2f}")
    print()
    
    # 4. Check if addresses are contracts
    print("4. Contract Detection")
    print("-" * 40)
    addresses_to_check = {
        "Vitalik (EOA)": vitalik,
        "USDC (Contract)": usdc,
        "USDT (Contract)": usdt,
    }
    
    for name, addr in addresses_to_check.items():
        is_contract = query.is_contract(addr)
        print(f"{name}: {'Contract' if is_contract else 'EOA'}")
    print()
    
    # 5. Get portfolio (ETH + multiple tokens)
    print("5. Portfolio Query")
    print("-" * 40)
    tokens = [usdc, usdt]
    portfolio = query.get_portfolio(vitalik, tokens)
    
    print(f"Portfolio for {portfolio.address}")
    print(f"  Block: {portfolio.block_number}")
    print(f"  ETH: {portfolio.eth_balance} wei")
    print(f"  Token balances: {portfolio.token_balances()}")
    print()
    
    # 6. Historical balance (if available)
    print("6. Historical Balance")
    print("-" * 40)
    historical_block = latest_block - 1000  # 1000 blocks ago
    
    try:
        historical_balance = query.get_eth_balance(vitalik, historical_block)
        current_balance = query.get_eth_balance(vitalik)
        
        print(f"Balance at block {historical_block}: {historical_balance} wei")
        print(f"Current balance: {current_balance} wei")
        
        diff = int(current_balance) - int(historical_balance)
        if diff > 0:
            print(f"Change: +{diff} wei (increased)")
        elif diff < 0:
            print(f"Change: {diff} wei (decreased)")
        else:
            print("Change: 0 wei (unchanged)")
    except Exception as e:
        print(f"Could not get historical balance: {e}")
    
    print("\n✅ Balance query examples complete!")

if __name__ == "__main__":
    main()