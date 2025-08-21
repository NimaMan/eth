#!/usr/bin/env python3
"""
Analyze the liquidity removal transaction in detail
"""

import pyreth
import json

def analyze_liquidity_removal():
    """Analyze the undetected scam transaction"""
    
    processor = pyreth.TxProcessor()
    
    # The undetected scam transaction
    tx_hash = "0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966"
    pool_address = "0xd5113065d0dA0CD94F8c0Ba7B2Fa61d8A48AE404"
    
    print("LIQUIDITY REMOVAL ANALYSIS")
    print("=" * 60)
    print(f"TX: {tx_hash}")
    print(f"Pool: {pool_address}")
    print()
    
    # Process the transaction
    tx = processor.process_transaction(tx_hash)
    
    print(f"Block: {tx.block_number}")
    print(f"From: {tx.from_address}")
    print(f"To: {tx.to_address}")
    print(f"Status: {'Success' if tx.status == '1' else 'Failed'}")
    print()
    
    # Analyze ERC20 transfers
    print("ERC20 TRANSFERS:")
    print("-" * 40)
    for i, transfer in enumerate(tx.erc20_transfers):
        print(f"\nTransfer {i+1}:")
        print(f"  Token: {transfer['token_address']}")
        print(f"  From: {transfer['from_address']}")
        print(f"  To: {transfer['to_address']}")
        print(f"  Amount: {transfer['amount']}")
        
        # Check if it's from the pool
        if transfer['from_address'].lower() == pool_address.lower():
            print("  ⚠️ TRANSFER FROM POOL")
    
    # Analyze Uniswap events
    print("\n\nUNISWAP EVENTS:")
    print("-" * 40)
    if len(tx.uniswap_v2_swaps) == 0:
        print("No Uniswap V2 swap events found")
        
        # Check for other DEX events
        print("\nChecking other DEX events...")
        print(f"Uniswap V3 Swaps: {len(tx.uniswap_v3_swaps)}")
        
    # Analyze internal transactions
    print("\n\nINTERNAL TRANSACTIONS:")
    print("-" * 40)
    eth_transfers = []
    for itx in tx.internal_transactions:
        if float(itx['value']) > 0:
            eth_transfers.append(itx)
            
    print(f"Total internal transactions: {len(tx.internal_transactions)}")
    print(f"ETH transfers (value > 0): {len(eth_transfers)}")
    
    for i, itx in enumerate(eth_transfers[:5]):  # Show first 5
        print(f"\nETH Transfer {i+1}:")
        print(f"  From: {itx['from_address']}")
        print(f"  To: {itx['to_address']}")
        print(f"  Value: {float(itx['value']) / 10**18:.6f} ETH")
        print(f"  Depth: {itx['depth']}")
        
        # Check if it's from the pool
        if itx['from_address'].lower() == pool_address.lower():
            print(f"  ⚠️ ETH REMOVED FROM POOL: {float(itx['value']) / 10**18:.6f} ETH")
    
    # Look for specific function calls
    print("\n\nTRANSACTION TYPE ANALYSIS:")
    print("-" * 40)
    print(f"Transaction Type: {tx.txn_type}")
    print(f"Actions: {tx.actions}")
    
    # Check input data
    input_data = tx.input
    print(f"\nInput Data (first 10 bytes): {input_data[:10] if len(input_data) > 10 else input_data}")
    
    # Common liquidity removal function signatures
    remove_liquidity_sigs = [
        "0xe8e33700",  # addLiquidity
        "0xf305d719",  # addLiquidityETH
        "0x02751cec",  # removeLiquidityETH
        "0xbaa2abde",  # removeLiquidityETHWithPermit
        "0x2195995c",  # removeLiquidityWithPermit
        "0xaf2979eb",  # removeLiquidityETHSupportingFeeOnTransferTokens
        "0x5b0d5984",  # removeLiquidityETHWithPermitSupportingFeeOnTransferTokens
    ]
    
    for sig in remove_liquidity_sigs:
        if input_data.startswith(sig):
            print(f"  ⚠️ LIQUIDITY FUNCTION DETECTED: {sig}")
            break
    
    # Summary
    print("\n\n" + "=" * 60)
    print("SUMMARY:")
    
    # Calculate total ETH removed
    total_eth_removed = sum(float(itx['value']) / 10**18 for itx in eth_transfers 
                           if itx['from_address'].lower() == pool_address.lower())
    
    if total_eth_removed > 0:
        print(f"⚠️ LIQUIDITY REMOVAL DETECTED!")
        print(f"   Total ETH removed from pool: {total_eth_removed:.6f} ETH")
    else:
        print("No direct ETH removal from pool detected in internal transactions")
        print("Checking ERC20 transfers from pool...")
        
        pool_transfers = [t for t in tx.erc20_transfers 
                         if t['from_address'].lower() == pool_address.lower()]
        if pool_transfers:
            print(f"  Found {len(pool_transfers)} token transfers from pool")

if __name__ == "__main__":
    analyze_liquidity_removal()