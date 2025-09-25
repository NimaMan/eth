#!/usr/bin/env python3
"""
Structured Price Sources Example - Demonstrates detailed price source tracking

This example shows the new structured price information that provides complete
transparency about where each price comes from, including:
- Exact protocol (UniswapV2, UniswapV3, SushiSwap, etc.)
- Specific stablecoin used (USDC, USDT, DAI)
- Pool addresses for complete transparency
- Pool-specific data (reserves, liquidity, fee tiers)

Algorithm:
1. Initialize PyReth price client with multiple protocols
2. Fetch prices using new structured API
3. Display detailed source information for each price
4. Show comprehensive ETH price dictionary across all sources
5. Demonstrate stablecoin/protocol transparency
"""

import pyreth
import json

def main():
    print("🔍 Structured Price Sources - Complete Transparency")
    print("=" * 60)
    
    try:
        # Create PyReth instance
        reth = pyreth.PyReth()
        price_client = reth.price_client()
        
        # Initialize multiple protocols
        print("🔧 Initializing protocols...")
        price_client.with_uniswap_v2()
        price_client.with_uniswap_v3()
        price_client.with_sushiswap()
        
        print("✅ Initialized: Uniswap V2, V3, SushiSwap")
        print()
        
        # Test new structured price methods
        print("💰 ETH Prices with Complete Source Information")
        print("-" * 50)
        
        # Get USDC price from Uniswap V2
        try:
            usdc_v2_price = price_client.get_eth_price("UniswapV2", "USDC")
            print(f"📊 Uniswap V2 ETH/USDC: ${usdc_v2_price.price:,.2f}")
            
            # Access structured source info
            source_info = usdc_v2_price.source_info
            print(f"   Protocol: {source_info['protocol']}")
            print(f"   Stablecoin: {source_info['stablecoin']}")
            print(f"   Pool Address: {source_info['pool_address']}")
            print(f"   Token0: {source_info['token0']} | Token1: {source_info['token1']}")
            print(f"   Reserve0: {source_info['reserve0']:,.2f} | Reserve1: {source_info['reserve1']:,.6f}")
            print()
            
        except Exception as e:
            print(f"❌ Uniswap V2 error: {e}")
        
        # Get USDC price from Uniswap V3 (0.05% fee)
        try:
            usdc_v3_500_price = price_client.get_eth_price("UniswapV3_500", "USDC")
            print(f"📊 Uniswap V3 (0.05%) ETH/USDC: ${usdc_v3_500_price.price:,.2f}")
            
            source_info = usdc_v3_500_price.source_info
            print(f"   Protocol: {source_info['protocol']}")
            print(f"   Stablecoin: {source_info['stablecoin']}")
            print(f"   Pool Address: {source_info['pool_address']}")
            print(f"   Fee Tier: {source_info['fee_tier']} bps (0.05%)")
            print(f"   Current Tick: {source_info['tick']}")
            print(f"   Liquidity: {source_info['liquidity']}")
            print()
            
        except Exception as e:
            print(f"❌ Uniswap V3 (0.05%) error: {e}")
        
        # Get USDC price from Uniswap V3 (0.3% fee)
        try:
            usdc_v3_3000_price = price_client.get_eth_price("UniswapV3_3000", "USDC")
            print(f"📊 Uniswap V3 (0.3%) ETH/USDC: ${usdc_v3_3000_price.price:,.2f}")
            
            source_info = usdc_v3_3000_price.source_info
            print(f"   Protocol: {source_info['protocol']}")
            print(f"   Fee Tier: {source_info['fee_tier']} bps (0.3%)")
            print(f"   Pool Address: {source_info['pool_address']}")
            print()
            
        except Exception as e:
            print(f"❌ Uniswap V3 (0.3%) error: {e}")
        
        # Show available sources
        print("🗂️  Available Protocol-Stablecoin Combinations")
        print("-" * 50)
        available_sources = price_client.get_available_sources()
        
        for stablecoin, protocols in available_sources.items():
            print(f"{stablecoin}: {', '.join(protocols)}")
        print()
        
        # Get comprehensive ETH prices
        print("🌍 Comprehensive ETH Price Dictionary")
        print("-" * 50)
        all_eth_prices = price_client.get_all_eth_prices()
        
        eth_prices = all_eth_prices["ETH"]
        for stablecoin, protocols in eth_prices.items():
            print(f"\n💵 {stablecoin} Prices:")
            
            for protocol, price_data in protocols.items():
                source_info = price_data.source_info
                pool_address = source_info['pool_address'][:10] + "..." if len(source_info['pool_address']) > 10 else source_info['pool_address']
                
                fee_info = ""
                if 'fee_tier' in source_info and source_info['fee_tier'] is not None:
                    fee_info = f" ({source_info['fee_tier']/100:.2f}%)"
                
                print(f"   {protocol}{fee_info}: ${price_data.price:,.2f} | Pool: {pool_address}")
        
        # Price consistency analysis
        print("\n📈 Price Consistency Analysis")
        print("-" * 50)
        
        all_prices = []
        for stablecoin, protocols in eth_prices.items():
            for protocol, price_data in protocols.items():
                all_prices.append({
                    'price': price_data.price,
                    'stablecoin': stablecoin,
                    'protocol': protocol,
                    'pool': price_data.source_info['pool_address']
                })
        
        if len(all_prices) > 1:
            prices_only = [p['price'] for p in all_prices]
            min_price = min(prices_only)
            max_price = max(prices_only)
            spread = max_price - min_price
            spread_pct = (spread / min_price) * 100
            
            print(f"Min Price: ${min_price:,.2f}")
            print(f"Max Price: ${max_price:,.2f}")
            print(f"Spread: ${spread:.2f} ({spread_pct:.3f}%)")
            
            # Find min/max sources
            min_source = next(p for p in all_prices if p['price'] == min_price)
            max_source = next(p for p in all_prices if p['price'] == max_price)
            
            print(f"Lowest:  {min_source['protocol']} {min_source['stablecoin']}")
            print(f"Highest: {max_source['protocol']} {max_source['stablecoin']}")
            
            if spread_pct < 0.1:
                print("✅ Excellent price consistency!")
            elif spread_pct < 0.5:
                print("🟡 Good price consistency")
            else:
                print("🔴 Large spread - potential arbitrage opportunity")
        
        print("\n✨ Structured price source tracking completed!")
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main()