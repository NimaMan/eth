#!/usr/bin/env python3
"""
Basic pyreth usage examples
Shows how to use ChainQuery, TxProcessor, and Simulator
"""

import pyreth
import time

def main():
    print("=" * 60)
    print("PyReth Basic Usage Examples")
    print("=" * 60)
    
    # Create the PyReth instance
    py_reth = pyreth.PyReth()
    
    # 1. ChainQuery - Direct database access
    print("\n1. ChainQuery - Direct Database Access")
    print("-" * 40)
    
    query = py_reth.chain_query()
    
    # Get ETH balance
    vitalik = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
    balance_wei = query.get_balance(vitalik)
    balance_eth = int(balance_wei) / 1e18
    print(f"Vitalik's balance: {balance_eth:.4f} ETH")
    
    # Get USDC total supply
    usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    supply = query.get_token_total_supply(usdc)
    decimals = query.get_token_decimals(usdc)
    supply_human = int(supply) / (10 ** decimals)
    print(f"USDC Total Supply: ${supply_human:,.2f}")
    
    # 2. TxProcessor - Process transactions
    print("\n2. TxProcessor - Transaction Processing")
    print("-" * 40)
    
    processor = py_reth.tx_processor()
    
    # Process a known Uniswap swap transaction
    tx_hash = "0x5c89f223fe19593cf85319dcdd3f6de618e770d958f72ea123d36db2fdf5d46e"
    
    start = time.time()
    tx = processor.process_transaction(tx_hash)
    elapsed = time.time() - start
    
    print(f"Processed tx in {elapsed*1000:.2f}ms")
    print(f"  Hash: {tx.hash[:10]}...")
    print(f"  Block: {tx.block_number}")
    print(f"  Type: {tx.txn_type}")
    print(f"  From: {tx.from_address[:10]}...")
    print(f"  To: {tx.to_address[:10]}...")
    
    if tx.uniswap_v2_swaps:
        print(f"  Uniswap V2 Swaps: {len(tx.uniswap_v2_swaps)}")
    if tx.uniswap_v3_swaps:
        print(f"  Uniswap V3 Swaps: {len(tx.uniswap_v3_swaps)}")
    if tx.erc20_transfers:
        print(f"  ERC20 Transfers: {len(tx.erc20_transfers)}")
    
    # 3. Simulator - Simulate transactions
    print("\n3. Simulator - Transaction Simulation")
    print("-" * 40)
    
    simulator = py_reth.simulator()
    
    # Simulate a simple ETH transfer
    tx_request = {
        "from": "0xF977814e90dA44bFA03b6295A0616a897441aceC",  # Binance
        "to": vitalik,
        "value": "1000000000000000000",  # 1 ETH
        "gas": 21000
    }
    
    result = simulator.simulate_transaction(tx_request)
    print(f"ETH Transfer Simulation:")
    print(f"  Status: {'Success' if result.status else 'Failed'}")
    print(f"  From: {result.from_address[:10]}...")
    print(f"  To: {result.to_address[:10]}...")
    print(f"  Value: {int(result.value) / 1e18:.4f} ETH")
    
    print("\n" + "=" * 60)
    print("✅ All examples completed successfully!")

if __name__ == "__main__":
    main()