#!/usr/bin/env python3
"""
Uniswap V3 Pools Example

Demonstrates how to use the pool_buy_sell_simulator with Uniswap V3 pools.
V3 pools have different fee tiers (0.05%, 0.3%, 1%) which affect liquidity
and trading costs.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

import pyreth

WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
WETH_DECIMALS = 18
USDC_DECIMALS = 6
TEST_AMOUNT_ETH = 1.0


def test_v3_pool(simulator, token: str, pool: str, fee_tier: int, tier_name: str):
    """Test a V3 pool with specific fee tier"""
    print(f"\n Testing {tier_name} Pool")
    print("-" * 40)
    print(f"Pool Address: {pool}")
    print(f"Fee Tier: {fee_tier} ({fee_tier/10000:.2f}%)")
    
    try:
        config = pyreth.PoolBuySellParameters.with_denom_amount(
            TEST_AMOUNT_ETH, USDC_DECIMALS, WETH_DECIMALS
        )
        config.denom_address = WETH_ADDRESS
        # Check the V3 pool
        result = simulator.check_uniswap_v3_pool(
            token_address=token,
            pool_address=pool,
            fee_tier=fee_tier,
            config=config,
        )

        print(f"\nResults:")
        print(f"  Can Buy:     {'✅' if result.can_buy else '❌'}")
        print(f"  Can Approve: {'✅' if result.can_approve else '❌'}")
        print(f"  Can Sell:    {'✅' if result.can_sell else '❌'}")
        print(f"  Tokens Out:  {result.tokens_received_raw}")
        print(f"  Denom Spent: {result.denom_spent_raw}")
        print(f"  Denom Recv : {result.denom_received_raw}")
        
        if result.can_buy and result.can_sell:
            print(f"\nCost Analysis:")
            print(f"  Buy Tax:     {result.buy_tax_percentage:.3f}%")
            print(f"  Sell Tax:    {result.sell_tax_percentage:.3f}%")
            total_cost = result.buy_tax_percentage + result.sell_tax_percentage + (fee_tier/10000 * 2)
            print(f"  Total Cost:  ~{total_cost:.3f}% (including pool fees)")
            
        if result.error_message:
            print(f"\n⚠️  Error: {result.error_message}")
            
        return result
        
    except Exception as e:
        print(f"❌ Error: {e}")
        return None


def main():
    print("=" * 70)
    print("Uniswap V3 Pool Analysis - Fee Tier Comparison")
    print("=" * 70)
    print()
    print("Uniswap V3 introduces multiple fee tiers for the same token pair.")
    print("Lower fees typically have less liquidity but cheaper swaps.")
    print("Higher fees have more liquidity but cost more per trade.")
    print()
    
    # USDC token and its V3 pools with different fee tiers
    USDC_TOKEN = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    
    # V3 pools for USDC/WETH with different fee tiers
    v3_pools = [
        # (pool_address, fee_tier_bps, description)
        ("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", 500, "0.05% Ultra Low Fee"),
        ("0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8", 3000, "0.3% Standard Fee"),
        ("0x7BeA39867e4169DBe237d55C8242a8f2fcDcc387", 10000, "1% High Fee"),
    ]
    
    try:
        # Initialize
        reth = pyreth.PyReth()
        simulator = reth.pool_buy_sell_simulator()
        print("✅ Pool Buy Sell Simulator initialized")
        
        print(f"\nAnalyzing USDC on different V3 fee tiers")
        print(f"Token: {USDC_TOKEN}")
        print("=" * 70)
        
        results = []
        for pool_addr, fee_tier, description in v3_pools:
            result = test_v3_pool(
                simulator,
                USDC_TOKEN,
                pool_addr,
                fee_tier,
                description
            )
            if result:
                results.append({
                    "description": description,
                    "fee_tier": fee_tier,
                    "tradeable": result.can_buy and result.can_approve and result.can_sell,
                    "buy_tax": result.buy_tax_percentage,
                    "sell_tax": result.sell_tax_percentage,
                    "pool": pool_addr
                })
        
        # Summary comparison
        print("\n" + "=" * 70)
        print("V3 Fee Tier Comparison Summary")
        print("=" * 70)
        
        tradeable_pools = [r for r in results if r["tradeable"]]
        
        if tradeable_pools:
            print(f"\n✅ USDC is tradeable on {len(tradeable_pools)} V3 pool(s)")
            
            # Compare costs
            print("\nCost Comparison (taxes + fees):")
            for pool in sorted(tradeable_pools, key=lambda x: x["fee_tier"]):
                pool_fee_pct = pool["fee_tier"] / 10000
                total_cost = pool["buy_tax"] + pool["sell_tax"] + (pool_fee_pct * 2)
                print(f"  • {pool['description']:20} - Total: {total_cost:.3f}%")
                print(f"    (Tax: {pool['buy_tax'] + pool['sell_tax']:.3f}%, "
                      f"Pool Fees: {pool_fee_pct * 2:.3f}%)")
            
            # Recommendation
            best_pool = min(tradeable_pools, 
                          key=lambda x: x["buy_tax"] + x["sell_tax"] + (x["fee_tier"]/10000 * 2))
            print(f"\n🏆 Recommended Pool: {best_pool['description']}")
            print(f"   Lowest total cost for small trades")
            
        else:
            print("❌ USDC not tradeable on tested V3 pools")
            
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()
