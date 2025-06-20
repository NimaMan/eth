#!/usr/bin/env python3
"""
Check Trading Enabled Status

Takes a contract address, builds a token, and prints trading enabled block and tx hash for each pool.
Not a test - just a verification script for actual token trading status.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

import argparse
from web3 import Web3
from eth_token.erc20_token.data.live_token_data_v1 import LiveTokenData
from eth_block_processor.utils.logger import get_logger


def check_trading_enabled(contract_address: str):
    """
    Build a token and check trading enabled status for each pool.
    
    Args:
        contract_address: The token contract address to check
    """
    print(f"=" * 80)
    print(f"TRADING ENABLED STATUS CHECK")
    print(f"=" * 80)
    print(f"Token Address: {contract_address}")
    print(f"=" * 80)
    
    try:
        # Build the token
        print(f"🏗️  Building token from contract address...")
        token = LiveTokenData(contract_address=contract_address)
        
        # Check if token has pools
        if not token.pool_manager.has_pools():
            print(f"❌ No pools found for token {contract_address}")
            return
            
        print(f"✅ Token built successfully")
        print(f"   - Total pools: {len(token.pool_manager.get_all_pools())}")
        print(f"   - V2 pools: {len(token.pool_manager.get_pools_by_protocol('V2'))}")
        print(f"   - V3 pools: {len(token.pool_manager.get_pools_by_protocol('V3'))}")
        print(f"   - V4 pools: {token.pool_manager.has_v4_pools()}")
        
        # Check pool manager level trading enabled
        print(f"\n🎯 POOL MANAGER TRADING ENABLED:")
        pm_trading_info = token.pool_manager.get_trading_enabled_info()
        print(f"   Enabled: {pm_trading_info['enabled']}")
        if pm_trading_info['enabled']:
            print(f"   Block: {pm_trading_info['block']}")
            print(f"   Tx Hash: {pm_trading_info['txn']}")
        else:
            print(f"   Block: N/A")
            print(f"   Tx Hash: N/A")
        
        # Check individual pool trading enabled
        print(f"\n📊 INDIVIDUAL POOL TRADING ENABLED:")
        all_pools = token.pool_manager.get_all_pools()
        
        if not all_pools:
            print(f"   No pools to check")
            return
            
        for i, pool in enumerate(all_pools, 1):
            pool_id = getattr(pool, 'pool_id', pool.pool_address)
            protocol = pool.get_protocol()
            
            # Get denom name for better display
            denom_name = "Unknown"
            if hasattr(pool, 'denom_address'):
                denom_map = {
                    '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2': 'WETH',
                    '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48': 'USDC',
                    '0xdac17f958d2ee523a2206206994597c13d831ec7': 'USDT',
                    '0x6b175474e89094c44da98b954eedeac495271d0f': 'DAI'
                }
                denom_name = denom_map.get(pool.denom_address.lower(), 'Unknown')
            
            print(f"\n   Pool {i}: {protocol} Pool")
            print(f"   Address: {pool_id}")
            print(f"   Paired with: {denom_name}")
            
            # Get trading enabled info
            trading_info = pool.get_trading_enabled_info()
            print(f"   Trading Enabled: {trading_info['enabled']}")
            
            if trading_info['enabled']:
                print(f"   Trading Enabled Block: {trading_info['block']}")
                print(f"   Trading Enabled Tx Hash: {trading_info['txn']}")
            else:
                print(f"   Trading Enabled Block: N/A")
                print(f"   Trading Enabled Tx Hash: N/A")
            
            # Additional pool stats
            print(f"   Token Reserve: {pool.get_token_reserve():.6f}")
            print(f"   Denom Reserve: {pool.get_denom_reserve():.6f}")
            print(f"   Current Price: {pool.get_price():.8f}")
            print(f"   Total Swaps: {pool.state.total_swaps}")
            print(f"   Healthy: {pool.is_healthy()}")
        
        print(f"\n" + "=" * 80)
        print(f"SUMMARY")
        print(f"=" * 80)
        
        enabled_pools = [p for p in all_pools if p.is_trading_enabled()]
        print(f"Total pools: {len(all_pools)}")
        print(f"Trading enabled pools: {len(enabled_pools)}")
        print(f"Trading disabled pools: {len(all_pools) - len(enabled_pools)}")
        
        if enabled_pools:
            earliest_block = min(p.trading_enabled_block for p in enabled_pools if p.trading_enabled_block > 0)
            print(f"Earliest trading enabled block: {earliest_block}")
        
    except Exception as e:
        print(f"❌ ERROR: {e}")
        import traceback
        traceback.print_exc()


def main():
    parser = argparse.ArgumentParser(description='Check trading enabled status for a token')
    parser.add_argument('contract_address', help='Token contract address')
    
    args = parser.parse_args()
    
    # Validate address format
    if not args.contract_address.startswith('0x') or len(args.contract_address) != 42:
        print(f"❌ Invalid contract address format: {args.contract_address}")
        print(f"Expected format: 0x followed by 40 hex characters")
        sys.exit(1)
    
    check_trading_enabled(args.contract_address)


if __name__ == "__main__":
    main()