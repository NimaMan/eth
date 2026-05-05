#!/usr/bin/env python3
"""
Example script demonstrating how to query stablecoin data using PyReth.

This shows the high-performance Rust-based implementation for querying
stablecoin balances and total supply directly from the blockchain.
"""


from eth_data.database.reth_chain_queries import (
    get_stablecoin_total_supply,
    get_stablecoin_balance,
    compare_top_stablecoins_supply,
    get_holder_all_stablecoin_balances
)


def main():
    print("\n" + "="*60)
    print("STABLECOIN QUERY EXAMPLES USING PYRETH")
    print("="*60)
    
    # Example 1: Get total supply of major stablecoins
    print("\n1. Total Supply of Major Stablecoins:")
    print("-" * 40)
    
    results = compare_top_stablecoins_supply()
    for token_name, data in results.items():
        if "error" not in data:
            print(f"  {token_name:6} Supply: {data['total_supply_formatted']:>20} {data['unit']}")
    
    # Example 2: Get specific stablecoin details
    print("\n2. Detailed USDC Information:")
    print("-" * 40)
    
    usdc_data = get_stablecoin_total_supply("USDC")
    print(f"  Token:          {usdc_data['name']}")
    print(f"  Address:        {usdc_data['address']}")
    print(f"  Total Supply:   {usdc_data['total_supply_formatted']}")
    print(f"  Decimals:       {usdc_data['decimals']}")
    print(f"  Unit:           {usdc_data['unit']}")
    print(f"  Block:          {usdc_data['block_number']:,}")
    
    # Example 3: Check stablecoin balance for a specific address
    print("\n3. Check Stablecoin Balance:")
    print("-" * 40)
    
    # Uniswap V3 USDC pool address
    pool_address = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
    
    for token in ["USDC", "USDT", "DAI"]:
        try:
            balance_data = get_stablecoin_balance(token, pool_address)
            if balance_data['balance'] > 0:
                print(f"  {token:6} Balance: {balance_data['balance_formatted']:>20}")
        except Exception as e:
            print(f"  {token:6} Error: {str(e)}")
    
    # Example 4: Get all non-zero stablecoin balances for an address
    print("\n4. All Stablecoin Balances for Binance 14:")
    print("-" * 40)
    
    binance_address = "0x28C6c06298d514Db089934071355E5743bf21d60"
    
    all_balances = get_holder_all_stablecoin_balances(
        binance_address, 
        only_non_zero=True
    )
    
    if all_balances:
        for token_name, balance_data in all_balances.items():
            print(f"  {token_name:10} {balance_data['balance_formatted']:>20} {balance_data['unit']}")
    else:
        print("  No non-zero stablecoin balances found")
    
    # Example 5: Query at specific block (recent block)
    print("\n5. Query at Recent Block:")
    print("-" * 40)
    
    # Use a recent block (current - 100)
    from pyreth import chain_query as pyreth_chain_query

    chain_query = pyreth_chain_query()
    current_block = chain_query.get_latest_block()
    recent_block = current_block - 100
    
    try:
        historical_data = get_stablecoin_total_supply("USDC", block_number=recent_block)
        print(f"  USDC Supply at block {recent_block:,}:")
        print(f"  {historical_data['total_supply_formatted']}")
    except Exception as e:
        print(f"  Error querying historical block: {e}")
    
    print("\n" + "="*60)
    print("EXAMPLES COMPLETED")
    print("="*60)


if __name__ == "__main__":
    main()
