#!/usr/bin/env python3
"""
Basic PyReth Price Client Usage - Demonstrates eth_prices integration

This example shows how to use PyReth's integrated price client to get
ETH prices from multiple DEX protocols using a single shared database connection.

Features demonstrated:
- Singleton database pattern (no multiple DB connections)
- Working protocols only (Uniswap V2/V3, SushiSwap)
- Real price data from Reth local database
- Sub-millisecond latency (~0.24-0.35ms per query)

Algorithm:
1. Create PyReth singleton instance (opens Reth database once)
2. Get price_client from singleton (shares database connection)
3. Initialize working protocols (Uniswap V2/V3, SushiSwap)
4. Fetch ETH/USD prices from each protocol
5. Display results with price consistency check
"""

import pyreth
import sys
import time

def main():
    print("🔗 PyReth Price Client Demo - ETH Price Fetching")
    print("=" * 60)
    
    try:
        # Create main PyReth instance (singleton pattern - opens DB once)
        print("📦 Initializing PyReth singleton...")
        reth = pyreth.PyReth()
        print(f"✅ Connected: {reth.connection_info()}")
        
        # Get price client that shares the database connection
        print("\n💰 Creating price client (shared DB connection)...")
        price_client = reth.price_client()
        
        # Initialize working protocols only (these return real prices)
        print("🔧 Initializing working DEX protocols...")
        working_protocols = []
        
        try:
            price_client.with_uniswap_v2()
            working_protocols.append("Uniswap V2")
            print("  ✅ Uniswap V2 initialized")
        except Exception as e:
            print(f"  ❌ Uniswap V2 failed: {e}")
        
        try:
            price_client.with_uniswap_v3()
            working_protocols.append("Uniswap V3")
            print("  ✅ Uniswap V3 initialized")
        except Exception as e:
            print(f"  ❌ Uniswap V3 failed: {e}")
            
        try:
            price_client.with_sushiswap()
            working_protocols.append("SushiSwap")
            print("  ✅ SushiSwap initialized")
        except Exception as e:
            print(f"  ❌ SushiSwap failed: {e}")
        
        print(f"\n📊 Successfully initialized {len(working_protocols)} protocols")
        print(f"Working protocols: {', '.join(working_protocols)}")
        
        # Fetch prices from working protocols
        print("\n💵 Fetching ETH/USD prices...")
        print("-" * 40)
        
        prices = {}
        total_time = 0
        
        # Uniswap V2
        if "Uniswap V2" in working_protocols:
            start_time = time.time()
            try:
                price_data = price_client.get_uniswap_v2_price("ETH/USD")
                elapsed = (time.time() - start_time) * 1000  # ms
                total_time += elapsed
                
                prices["Uniswap V2"] = price_data.price
                print(f"✅ Uniswap V2    $ {price_data.price:,.2f} ({elapsed:.2f}ms)")
            except Exception as e:
                print(f"❌ Uniswap V2    Error: {e}")
        
        # Uniswap V3 (use specific fee tier - 0.05% is most liquid)
        if "Uniswap V3" in working_protocols:
            start_time = time.time()
            try:
                price_data = price_client.get_uniswap_v3_price("ETH/USD_V3_500")  # 0.05% fee tier
                elapsed = (time.time() - start_time) * 1000  # ms
                total_time += elapsed
                
                prices["Uniswap V3"] = price_data.price
                print(f"✅ Uniswap V3    $ {price_data.price:,.2f} ({elapsed:.2f}ms)")
            except Exception as e:
                print(f"❌ Uniswap V3    Error: {e}")
        
        # SushiSwap
        if "SushiSwap" in working_protocols:
            start_time = time.time()
            try:
                price_data = price_client.get_sushiswap_price("ETH/USD")
                elapsed = (time.time() - start_time) * 1000  # ms
                total_time += elapsed
                
                prices["SushiSwap"] = price_data.price
                print(f"✅ SushiSwap     $ {price_data.price:,.2f} ({elapsed:.2f}ms)")
            except Exception as e:
                print(f"❌ SushiSwap     Error: {e}")
        
        # Price consistency analysis
        if len(prices) > 1:
            print("\n📈 Price Consistency Analysis")
            print("-" * 40)
            
            price_values = list(prices.values())
            min_price = min(price_values)
            max_price = max(price_values)
            spread = max_price - min_price
            spread_pct = (spread / min_price) * 100
            
            print(f"Min Price: $ {min_price:,.2f}")
            print(f"Max Price: $ {max_price:,.2f}")
            print(f"Spread:    $ {spread:.2f} ({spread_pct:.3f}%)")
            
            if spread_pct < 0.1:
                print("✅ Excellent price consistency across protocols!")
            elif spread_pct < 0.5:
                print("🟡 Good price consistency")
            else:
                print("🔴 Large price spread - potential arbitrage opportunity")
        
        # Performance summary
        print(f"\n⚡ Performance Summary")
        print("-" * 40)
        print(f"Total queries: {len(prices)}")
        print(f"Total time: {total_time:.2f}ms")
        print(f"Average latency: {total_time / len(prices):.2f}ms per query")
        print(f"Throughput: {1000 / (total_time / len(prices)):.0f} queries/second")
        
        # Protocol status
        print(f"\n🔍 Protocol Status")
        print("-" * 40)
        print(f"✅ Working: {len(working_protocols)}/6 protocols")
        print(f"❌ Disabled: Chainlink, Curve, Balancer (research needed)")
        
        # Client summary
        print(f"\n📋 Client Summary")
        print("-" * 40)
        summary = price_client.get_summary()
        for key, value in summary.items():
            print(f"{key}: {value}")
        
        print(f"\n✨ Demo completed successfully!")
        
    except Exception as e:
        print(f"❌ Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()