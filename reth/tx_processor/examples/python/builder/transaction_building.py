#!/usr/bin/env python3
"""
Example usage of the TxBuilder functionality within ethtx

This demonstrates how to use the user-friendly transaction builder
from Python without needing to understand ABI encoding or contract addresses.

This example was migrated from tx_builder/examples/python_usage_example.py
"""

import ethtx

def main():
    """
    Example showing how TxBuilder is used within ethtx
    """
    
    print("=== ethtx.TxBuilder Usage Example ===\n")
    
    # Create a transaction builder for mainnet
    builder = ethtx.TxBuilder.mainnet()
    
    # Example addresses (using actual mainnet addresses)
    from_address = "0x742d35Cc6134C0532925a3b8C17ebb6F5E9DFcf4"
    to_address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
    
    print("1. ETH Transfer:")
    eth_tx = builder.eth_transfer(from_address, to_address, "1.5")
    print(f"   From: {eth_tx['from']}")
    print(f"   To: {eth_tx['to']}")
    print(f"   Value: {eth_tx['value']} wei (1.5 ETH)")
    print(f"   Gas: {eth_tx.get('gas', 'default')}")
    print()
    
    print("2. ERC20 Transfer (USDC):")
    usdc_tx = builder.erc20_transfer("USDC", from_address, to_address, "1000.0")
    print(f"   From: {usdc_tx['from']}")
    print(f"   To: {usdc_tx['to']} (USDC contract)")
    print(f"   Data: {usdc_tx['data'][:50]}... (encoded transfer call)")
    print(f"   Amount: 1000.0 USDC")
    print()
    
    print("3. ERC20 Approval (Unlimited USDC to Uniswap):")
    approve_tx = builder.erc20_approve("USDC", from_address, "uniswap_router", "unlimited")
    print(f"   Owner: {approve_tx['from']}")
    print(f"   To: {approve_tx['to']} (USDC contract)")
    print(f"   Data: {approve_tx['data'][:50]}... (encoded approve call)")
    print(f"   Amount: Unlimited")
    print()
    
    print("4. Token Information:")
    usdc_info = builder.get_token_info("USDC")
    print(f"   Symbol: {usdc_info['symbol']}")
    print(f"   Address: {usdc_info['address']}")
    print(f"   Decimals: {usdc_info['decimals']}")
    print()
    
    print("5. Available Tokens:")
    tokens = builder.list_tokens()
    print(f"   Available tokens: {', '.join(tokens[:10])}...")
    print(f"   Total tokens: {len(tokens)}")
    print()
    
    print("6. Gas Estimation:")
    gas_eth = builder.estimate_gas("eth_transfer")
    gas_erc20 = builder.estimate_gas("erc20_transfer")
    print(f"   ETH transfer: {gas_eth} gas")
    print(f"   ERC20 transfer: {gas_erc20} gas")
    print()
    
    print("✅ TxBuilder integration successful!")
    print("   This functionality is now part of the unified ethtx module")

if __name__ == "__main__":
    try:
        main()
    except ImportError:
        print("❌ ethtx module not built. Please run:")
        print("   cd /home/nima/code/crypto/rust/tx_processor")
        print("   maturin develop --release --features python")
    except Exception as e:
        print(f"❌ Error: {e}")