#!/usr/bin/env python3
"""
Stablecoin Market Analysis by Currency Unit using PyReth

Demonstrates how to analyze stablecoin markets grouped by currency unit (USD, EUR, JPY, etc.)
Each unit shows totals in its own denomination without cross-currency conversion.
"""

from pyreth import PyReth
from typing import Dict, List
import sys


def format_currency_amount(amount: float, unit_name: str) -> str:
    """Format amount based on currency unit"""
    formatters = {
        "US Dollar": lambda x: f"${x:,.2f}",
        "Euro": lambda x: f"€{x:,.2f}",
        "Japanese Yen": lambda x: f"¥{x:,.0f}",
        "British Pound": lambda x: f"£{x:,.2f}",
        "Singapore Dollar": lambda x: f"S${x:,.2f}",
        "Australian Dollar": lambda x: f"A${x:,.2f}",
        "Canadian Dollar": lambda x: f"C${x:,.2f}",
        "Chinese Yuan": lambda x: f"¥{x:,.2f}",
        "Indonesian Rupiah": lambda x: f"Rp {x:,.0f}",
    }
    
    formatter = formatters.get(unit_name, lambda x: f"{x:,.2f} {unit_name}")
    return formatter(amount)


def analyze_stablecoin_market_by_unit():
    """Analyze stablecoin market by currency unit"""
    
    print("💱 Stablecoin Market Analysis by Currency Unit (PyReth)")
    print("=" * 70)
    
    # Initialize PyReth with singleton pattern
    reth = PyReth()
    chain_query = reth.chain_query()
    
    # Get latest block
    latest_block = chain_query.get_latest_block()
    print(f"\n📊 Analyzing at block: {latest_block}")
    print("-" * 70)
    
    # Get entity statistics first
    entity_stats = chain_query.get_entity_stats()
    print(f"\n📈 Tracking {entity_stats['stablecoin_count']} stablecoins")
    
    # Analyze market by currency unit
    market_by_unit = chain_query.analyze_stablecoin_market_by_unit()
    
    print("\n🌍 MARKET BY CURRENCY UNIT")
    print("=" * 70)
    
    # Sort units by total supply
    units = market_by_unit['units']
    sorted_units = sorted(
        units.items(),
        key=lambda x: x[1]['total_supply_in_unit'],
        reverse=True
    )
    
    # Display each currency unit
    for unit_name, unit_data in sorted_units:
        # Skip units with no supply
        if unit_data['total_supply_in_unit'] == 0:
            continue
        
        print(f"\n💵 {unit_name} Market")
        print("-" * 50)
        
        # Format total based on currency
        total_formatted = format_currency_amount(
            unit_data['total_supply_in_unit'],
            unit_name
        )
        
        print(f"Total Supply: {total_formatted}")
        print(f"Active Tokens: {len(unit_data['tokens'])}")
        
        # Show top tokens in this unit
        if unit_data['tokens']:
            print("\nTop Stablecoins:")
            for i, token in enumerate(unit_data['tokens'][:5], 1):
                print(f"  {i}. {token['symbol']}")
                
                # Format amount based on currency
                amount_formatted = format_currency_amount(
                    token['total_supply'],
                    unit_name
                )
                
                print(f"     Supply: {amount_formatted}")
                print(f"     Market Share (within {unit_name}): {token['market_share_percent']:.2f}%")
        
        # Show concentration metrics for this unit
        if len(unit_data['tokens']) >= 3:
            top_3_share = sum(
                token['market_share_percent'] 
                for token in unit_data['tokens'][:3]
            )
            
            print("\nConcentration Metrics:")
            print(f"  Top 3 Control: {top_3_share:.2f}% of {unit_name} market")
            
            if top_3_share > 80:
                print("  ⚠️  Highly concentrated market")
            elif top_3_share > 60:
                print("  ⚡ Moderately concentrated market")
            else:
                print("  ✅ Competitive market")
    
    # Summary statistics
    print("\n📈 OVERALL SUMMARY")
    print("=" * 70)
    
    active_units = sum(1 for _, data in sorted_units if data['total_supply_in_unit'] > 0)
    total_tokens = sum(len(data['tokens']) for _, data in sorted_units)
    
    print(f"Active Currency Units: {active_units}")
    print(f"Total Active Stablecoins: {total_tokens}")
    
    # Show unit diversity
    print("\nUnit Diversity:")
    for unit_name, unit_data in sorted_units:
        if unit_data['total_supply_in_unit'] > 0:
            token_count = len(unit_data['tokens'])
            if token_count > 5:
                emoji = "🔥"
            elif token_count > 2:
                emoji = "📊"
            else:
                emoji = "🌱"
            
            print(f"  {emoji} {unit_name}: {token_count} token(s)")
    
    # Dominant unit analysis
    if sorted_units:
        dominant_unit, dominant_data = sorted_units[0]
        print(f"\n🏆 Dominant Currency Unit: {dominant_unit}")
        print(f"  Has {len(dominant_data['tokens'])} active stablecoins")
        
        if dominant_data['tokens']:
            top_token = dominant_data['tokens'][0]
            print(f"  Led by: {top_token['symbol']} ({top_token['market_share_percent']:.2f}% of {dominant_unit} market)")
    
    print("\n✅ Market analysis by currency unit complete!")
    
    print("\n💡 Key Insights:")
    print("- Each currency unit's market is analyzed independently")
    print("- Market shares are calculated WITHIN each unit (not across units)")
    print("- No cross-currency conversion is performed")
    print("- This provides accurate representation of each currency's ecosystem")
    
    # Also demonstrate overall market share analysis
    print("\n" + "=" * 70)
    print("📊 OVERALL MARKET SHARE ANALYSIS")
    print("=" * 70)
    
    market_share = chain_query.analyze_stablecoin_market_share()
    
    print(f"\nTotal Market Supply (USD equivalent): ${market_share['total_market_supply']:,.2f}")
    print(f"Top 3 Concentration: {market_share['top_3_concentration']:.2f}%")
    print(f"Top 5 Concentration: {market_share['top_5_concentration']:.2f}%")
    print(f"Herfindahl Index: {market_share['herfindahl_index']:.2f}")
    
    print("\nTop 5 Stablecoins Overall:")
    for i, coin in enumerate(market_share['stablecoins'][:5], 1):
        print(f"  {i}. {coin['symbol']} - {coin['unit']}")
        print(f"     Supply: {coin['total_supply']:,.2f}")
        print(f"     Market Share: {coin['market_share_percent']:.2f}%")


def test_entity_identification():
    """Test entity identification capabilities"""
    
    print("\n" + "=" * 70)
    print("🔍 ENTITY IDENTIFICATION TEST")
    print("=" * 70)
    
    reth = PyReth()
    chain_query = reth.chain_query()
    
    test_addresses = [
        ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC"),
        ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT"),
        ("0x28C6c06298d514Db089934071355E5743bf21d60", "Binance"),
        ("0x0171F896002665C3ea3Ed0c55f21026cA0A734A0", "BlackRock ETF"),
    ]
    
    for address, expected in test_addresses:
        entity_type = chain_query.identify_entity(address)
        
        emoji = {
            "stablecoin": "🪙",
            "cex": "🏦",
            "etf": "📈",
            "unknown": "❓"
        }.get(entity_type, "❓")
        
        print(f"\n{emoji} {address[:10]}...")
        print(f"  Expected: {expected}")
        print(f"  Type: {entity_type}")
        
        # Get detailed info based on type
        if entity_type == "stablecoin":
            info = chain_query.get_stablecoin_info(address)
            if info:
                print(f"  Details: {info['symbol']} ({info['unit']}) - {info['decimals']} decimals")
        elif entity_type == "cex":
            info = chain_query.get_cex_info(address)
            if info:
                print(f"  Details: {info['name']} ({info['exchange']})")
        elif entity_type == "etf":
            info = chain_query.get_etf_info(address)
            if info:
                print(f"  Details: {info['name']} ({info['provider']})")


if __name__ == "__main__":
    try:
        analyze_stablecoin_market_by_unit()
        test_entity_identification()
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)