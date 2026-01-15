#!/usr/bin/env python3
"""
Combined dashboard example showing how to get CEX, ETF, and Stablecoin data together.

This demonstrates how the Ethereum Today dashboard aggregates all entity data
into a single comprehensive view of the Ethereum ecosystem.
"""

from eth_data.database.reth_chain_queries.entities.stablecoin_queries import StablecoinQueries
from eth_data.database.reth_chain_queries.entities.cex_queries import CEXQueries
from eth_data.database.reth_chain_queries.entities.etf_queries import ETFQueries
from eth_data.database.reth_chain_queries.entities.base_queries import TimePeriod
from datetime import datetime
import json
from typing import Dict, Any


def format_number(num: float, decimals: int = 2) -> str:
    """Format large numbers with appropriate units."""
    if abs(num) >= 1e12:
        return f"{num/1e12:.{decimals}f}T"
    elif abs(num) >= 1e9:
        return f"{num/1e9:.{decimals}f}B"
    elif abs(num) >= 1e6:
        return f"{num/1e6:.{decimals}f}M"
    elif abs(num) >= 1e3:
        return f"{num/1e3:.{decimals}f}K"
    else:
        return f"{num:.{decimals}f}"


def get_stablecoin_dashboard_data(stablecoin_queries: StablecoinQueries) -> Dict[str, Any]:
    """Get stablecoin data formatted for dashboard."""
    # Get market overview
    overview = stablecoin_queries.get_market_overview(hours_back=24)
    
    # Get supply changes for multiple periods
    latest_block = stablecoin_queries.get_latest_block()
    changes_1d = stablecoin_queries.get_supply_changes_between_blocks(
        latest_block - 7200, latest_block  # ~24 hours
    )
    changes_7d = stablecoin_queries.get_supply_changes_between_blocks(
        latest_block - (7 * 7200), latest_block  # ~7 days
    )
    changes_30d = stablecoin_queries.get_supply_changes_between_blocks(
        latest_block - (30 * 7200), latest_block  # ~30 days
    )
    
    # Format token data
    tokens = {}
    for token in ['USDC', 'USDT', 'DAI']:
        token_data = overview.get('tokens', {}).get(token, {})
        tokens[token] = {
            'supply': token_data.get('total_supply', 0),
            'change_1d': changes_1d.get('tokens', {}).get(token, {}).get('change', 0),
            'change_7d': changes_7d.get('tokens', {}).get(token, {}).get('change', 0),
            'change_30d': changes_30d.get('tokens', {}).get(token, {}).get('change', 0),
        }
    
    # Calculate totals
    total_supply = sum(t['supply'] for t in tokens.values())
    total_change_1d = sum(t['change_1d'] for t in tokens.values())
    total_change_7d = sum(t['change_7d'] for t in tokens.values())
    total_change_30d = sum(t['change_30d'] for t in tokens.values())
    
    # Generate insight
    if total_change_1d > 1e9:
        insight = f"Stablecoin expansion: +${format_number(total_change_1d)} in 24h signals risk-on sentiment"
    elif total_change_1d < -1e9:
        insight = f"Stablecoin contraction: -${format_number(abs(total_change_1d))} in 24h indicates de-risking"
    else:
        insight = "Stablecoin supply stable, market in equilibrium"
    
    return {
        'total_supply': total_supply,
        'tokens': tokens,
        'total_changes': {
            '1d': total_change_1d,
            '7d': total_change_7d,
            '30d': total_change_30d
        },
        'insight': insight,
        'health_status': overview.get('health_status', 'unknown')
    }


def get_cex_dashboard_data(cex_queries: CEXQueries) -> Dict[str, Any]:
    """Get CEX flow data formatted for dashboard."""
    # Get current balances
    all_balances = cex_queries.get_all_cex_balances()
    
    # Get flows for different periods
    flows_1d = cex_queries.get_flows_for_period(TimePeriod.DAILY, 1)
    flows_7d = cex_queries.get_flows_for_period(TimePeriod.DAILY, 7)
    flows_30d = cex_queries.get_flows_for_period(TimePeriod.DAILY, 30)
    
    # Format exchange data
    exchanges = {}
    for exchange in ['Binance', 'Coinbase', 'Kraken']:
        balance = all_balances.get(exchange, 0)
        exchange_flows = flows_1d.get('exchange_flows', {}).get(exchange, {})
        
        exchanges[exchange] = {
            'balance': balance,
            'flow_1d': exchange_flows.get('net_flow_eth', 0),
            'flow_7d': flows_7d.get('exchange_flows', {}).get(exchange, {}).get('net_flow_eth', 0),
            'flow_30d': flows_30d.get('exchange_flows', {}).get(exchange, {}).get('net_flow_eth', 0)
        }
    
    # Calculate total flows
    total_balance = sum(all_balances.values())
    total_flow_1d = flows_1d.get('total_net_flow_eth', 0)
    total_flow_7d = flows_7d.get('total_net_flow_eth', 0)
    total_flow_30d = flows_30d.get('total_net_flow_eth', 0)
    
    # Generate insight
    if total_flow_1d < -10000:
        insight = f"Heavy withdrawals ({format_number(abs(total_flow_1d))} ETH) signal accumulation phase. Bullish for ETH price"
    elif total_flow_1d > 10000:
        insight = f"Large deposits ({format_number(total_flow_1d)} ETH) suggest potential selling pressure ahead"
    else:
        insight = "Balanced CEX flows indicate stable market conditions"
    
    return {
        'total_balance': total_balance,
        'exchanges': exchanges,
        'total_flows': {
            '1d': total_flow_1d,
            '7d': total_flow_7d,
            '30d': total_flow_30d
        },
        'net_flow_24h': total_flow_1d,
        'insight': insight
    }


def get_etf_dashboard_data(etf_queries: ETFQueries) -> Dict[str, Any]:
    """Get ETF holdings data formatted for dashboard."""
    # Get current holdings
    all_holdings = etf_queries.get_all_etf_holdings()
    
    # Get flows for different periods
    flows_1d = etf_queries.get_flows_for_period(TimePeriod.DAILY, 1)
    flows_7d = etf_queries.get_flows_for_period(TimePeriod.DAILY, 7)
    flows_30d = etf_queries.get_flows_for_period(TimePeriod.DAILY, 30)
    
    # Format provider data
    providers = {}
    for provider in ['BlackRock', 'Grayscale', 'Fidelity']:
        holdings = all_holdings.get(provider, 0)
        provider_flows = flows_1d.get('provider_flows', {}).get(provider, {})
        
        providers[provider] = {
            'aum': holdings * 3500,  # Convert to USD at $3500/ETH
            'flow_1d': provider_flows.get('net_flow_eth', 0),
            'flow_7d': flows_7d.get('provider_flows', {}).get(provider, {}).get('net_flow_eth', 0),
            'flow_30d': flows_30d.get('provider_flows', {}).get(provider, {}).get('net_flow_eth', 0)
        }
    
    # Calculate totals
    total_holdings = sum(all_holdings.values())
    total_aum = total_holdings * 3500
    total_flow_1d = flows_1d.get('total_net_flow_eth', 0)
    total_flow_7d = flows_7d.get('total_net_flow_eth', 0)
    total_flow_30d = flows_30d.get('total_net_flow_eth', 0)
    
    # Generate insight
    if total_flow_1d > 1000:
        insight = f"Strong ETF inflows (+{format_number(total_flow_1d)} ETH) show institutional accumulation"
    elif total_flow_1d < -1000:
        insight = f"ETF outflows (-{format_number(abs(total_flow_1d))} ETH) suggest profit-taking"
    else:
        insight = "Steady ETF holdings reflect stable institutional interest"
    
    return {
        'total_aum': total_aum,
        'providers': providers,
        'total_flows': {
            '1d': total_flow_1d,
            '7d': total_flow_7d,
            '30d': total_flow_30d
        },
        'net_flow_24h': total_flow_1d,
        'insight': insight
    }


def main():
    print("\n" + "="*80)
    print("ETHEREUM TODAY - COMPREHENSIVE DASHBOARD")
    print("="*80)
    
    # Initialize all query classes (they share PyReth instance)
    stablecoin_queries = StablecoinQueries()
    cex_queries = CEXQueries()
    etf_queries = ETFQueries()
    
    print("\nGathering data from all sources...")
    
    # Get dashboard data for each category
    stablecoin_data = get_stablecoin_dashboard_data(stablecoin_queries)
    cex_data = get_cex_dashboard_data(cex_queries)
    etf_data = get_etf_dashboard_data(etf_queries)
    
    # Display formatted dashboard
    print("\n" + "="*80)
    print("📊 STABLECOIN MARKET")
    print("="*80)
    
    print(f"Total Supply: ${format_number(stablecoin_data['total_supply'])}")
    print(f"24h Change:   {'+' if stablecoin_data['total_changes']['1d'] >= 0 else ''}${format_number(stablecoin_data['total_changes']['1d'])}")
    print(f"Status:       {stablecoin_data['health_status'].upper()}")
    print(f"\nTop Stablecoins:")
    
    for token, data in stablecoin_data['tokens'].items():
        print(f"  {token:6} ${format_number(data['supply']):>8} "
              f"(24h: {'+' if data['change_1d'] >= 0 else ''}${format_number(data['change_1d'])})")
    
    print(f"\n💡 {stablecoin_data['insight']}")
    
    print("\n" + "="*80)
    print("🏦 CEX FLOWS")
    print("="*80)
    
    print(f"Total Balance: {format_number(cex_data['total_balance'])} ETH")
    print(f"24h Net Flow:  {'+' if cex_data['net_flow_24h'] >= 0 else ''}{format_number(cex_data['net_flow_24h'])} ETH")
    print(f"\nExchange Balances & Flows:")
    
    for exchange, data in cex_data['exchanges'].items():
        flow_indicator = '↗' if data['flow_1d'] > 0 else '↘' if data['flow_1d'] < 0 else '→'
        print(f"  {exchange:10} {format_number(data['balance']):>8} ETH "
              f"({flow_indicator} {format_number(abs(data['flow_1d']))} ETH/24h)")
    
    print(f"\n💡 {cex_data['insight']}")
    
    print("\n" + "="*80)
    print("💼 ETF HOLDINGS")
    print("="*80)
    
    print(f"Total AUM:    ${format_number(etf_data['total_aum'])}")
    print(f"24h Net Flow: {'+' if etf_data['net_flow_24h'] >= 0 else ''}{format_number(etf_data['net_flow_24h'])} ETH")
    print(f"\nProvider Holdings & Flows:")
    
    for provider, data in etf_data['providers'].items():
        flow_indicator = '↗' if data['flow_1d'] > 0 else '↘' if data['flow_1d'] < 0 else '→'
        print(f"  {provider:10} ${format_number(data['aum']):>8} "
              f"({flow_indicator} {format_number(abs(data['flow_1d']))} ETH/24h)")
    
    print(f"\n💡 {etf_data['insight']}")
    
    # Create combined JSON for API/Dashboard
    print("\n" + "="*80)
    print("📋 COMPLETE DASHBOARD DATA (JSON)")
    print("="*80)
    
    dashboard_json = {
        'timestamp': datetime.now().isoformat(),
        'summary': {
            'stablecoin_supply': stablecoin_data['total_supply'],
            'cex_balance': cex_data['total_balance'],
            'etf_aum': etf_data['total_aum']
        },
        'flows_24h': {
            'stablecoin_change': stablecoin_data['total_changes']['1d'],
            'cex_net_flow': cex_data['net_flow_24h'],
            'etf_net_flow': etf_data['net_flow_24h']
        },
        'stablecoins': stablecoin_data,
        'cex': cex_data,
        'etf': etf_data,
        'insights': {
            'stablecoins': stablecoin_data['insight'],
            'cex': cex_data['insight'],
            'etf': etf_data['insight']
        }
    }
    
    print(json.dumps(dashboard_json, indent=2, default=str))
    
    # Market overview summary
    print("\n" + "="*80)
    print("📈 MARKET OVERVIEW")
    print("="*80)
    
    # Determine overall market sentiment
    stablecoin_expanding = stablecoin_data['total_changes']['1d'] > 1e9
    cex_withdrawing = cex_data['net_flow_24h'] < -5000
    etf_inflowing = etf_data['net_flow_24h'] > 500
    
    bullish_signals = sum([stablecoin_expanding, cex_withdrawing, etf_inflowing])
    
    if bullish_signals >= 2:
        sentiment = "BULLISH 🟢"
        reason = "Multiple positive indicators across stablecoins, CEX flows, and ETF holdings"
    elif bullish_signals <= 1:
        sentiment = "BEARISH 🔴"
        reason = "Weak signals across major market indicators"
    else:
        sentiment = "NEUTRAL 🟡"
        reason = "Mixed signals - market in transition phase"
    
    print(f"\nOverall Market Sentiment: {sentiment}")
    print(f"Reason: {reason}")
    
    print("\nKey Metrics (24h):")
    print(f"  • Stablecoin Supply:  {'+' if stablecoin_data['total_changes']['1d'] >= 0 else ''}${format_number(stablecoin_data['total_changes']['1d'])}")
    print(f"  • CEX Net Flow:       {'+' if cex_data['net_flow_24h'] >= 0 else ''}{format_number(cex_data['net_flow_24h'])} ETH")
    print(f"  • ETF Net Flow:       {'+' if etf_data['net_flow_24h'] >= 0 else ''}{format_number(etf_data['net_flow_24h'])} ETH")
    
    print("\n" + "="*80)
    print("DASHBOARD GENERATION COMPLETED")
    print("="*80)


if __name__ == "__main__":
    main()