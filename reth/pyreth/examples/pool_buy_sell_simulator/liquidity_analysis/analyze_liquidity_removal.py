#!/usr/bin/env python3
"""
Analyze Liquidity Removal Transactions

This example shows how to detect and analyze liquidity removal from DEX pools,
which can indicate rug pulls or legitimate liquidity management.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))))

import pyreth
import json


def analyze_liquidity_removal():
    """Analyze a liquidity removal transaction in detail"""
    
    # Create PyReth instance with proper singleton pattern
    py_reth = pyreth.PyReth()
    processor = py_reth.tx_processor()
    
    # The liquidity removal transaction to analyze
    tx_hash = "0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966"
    pool_address = "0xd5113065d0dA0CD94F8c0Ba7B2Fa61d8A48AE404"
    
    print("LIQUIDITY REMOVAL ANALYSIS")
    print("=" * 60)
    print(f"TX: {tx_hash}")
    print(f"Pool: {pool_address}")
    print()
    
    try:
        # Process the transaction
        tx = processor.process_transaction_from_hash_with_simulation(tx_hash)
        
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
            
            # Handle transfer as dict or object
            if isinstance(transfer, dict):
                token_addr = transfer.get('token_address', 'Unknown')
                from_addr = transfer.get('from_address', 'Unknown')
                to_addr = transfer.get('to_address', 'Unknown')
                amount = transfer.get('amount', 0)
            else:
                token_addr = transfer.token_address
                from_addr = transfer.from_address
                to_addr = transfer.to_address
                amount = transfer.amount
            
            print(f"  Token: {token_addr}")
            print(f"  From: {from_addr}")
            print(f"  To: {to_addr}")
            print(f"  Amount: {amount}")
            
            # Check if it's from the pool
            if from_addr.lower() == pool_address.lower():
                print("  ⚠️ TRANSFER FROM POOL - Potential liquidity removal")
        
        # Analyze Uniswap events
        print("\n\nUNISWAP EVENTS:")
        print("-" * 40)
        
        # Check for swap events
        if hasattr(tx, 'uniswap_v2_swaps'):
            print(f"Uniswap V2 Swaps: {len(tx.uniswap_v2_swaps)}")
            
        if hasattr(tx, 'uniswap_v3_swaps'):
            print(f"Uniswap V3 Swaps: {len(tx.uniswap_v3_swaps)}")
        
        # Check for burn events (liquidity removal)
        if hasattr(tx, 'burns'):
            print(f"Burn Events: {len(tx.burns)}")
            for burn in tx.burns:
                print(f"  ⚠️ BURN EVENT DETECTED - Liquidity removal confirmed")
                
        # Analyze internal transactions
        print("\n\nINTERNAL TRANSACTIONS:")
        print("-" * 40)
        eth_transfers = []
        
        for itx in tx.internal_transactions:
            # Handle as dict or object
            if isinstance(itx, dict):
                value = float(itx.get('value', 0))
                from_addr = itx.get('from_address', 'Unknown')
                to_addr = itx.get('to_address', 'Unknown')
                depth = itx.get('depth', 0)
            else:
                value = float(itx.value if hasattr(itx, 'value') else 0)
                from_addr = itx.from_address
                to_addr = itx.to_address
                depth = itx.depth if hasattr(itx, 'depth') else 0
                
            if value > 0:
                eth_transfers.append({
                    'from': from_addr,
                    'to': to_addr,
                    'value': value,
                    'depth': depth
                })
                
        print(f"Total internal transactions: {len(tx.internal_transactions)}")
        print(f"ETH transfers (value > 0): {len(eth_transfers)}")
        
        for i, itx in enumerate(eth_transfers[:5]):  # Show first 5
            print(f"\nETH Transfer {i+1}:")
            print(f"  From: {itx['from']}")
            print(f"  To: {itx['to']}")
            print(f"  Value: {itx['value'] / 10**18:.6f} ETH")
            print(f"  Depth: {itx['depth']}")
            
            # Check if it's from the pool
            if itx['from'].lower() == pool_address.lower():
                print(f"  ⚠️ ETH REMOVED FROM POOL: {itx['value'] / 10**18:.6f} ETH")
        
        # Look for specific function calls
        print("\n\nTRANSACTION TYPE ANALYSIS:")
        print("-" * 40)
        print(f"Transaction Type: {tx.tx_type}")
        if hasattr(tx, 'actions'):
            print(f"Actions: {tx.actions}")
        
        # Check input data
        input_data = tx.input if isinstance(tx.input, str) else "0x" + tx.input.hex() if hasattr(tx.input, 'hex') else str(tx.input)
        print(f"\nInput Data (first 10 bytes): {input_data[:10] if len(input_data) > 10 else input_data}")
        
        # Common liquidity removal function signatures
        liquidity_signatures = {
            "0xe8e33700": "addLiquidity",
            "0xf305d719": "addLiquidityETH",
            "0x02751cec": "removeLiquidityETH",
            "0xbaa2abde": "removeLiquidityETHWithPermit",
            "0x2195995c": "removeLiquidityWithPermit",
            "0xaf2979eb": "removeLiquidityETHSupportingFeeOnTransferTokens",
            "0x5b0d5984": "removeLiquidityETHWithPermitSupportingFeeOnTransferTokens",
        }
        
        for sig, name in liquidity_signatures.items():
            if input_data.startswith(sig):
                print(f"  ⚠️ LIQUIDITY FUNCTION DETECTED: {name} ({sig})")
                break
        
        # Summary
        print("\n\n" + "=" * 60)
        print("SUMMARY:")
        
        # Calculate total ETH removed
        total_eth_removed = sum(
            etx['value'] / 10**18 for etx in eth_transfers 
            if etx['from'].lower() == pool_address.lower()
        )
        
        # Check for liquidity removal indicators
        removal_detected = False
        
        if total_eth_removed > 0:
            print(f"⚠️ LIQUIDITY REMOVAL DETECTED!")
            print(f"   Total ETH removed from pool: {total_eth_removed:.6f} ETH")
            removal_detected = True
            
        # Check ERC20 transfers from pool
        pool_transfers = []
        for transfer in tx.erc20_transfers:
            if isinstance(transfer, dict):
                from_addr = transfer.get('from_address', '')
            else:
                from_addr = transfer.from_address
                
            if from_addr.lower() == pool_address.lower():
                pool_transfers.append(transfer)
                
        if pool_transfers:
            print(f"⚠️ TOKEN TRANSFERS FROM POOL DETECTED!")
            print(f"   Found {len(pool_transfers)} token transfers from pool")
            removal_detected = True
            
        # Check for burn events
        if hasattr(tx, 'burns') and len(tx.burns) > 0:
            print(f"⚠️ BURN EVENTS DETECTED!")
            print(f"   Found {len(tx.burns)} burn events (LP token burns)")
            removal_detected = True
            
        if not removal_detected:
            print("✅ No direct liquidity removal indicators found")
            print("   This may be a regular swap or other pool interaction")
            
    except Exception as e:
        print(f"Error analyzing transaction: {e}")
        import traceback
        traceback.print_exc()


def analyze_multiple_removals():
    """Analyze multiple potential liquidity removal transactions"""
    
    print("\n\n" + "=" * 60)
    print("ANALYZING MULTIPLE LIQUIDITY REMOVALS")
    print("=" * 60)
    
    # List of suspicious transactions to check
    suspicious_txs = [
        ("0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966", "Potential rug pull"),
        # Add more transactions here
    ]
    
    py_reth = pyreth.PyReth()
    processor = py_reth.tx_processor()
    
    for tx_hash, description in suspicious_txs:
        print(f"\nChecking: {description}")
        print(f"TX: {tx_hash}")
        
        try:
            tx = processor.process_transaction_from_hash_with_simulation(tx_hash)
            
            # Quick checks for liquidity removal
            has_burns = hasattr(tx, 'burns') and len(tx.burns) > 0
            has_remove_function = False
            
            input_data = tx.input if isinstance(tx.input, str) else "0x" + tx.input.hex() if hasattr(tx.input, 'hex') else str(tx.input)
            remove_sigs = ["0x02751cec", "0xbaa2abde", "0x2195995c", "0xaf2979eb", "0x5b0d5984"]
            
            for sig in remove_sigs:
                if input_data.startswith(sig):
                    has_remove_function = True
                    break
                    
            if has_burns or has_remove_function:
                print("  ⚠️ LIQUIDITY REMOVAL LIKELY")
            else:
                print("  ✅ No obvious removal indicators")
                
        except Exception as e:
            print(f"  ❌ Error: {e}")


if __name__ == "__main__":
    analyze_liquidity_removal()
    # Optionally analyze multiple transactions
    # analyze_multiple_removals()
