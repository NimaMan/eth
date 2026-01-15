#!/usr/bin/env python3
"""
Example script demonstrating how to query ETF holdings and flow data.

This shows how to:
- Get current ETF provider holdings
- Track inflows and outflows
- Compare different providers
- Detect large transfers
- Analyze cumulative flows over time
"""

from eth_data.database.reth_chain_queries.entities.etf_queries import ETFQueries
from datetime import datetime
import json


def format_eth(wei_or_eth: float) -> str:
    """Format ETH amount for display."""
    if abs(wei_or_eth) >= 1e6:
        return f"{wei_or_eth/1e6:.2f}M"
    elif abs(wei_or_eth) >= 1e3:
        return f"{wei_or_eth/1e3:.2f}K"
    else:
        return f"{wei_or_eth:.2f}"


def format_usd(eth_amount: float, eth_price: float = 3500) -> str:
    """Convert ETH to USD for AUM display."""
    usd_value = eth_amount * eth_price
    if usd_value >= 1e9:
        return f"${usd_value/1e9:.2f}B"
    elif usd_value >= 1e6:
        return f"${usd_value/1e6:.2f}M"
    else:
        return f"${usd_value/1e3:.2f}K"


def main():
    print("\n" + "="*80)
    print("ETF HOLDINGS AND FLOW ANALYSIS")
    print("="*80)
    
    # Initialize ETF queries
    etf_queries = ETFQueries()
    
    # Assumed ETH price for AUM calculations
    ETH_PRICE = 3500
    
    # Example 1: Current ETF Holdings
    print("\n1. Current ETF Provider Holdings:")
    print("-" * 50)
    
    all_holdings = etf_queries.get_all_etf_holdings()
    total_etf_eth = sum(all_holdings.values())
    
    print(f"  Total ETH in ETFs:  {format_eth(total_etf_eth)} ETH")
    print(f"  Total AUM (at ${ETH_PRICE}): {format_usd(total_etf_eth, ETH_PRICE)}")
    print(f"  Number of Providers: {len(all_holdings)}\n")
    
    # Show all providers sorted by holdings
    sorted_providers = sorted(all_holdings.items(), key=lambda x: x[1], reverse=True)
    
    print("  ETF Provider Holdings:")
    print(f"  {'Provider':<20} {'ETH Holdings':>15} {'AUM':>15} {'Market Share':>12}")
    print(f"  {'--------':<20} {'------------':>15} {'---':>15} {'------------':>12}")
    
    for provider, holdings in sorted_providers:
        market_share = (holdings / total_etf_eth * 100) if total_etf_eth > 0 else 0
        print(f"  {provider:<20} {format_eth(holdings):>15} "
              f"{format_usd(holdings, ETH_PRICE):>15} {market_share:>11.2f}%")
    
    # Example 2: 24-Hour Flow Analysis
    print("\n2. 24-Hour ETF Flows:")
    print("-" * 50)
    
    flows_24h = etf_queries.get_flows_summary(hours_back=24)
    
    print(f"  Net Flow:        {format_eth(flows_24h['net_flow']):>15} ETH")
    print(f"  Total Inflows:   {format_eth(flows_24h['total_inflow']):>15} ETH")
    print(f"  Total Outflows:  {format_eth(flows_24h['total_outflow']):>15} ETH")
    print(f"  Flow Direction:  {flows_24h['sentiment'].upper()}")
    
    # Top inflows
    if flows_24h['top_inflows']:
        print("\n  Top Inflows (24h):")
        for item in flows_24h['top_inflows']:
            print(f"    {item['provider']:20} +{format_eth(item['net'])} ETH "
                  f"({format_usd(item['net'], ETH_PRICE)})")
    
    # Top outflows
    if flows_24h['top_outflows']:
        print("\n  Top Outflows (24h):")
        for item in flows_24h['top_outflows']:
            print(f"    {item['provider']:20} -{format_eth(item['net'])} ETH "
                  f"({format_usd(item['net'], ETH_PRICE)})")
    
    # Example 3: Multi-Period Comparison
    print("\n3. Multi-Period Flow Comparison:")
    print("-" * 50)
    
    # Get flows for different periods
    daily_flows = etf_queries.get_daily_flows(days_back=1)
    weekly_flows = etf_queries.get_weekly_flows(weeks_back=1)
    
    print(f"  {'Period':<10} {'Net Flow':>15} {'Inflows':>15} {'Outflows':>15} {'USD Value':>20}")
    print(f"  {'------':<10} {'--------':>15} {'-------':>15} {'--------':>15} {'---------':>20}")
    
    daily_net = daily_flows.get('total_net_flow_eth', 0)
    print(f"  {'24 Hours':<10} {format_eth(daily_net):>15} "
          f"{format_eth(daily_flows.get('total_inflow_eth', 0)):>15} "
          f"{format_eth(daily_flows.get('total_outflow_eth', 0)):>15} "
          f"{format_usd(daily_net, ETH_PRICE):>20}")
    
    weekly_net = weekly_flows.get('total_net_flow_eth', 0)
    print(f"  {'7 Days':<10} {format_eth(weekly_net):>15} "
          f"{format_eth(weekly_flows.get('total_inflow_eth', 0)):>15} "
          f"{format_eth(weekly_flows.get('total_outflow_eth', 0)):>15} "
          f"{format_usd(weekly_net, ETH_PRICE):>20}")
    
    # Example 4: Provider Market Share Analysis
    print("\n4. ETF Provider Market Share:")
    print("-" * 50)
    
    market_shares = etf_queries.get_market_share()
    top_providers = etf_queries.get_top_providers(n=5)
    
    print("  Top 5 ETF Providers by Market Share:\n")
    for provider, holdings, share in top_providers:
        print(f"  {provider:20}")
        print(f"    Holdings: {format_eth(holdings)} ETH")
        print(f"    AUM:      {format_usd(holdings, ETH_PRICE)}")
        print(f"    Share:    {share:.2f}%")
        print()
    
    # Example 5: Provider Comparison (Big 3)
    print("\n5. Big 3 Provider Comparison:")
    print("-" * 50)
    
    latest_block = etf_queries.query.get_latest_block()
    from_block = latest_block - 7200  # ~24 hours
    
    big_3 = ["BlackRock", "Grayscale", "Fidelity"]
    comparison = etf_queries.compare_providers(big_3, from_block, latest_block)
    
    print(f"  {'Provider':<15} {'Inflows':>12} {'Outflows':>12} {'Net Flow':>12} {'Direction':>10}")
    print(f"  {'--------':<15} {'-------':>12} {'--------':>12} {'--------':>12} {'---------':>10}")
    
    for provider in big_3:
        if provider in comparison:
            data = comparison[provider]
            inflow = data.get('inflow_eth', 0)
            outflow = data.get('outflow_eth', 0)
            net = data.get('net_flow_eth', 0)
            direction = '↗' if net > 0 else '↘' if net < 0 else '→'
            
            print(f"  {provider:<15} {format_eth(inflow):>12} "
                  f"{format_eth(outflow):>12} {format_eth(net):>12} {direction:>10}")
    
    # Example 6: Large Transfer Detection
    print("\n6. Large ETF Transfers (>1000 ETH):")
    print("-" * 50)
    
    # Check for large transfers in the past 24 hours
    large_transfers = etf_queries.get_large_transfers(
        from_block=latest_block - 7200,
        to_block=latest_block,
        min_eth_amount=1000
    )
    
    if large_transfers:
        print(f"  Found {len(large_transfers)} large transfers:\n")
        for transfer in large_transfers[:5]:  # Show top 5
            print(f"  {transfer['provider']:20} "
                  f"{transfer['type'].upper():8} "
                  f"{format_eth(transfer['amount'])} ETH "
                  f"({format_usd(transfer['amount'], ETH_PRICE)})")
    else:
        print("  No large transfers detected in the past 24 hours")
    
    # Example 7: Cumulative Flow Analysis (30 days)
    print("\n7. Cumulative Flow Analysis (30 days):")
    print("-" * 50)
    
    cumulative_flows = etf_queries.get_cumulative_flows(
        provider=None,  # All providers
        from_block=latest_block - (30 * 7200),
        to_block=latest_block,
        sample_interval=7 * 7200  # Weekly samples
    )
    
    print("  Weekly Cumulative Flows:\n")
    print(f"  {'Week':<8} {'Cumulative In':>15} {'Cumulative Out':>15} {'Net Position':>15}")
    print(f"  {'----':<8} {'-------------':>15} {'--------------':>15} {'------------':>15}")
    
    for i, data in enumerate(cumulative_flows):
        week_label = f"Week {i+1}"
        print(f"  {week_label:<8} {format_eth(data['cumulative_inflow']):>15} "
              f"{format_eth(data['cumulative_outflow']):>15} "
              f"{format_eth(data['cumulative_net']):>15}")
    
    # Example 8: Check if address is ETF
    print("\n8. Address Classification:")
    print("-" * 50)
    
    test_addresses = [
        "0x9Db0FB0Aebe6A925B7838D16e3993A3976A64AaB",  # Example BlackRock
        "0x29d7ebCA656665C1A52a92F830E413E394db6b4F",  # Example Grayscale
        "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",  # Not ETF (Uniswap pool)
    ]
    
    for addr in test_addresses:
        is_etf = etf_queries.is_etf_address(addr)
        if is_etf:
            info = etf_queries.get_etf_info(addr)
            if info:
                print(f"  {addr[:10]}...{addr[-8:]} : {info.get('name', 'Unknown')} ETF")
        else:
            print(f"  {addr[:10]}...{addr[-8:]} : Not an ETF address")
    
    # Example 9: Flow Statistics
    print("\n9. Flow Statistics (7 days):")
    print("-" * 50)
    
    # Get flow statistics for the past week
    stats = etf_queries.get_flow_statistics(
        from_block=latest_block - (7 * 7200),
        to_block=latest_block
    )
    
    if 'total_inflow_eth' in stats:
        avg_daily_inflow = stats.get('total_inflow_eth', 0) / 7
        avg_daily_outflow = stats.get('total_outflow_eth', 0) / 7
        avg_daily_net = stats.get('total_net_flow_eth', 0) / 7
        
        print(f"  7-Day Averages:")
        print(f"    Daily Inflow:  {format_eth(avg_daily_inflow)} ETH ({format_usd(avg_daily_inflow, ETH_PRICE)})")
        print(f"    Daily Outflow: {format_eth(avg_daily_outflow)} ETH ({format_usd(avg_daily_outflow, ETH_PRICE)})")
        print(f"    Daily Net:     {format_eth(avg_daily_net)} ETH ({format_usd(avg_daily_net, ETH_PRICE)})")
    
    # Example 10: Dashboard-Ready JSON
    print("\n10. Dashboard Data (JSON Format):")
    print("-" * 50)
    
    dashboard_data = {
        'timestamp': datetime.now().isoformat(),
        'total_aum': {
            'eth': total_etf_eth,
            'usd': total_etf_eth * ETH_PRICE
        },
        'provider_count': len(all_holdings),
        'flows_24h': {
            'net_eth': flows_24h['net_flow'],
            'net_usd': flows_24h['net_flow'] * ETH_PRICE,
            'inflows': flows_24h['total_inflow'],
            'outflows': flows_24h['total_outflow'],
            'sentiment': flows_24h['sentiment']
        },
        'top_providers': [
            {
                'name': provider,
                'holdings_eth': holdings,
                'aum_usd': holdings * ETH_PRICE,
                'market_share': share
            }
            for provider, holdings, share in top_providers[:3]
        ],
        'flows_7d': {
            'net_eth': weekly_flows.get('total_net_flow_eth', 0),
            'net_usd': weekly_flows.get('total_net_flow_eth', 0) * ETH_PRICE
        }
    }
    
    print(json.dumps(dashboard_data, indent=2, default=str))
    
    print("\n" + "="*80)
    print("ETF ANALYSIS COMPLETED")
    print("="*80)


if __name__ == "__main__":
    main()