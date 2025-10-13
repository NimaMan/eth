#!/usr/bin/env python3
"""
Batch Token Analysis Example

Demonstrates how to analyze multiple tokens in batch using the pool_buy_sell_simulator.
Generates a CSV report with trading viability and tax information for each token.
"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

import pyreth
import csv
import time
from datetime import datetime
from typing import List, Dict, Any


# Common tokens to analyze
TOKENS_TO_ANALYZE = [
    # Format: (symbol, token_address, pool_address, pool_type, decimals)
    
    # Stablecoins
    ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 
     "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc", "V2", 6),
    
    ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7",
     "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", "V2", 6),
    
    ("DAI", "0x6B175474E89094C44Da98b954EedeAC495271d0F",
     "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", "V2", 18),
    
    # DeFi Tokens
    ("UNI", "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984",
     "0xd3d2E2692501A5c9Ca623199D38826e513033a17", "V2", 18),
    
    ("AAVE", "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9",
     "0xDFC14d2Af169B0D36C4EFF567Ada9b2E0CAE044f", "V2", 18),
    
    ("LINK", "0x514910771AF9Ca656af840dff83E8264EcF986CA",
     "0xa2107FA5B38d9bbd2C461D6EDf11B11A50F6b974", "V2", 18),
    
    # Wrapped Assets
    ("WBTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
     "0xBb2b8038a1640196FbE3e38816F3e67Cba72D940", "V2", 8),
    
    # Meme Tokens (may have taxes)
    ("SHIB", "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
     "0x811beEd0119b4AfCE20D2583EB608C6F7AF1954f", "V2", 18),
    
    ("PEPE", "0x6982508145454Ce325dDbE47a25d4ec3d2311933",
     "0xA43fe16908251ee70EF74718545e4FE6C5cCEc9f", "V2", 18),
]


def analyze_token(simulator, symbol: str, token: str, pool: str, 
                 pool_type: str, decimals: int) -> Dict[str, Any]:
    """Analyze a single token and return results"""
    start_time = time.time()
    
    try:
        # Create config with token decimals
        config = pyreth.PoolBuySellParameters()
        config.test_amount_eth = 0.01  # Small amount for testing
        config.token_decimals = decimals
        
        # Check based on pool type
        if pool_type == "V2":
            result = simulator.check_uniswap_v2_pool(token, pool, config)
        else:
            # For V3, would need fee tier - defaulting to V2 for this example
            result = simulator.check_uniswap_v2_pool(token, pool, config)
        
        analysis_time = time.time() - start_time
        
        return {
            "symbol": symbol,
            "token_address": token,
            "pool_address": pool,
            "pool_type": pool_type,
            "decimals": decimals,
            "can_buy": result.can_buy,
            "can_approve": result.can_approve,
            "can_sell": result.can_sell,
            "is_tradeable": result.can_buy and result.can_approve and result.can_sell,
            "buy_tax": round(result.buy_tax_percentage, 2),
            "sell_tax": round(result.sell_tax_percentage, 2),
            "total_tax": round(result.buy_tax_percentage + result.sell_tax_percentage, 2),
            "block_number": result.block_number,
            "error": result.error_message or "",
            "analysis_time": round(analysis_time, 2)
        }
        
    except Exception as e:
        analysis_time = time.time() - start_time
        return {
            "symbol": symbol,
            "token_address": token,
            "pool_address": pool,
            "pool_type": pool_type,
            "decimals": decimals,
            "can_buy": False,
            "can_approve": False,
            "can_sell": False,
            "is_tradeable": False,
            "buy_tax": -1,
            "sell_tax": -1,
            "total_tax": -1,
            "block_number": 0,
            "error": str(e),
            "analysis_time": round(analysis_time, 2)
        }


def save_results_to_csv(results: List[Dict], filename: str):
    """Save analysis results to CSV file"""
    if not results:
        return
    
    # Define field order
    fieldnames = [
        "symbol", "is_tradeable", "buy_tax", "sell_tax", "total_tax",
        "can_buy", "can_approve", "can_sell", 
        "token_address", "pool_address", "pool_type", "decimals",
        "block_number", "analysis_time", "error"
    ]
    
    with open(filename, 'w', newline='') as csvfile:
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(results)


def main():
    print("=" * 80)
    print("Batch Token Analysis - Testing Multiple Tokens")
    print("=" * 80)
    print()
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    output_file = f"token_analysis_{timestamp}.csv"
    
    try:
        # Initialize
        reth = pyreth.PyReth()
        simulator = reth.pool_buy_sell_simulator()
        print("✅ Pool Buy Sell Simulator initialized")
        print()
        
        print(f"Analyzing {len(TOKENS_TO_ANALYZE)} tokens...")
        print("This may take a few minutes...\n")
        
        results = []
        tradeable_count = 0
        taxed_count = 0
        
        # Analyze each token
        for i, (symbol, token, pool, pool_type, decimals) in enumerate(TOKENS_TO_ANALYZE, 1):
            print(f"[{i}/{len(TOKENS_TO_ANALYZE)}] Analyzing {symbol}...", end=" ")
            
            result = analyze_token(simulator, symbol, token, pool, pool_type, decimals)
            results.append(result)
            
            if result["is_tradeable"]:
                tradeable_count += 1
                if result["total_tax"] > 0:
                    taxed_count += 1
                status = f"✅ Tradeable (Tax: {result['total_tax']:.1f}%)"
            else:
                status = "❌ Not tradeable"
            
            print(f"{status} [{result['analysis_time']}s]")
        
        # Save to CSV
        save_results_to_csv(results, output_file)
        print(f"\n📊 Results saved to: {output_file}")
        
        # Print summary
        print("\n" + "=" * 80)
        print("Analysis Summary")
        print("=" * 80)
        print(f"Total tokens analyzed: {len(results)}")
        print(f"Tradeable tokens: {tradeable_count} ({tradeable_count/len(results)*100:.1f}%)")
        print(f"Tokens with taxes: {taxed_count}")
        print()
        
        # Show top results
        tradeable_results = [r for r in results if r["is_tradeable"]]
        if tradeable_results:
            # Sort by total tax (lowest first)
            tradeable_results.sort(key=lambda x: x["total_tax"])
            
            print("Best tokens (lowest taxes):")
            for r in tradeable_results[:5]:
                print(f"  • {r['symbol']:6} - Total Tax: {r['total_tax']:5.2f}% "
                      f"(Buy: {r['buy_tax']:.2f}%, Sell: {r['sell_tax']:.2f}%)")
                      
            if taxed_count > 0:
                print("\nTokens with highest taxes:")
                high_tax = sorted(tradeable_results, key=lambda x: x["total_tax"], reverse=True)
                for r in high_tax[:3]:
                    if r["total_tax"] > 0:
                        print(f"  • {r['symbol']:6} - Total Tax: {r['total_tax']:5.2f}% "
                              f"(Buy: {r['buy_tax']:.2f}%, Sell: {r['sell_tax']:.2f}%)")
        
        # Show failed tokens
        failed = [r for r in results if not r["is_tradeable"]]
        if failed:
            print(f"\nFailed tokens ({len(failed)}):")
            for r in failed[:5]:
                error_msg = r['error'][:50] + "..." if len(r['error']) > 50 else r['error']
                print(f"  • {r['symbol']:6} - {error_msg}")
                
        total_time = sum(r["analysis_time"] for r in results)
        print(f"\nTotal analysis time: {total_time:.1f} seconds")
        print(f"Average time per token: {total_time/len(results):.2f} seconds")
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    main()