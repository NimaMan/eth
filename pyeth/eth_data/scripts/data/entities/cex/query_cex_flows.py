#!/usr/bin/env python3
"""
Example script demonstrating how to query CEX (Centralized Exchange) flow data.

This shows how to:
- Get current exchange balances
- Track deposits and withdrawals
- Analyze flow patterns
- Calculate market share
- Detect unusual movements
"""

from eth_data.database.reth_chain_queries.entities.cex_queries import CEXQueries
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


def main():
    print("\n" + "="*80)
    print("CEX FLOW ANALYSIS EXAMPLES")
    print("="*80)
    
    # Initialize CEX queries
    cex_queries = CEXQueries()
    
    # Example 1: Current Exchange Balances
    print("\n1. Current Exchange Balances:")
    print("-" * 50)
    
    all_balances = cex_queries.get_all_cex_balances()
    total_cex_eth = sum(all_balances.values())
    
    # Sort by balance
    sorted_exchanges = sorted(all_balances.items(), key=lambda x: x[1], reverse=True)
    
    print(f"  Total ETH in CEXs: {format_eth(total_cex_eth)} ETH")
    print(f"  Number of Exchanges: {len(all_balances)}\n")
    
    # Show top 10
    print("  Top 10 Exchanges by ETH Holdings:")
    for exchange, balance in sorted_exchanges[:10]:
        market_share = (balance / total_cex_eth * 100) if total_cex_eth > 0 else 0
        print(f"    {exchange:20} {format_eth(balance):>12} ETH ({market_share:5.2f}%)")
    
    # Example 2: 24-Hour Flow Analysis
    print("\n2. 24-Hour CEX Flows:")
    print("-" * 50)
    
    flows_24h = cex_queries.get_exchange_flows(hours_back=24)
    
    print(f"  Net Flow:       {format_eth(flows_24h['net_flow']):>15} ETH")
    print(f"  Total Deposits: {format_eth(flows_24h['total_deposits']):>15} ETH")
    print(f"  Total Withdrawals: {format_eth(flows_24h['total_withdrawals']):>12} ETH")
    print(f"  Sentiment:      {flows_24h['sentiment'].upper()}")
    print(f"  Reason:         {flows_24h['sentiment_reason']}")
    
    # Top deposits
    if flows_24h['top_deposits']:
        print("\n  Top Deposits (24h):")
        for item in flows_24h['top_deposits'][:3]:
            print(f"    {item['exchange']:20} +{format_eth(item['amount'])} ETH")
    
    # Top withdrawals
    if flows_24h['top_withdrawals']:
        print("\n  Top Withdrawals (24h):")
        for item in flows_24h['top_withdrawals'][:3]:
            print(f"    {item['exchange']:20} -{format_eth(item['amount'])} ETH")
    
    # Example 3: Multi-Period Analysis
    print("\n3. Multi-Period Flow Analysis:")
    print("-" * 50)
    
    # Get flows for different periods
    daily_flows = cex_queries.get_daily_flows(days_back=1)
    weekly_flows = cex_queries.get_weekly_flows(weeks_back=1)
    
    print(f"  {'Period':<10} {'Net Flow':>15} {'Inflows':>15} {'Outflows':>15}")
    print(f"  {'------':<10} {'--------':>15} {'-------':>15} {'--------':>15}")
    
    print(f"  {'24 Hours':<10} {format_eth(daily_flows.get('total_net_flow_eth', 0)):>15} "
          f"{format_eth(daily_flows.get('total_inflow_eth', 0)):>15} "
          f"{format_eth(daily_flows.get('total_outflow_eth', 0)):>15}")
    
    print(f"  {'7 Days':<10} {format_eth(weekly_flows.get('total_net_flow_eth', 0)):>15} "
          f"{format_eth(weekly_flows.get('total_inflow_eth', 0)):>15} "
          f"{format_eth(weekly_flows.get('total_outflow_eth', 0)):>15}")
    
    # Example 4: Exchange Market Share
    print("\n4. Exchange Market Share Analysis:")
    print("-" * 50)
    
    market_shares = cex_queries.get_market_share()
    concentration_risk = cex_queries.calculate_concentration_risk()
    
    print(f"  Market Concentration Metrics:")
    print(f"    HHI Index:           {concentration_risk['hhi']:.0f}")
    print(f"    Top 3 Concentration: {concentration_risk['top_3_concentration']:.1f}%")
    print(f"    Exchange Count:      {concentration_risk['exchange_count']}")
    print(f"    Risk Level:          {concentration_risk['risk_level'].upper()}")
    
    # Example 5: Specific Exchange Analysis
    print("\n5. Binance Specific Analysis:")
    print("-" * 50)
    
    binance_balance = cex_queries.get_exchange_balance("Binance")
    print(f"  Current Balance: {format_eth(binance_balance)} ETH")
    
    # Get recent blocks for flow analysis
    latest_block = cex_queries.query.get_latest_block()
    from_block = latest_block - 7200  # ~24 hours
    
    # Analyze deposit patterns
    deposit_patterns = cex_queries.analyze_deposit_patterns(
        "Binance", from_block, latest_block, min_eth=100
    )
    
    print(f"\n  Deposit Patterns (24h):")
    print(f"    Total Deposits:    {format_eth(deposit_patterns['total_deposits'])} ETH")
    print(f"    Deposit Count:     {deposit_patterns['deposit_count']}")
    if deposit_patterns['deposit_count'] > 0:
        print(f"    Avg Deposit Size:  {format_eth(deposit_patterns['avg_deposit_size'])} ETH")
    print(f"    Large Deposits:    {deposit_patterns['large_deposits']} (>100 ETH)")
    
    # Analyze withdrawal patterns
    withdrawal_patterns = cex_queries.analyze_withdrawal_patterns(
        "Binance", from_block, latest_block, min_eth=100
    )
    
    print(f"\n  Withdrawal Patterns (24h):")
    print(f"    Total Withdrawals: {format_eth(withdrawal_patterns['total_withdrawals'])} ETH")
    print(f"    Withdrawal Count:  {withdrawal_patterns['withdrawal_count']}")
    if withdrawal_patterns['withdrawal_count'] > 0:
        print(f"    Avg Withdrawal:    {format_eth(withdrawal_patterns['avg_withdrawal_size'])} ETH")
    print(f"    Large Withdrawals: {withdrawal_patterns['large_withdrawals']} (>100 ETH)")
    
    # Example 6: Top Exchanges by Holdings
    print("\n6. Top ETH Holders Among Exchanges:")
    print("-" * 50)
    
    top_exchanges = cex_queries.get_top_exchanges(n=5)
    
    print(f"  {'Exchange':<20} {'ETH Holdings':>15} {'Market Share':>12}")
    print(f"  {'--------':<20} {'------------':>15} {'------------':>12}")
    
    for exchange, amount, share in top_exchanges:
        print(f"  {exchange:<20} {format_eth(amount):>15} {share:>11.2f}%")
    
    # Example 7: Historical Balance Tracking (30 days, weekly samples)
    print("\n7. Historical Balance Trends (30 days):")
    print("-" * 50)
    
    # Get balance history for all exchanges
    history = cex_queries.get_balance_history(
        exchange=None,  # All exchanges
        from_block=latest_block - (30 * 7200),  # 30 days
        to_block=latest_block,
        sample_interval=7 * 7200  # Weekly samples
    )
    
    print(f"  {'Week':<8} {'Block':>10} {'Total Balance':>20}")
    print(f"  {'----':<8} {'-----':>10} {'-------------':>20}")
    
    for i, snapshot in enumerate(history):
        week_label = f"Week {i+1}"
        print(f"  {week_label:<8} {snapshot['block']:>10,} "
              f"{format_eth(snapshot['total_balance']):>20} ETH")
    
    # Example 8: Check if address is CEX
    print("\n8. Address Classification:")
    print("-" * 50)
    
    test_addresses = [
        "0x28C6c06298d514Db089934071355E5743bf21d60",  # Binance 14
        "0x503828976D22510aad0201ac7EC88293211D23Da",  # Coinbase 6
        "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",  # Not CEX (Uniswap pool)
    ]
    
    for addr in test_addresses:
        is_cex = cex_queries.is_cex_address(addr)
        if is_cex:
            info = cex_queries.get_cex_info(addr)
            exchange = info.get('exchange', info.get('name', 'Unknown'))
            label = info.get('name', 'Unknown')
            print(f"  {addr[:10]}...{addr[-8:]} : {exchange} - {label}")
        else:
            print(f"  {addr[:10]}...{addr[-8:]} : Not a CEX address")
    
    # Example 9: Dashboard-Ready Data
    print("\n9. Dashboard Summary (JSON Format):")
    print("-" * 50)
    
    dashboard_data = {
        'timestamp': datetime.now().isoformat(),
        'total_balance': total_cex_eth,
        'exchange_count': len(all_balances),
        'flows_24h': {
            'net': flows_24h['net_flow'],
            'deposits': flows_24h['total_deposits'],
            'withdrawals': flows_24h['total_withdrawals'],
            'sentiment': flows_24h['sentiment']
        },
        'top_exchanges': [
            {'name': ex, 'balance': bal, 'share': share}
            for ex, bal, share in top_exchanges[:3]
        ],
        'concentration': {
            'hhi': concentration_risk['hhi'],
            'risk': concentration_risk['risk_level']
        }
    }
    
    print(json.dumps(dashboard_data, indent=2, default=str))
    
    print("\n" + "="*80)
    print("CEX ANALYSIS COMPLETED")
    print("="*80)


if __name__ == "__main__":
    main()