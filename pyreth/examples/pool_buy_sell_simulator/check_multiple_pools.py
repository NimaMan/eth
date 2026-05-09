#!/usr/bin/env python3
"""
Check Multiple Pools Example

Demonstrates how to compare trading the same token across different pools
and DEXes. This example tests UNI token on both Uniswap V2 and V3 pools
with different fee tiers.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from pyreth import pool_buy_sell_simulator as pyreth_pool_buy_sell_simulator
from typing import List, Tuple


def check_pool(simulator, token: str, pool: str, pool_type: str, fee_tier: int = None) -> dict:
    """Check a single pool and return results"""
    try:
        if pool_type == "V2":
            result = simulator.check_uniswap_v2_pool(token, pool)
        elif pool_type == "V3" and fee_tier:
            result = simulator.check_uniswap_v3_pool(token, pool, fee_tier)
        else:
            return {"error": "Invalid pool type or missing fee tier"}
        
        return {
            "pool_type": pool_type,
            "fee_tier": fee_tier,
            "can_trade": result.can_buy and result.can_approve and result.can_sell,
            "buy_tax": result.buy_tax_percentage,
            "sell_tax": result.sell_tax_percentage,
            "error": result.error_message
        }
    except Exception as e:
        return {"error": str(e)}


def main():
    print("=" * 70)
    print("Multiple Pool Comparison - UNI Token")
    print("=" * 70)
    print()
    
    # UNI token address
    UNI_TOKEN = "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984"
    
    # Different pools to test
    pools_to_test = [
        # (pool_address, pool_type, fee_tier, description)
        ("0xd3d2E2692501A5c9Ca623199D38826e513033a17", "V2", None, "Uniswap V2 UNI/WETH"),
        ("0x1d42064Fc4Beb5F8aAF85F4617AE8b3b5B8Bd801", "V3", 3000, "Uniswap V3 0.3% fee"),
        ("0xe8c6b8ef04bb5710dd9007fc6f38d8a6d8e92a2f", "V3", 10000, "Uniswap V3 1% fee"),
    ]
    
    try:
        # Initialize
        simulator = pyreth_pool_buy_sell_simulator()
        print("✅ Pool Buy Sell Simulator initialized")
        print()
        
        print(f"Testing UNI token: {UNI_TOKEN}")
        print("Comparing across different pools and fee tiers...")
        print()
        
        # Test each pool
        results = []
        for pool_addr, pool_type, fee_tier, description in pools_to_test:
            print(f"Testing: {description}")
            print(f"  Pool: {pool_addr}")
            
            result = check_pool(simulator, UNI_TOKEN, pool_addr, pool_type, fee_tier)
            result["description"] = description
            result["pool_address"] = pool_addr
            results.append(result)
            
            if "error" in result and result["error"]:
                print(f"  ❌ Error: {result['error']}")
            else:
                status = "✅" if result["can_trade"] else "❌"
                print(f"  {status} Tradeable: {result['can_trade']}")
                if result["can_trade"]:
                    print(f"     Buy Tax: {result['buy_tax']:.2f}%")
                    print(f"     Sell Tax: {result['sell_tax']:.2f}%")
            print()
        
        # Summary comparison
        print("=" * 70)
        print("Summary Comparison")
        print("=" * 70)
        
        tradeable_pools = [r for r in results if r.get("can_trade", False)]
        
        if tradeable_pools:
            print(f"\n✅ Tradeable on {len(tradeable_pools)} pool(s):")
            
            # Find best pool (lowest total tax)
            best_pool = min(tradeable_pools, 
                          key=lambda x: x.get("buy_tax", 0) + x.get("sell_tax", 0))
            
            print(f"\n🏆 Best Pool: {best_pool['description']}")
            print(f"   Total Tax: {best_pool['buy_tax'] + best_pool['sell_tax']:.2f}%")
            print(f"   Pool Address: {best_pool['pool_address']}")
            
            print(f"\nAll Tradeable Pools:")
            for pool in tradeable_pools:
                total_tax = pool['buy_tax'] + pool['sell_tax']
                print(f"  • {pool['description']}: {total_tax:.2f}% total tax")
        else:
            print("❌ Token not tradeable on any tested pools")
            
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()