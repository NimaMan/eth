#!/usr/bin/env python3
"""
Entity flows analysis using ChainQuery with time conversion.

This example demonstrates:
- Analyzing ETF, CEX, and stablecoin flows
- Using time conversion for period-based analysis
- Aggregating flows by different time periods
- Cross-entity correlation analysis
"""

import pyreth
from datetime import datetime, timedelta, timezone
import json
from typing import Dict, List, Tuple


class EntityFlowAnalyzer:
    """Analyzer for entity flows using ChainQuery."""
    
    def __init__(self):
        """Initialize with PyReth singleton."""
        self.reth = pyreth.PyReth()
        self.query = self.reth.chain_query()
    
    def analyze_period(
        self, 
        period_name: str, 
        start_time: datetime, 
        end_time: datetime
    ) -> Dict:
        """Analyze all entity flows for a time period."""
        print(f"\n{'='*60}")
        print(f"📊 Analyzing: {period_name}")
        print(f"{'='*60}")
        
        # Convert time to blocks
        start_iso = start_time.isoformat().replace('+00:00', 'Z')
        end_iso = end_time.isoformat().replace('+00:00', 'Z')
        
        start_block, end_block = self.query.get_blocks_for_time_range(
            start_iso, end_iso
        )
        
        print(f"Time: {start_time:%Y-%m-%d %H:%M} to {end_time:%Y-%m-%d %H:%M}")
        print(f"Blocks: {start_block:,} to {end_block:,} ({end_block - start_block:,} blocks)")
        
        results = {
            'period': period_name,
            'start_time': start_iso,
            'end_time': end_iso,
            'start_block': start_block,
            'end_block': end_block,
            'block_count': end_block - start_block,
        }
        
        # Analyze each entity type
        results['etf'] = self.analyze_etf_flows(start_block, end_block)
        results['cex'] = self.analyze_cex_flows(start_block, end_block)
        results['stablecoin'] = self.analyze_stablecoin_supply(start_block, end_block)
        
        # Cross-entity analysis
        results['correlation'] = self.analyze_correlation(results)
        
        return results
    
    def analyze_etf_flows(self, start_block: int, end_block: int) -> Dict:
        """Analyze ETF flows for a block range."""
        print("\n📈 ETF Flows:")
        print("-" * 40)
        
        try:
            flows = self.query.calculate_etf_flows_between_blocks(
                start_block, end_block
            )
            
            # Find top movers
            top_inflows = []
            top_outflows = []
            
            if 'providers' in flows:
                for provider, data in flows['providers'].items():
                    if data.get('inflow_eth', 0) > 0:
                        top_inflows.append((provider, data['inflow_eth']))
                    if data.get('outflow_eth', 0) > 0:
                        top_outflows.append((provider, data['outflow_eth']))
                
                top_inflows.sort(key=lambda x: x[1], reverse=True)
                top_outflows.sort(key=lambda x: x[1], reverse=True)
                
                # Display top 3
                if top_inflows:
                    print("  Top Inflows:")
                    for provider, amount in top_inflows[:3]:
                        print(f"    ↗️  {provider}: {amount:.2f} ETH")
                
                if top_outflows:
                    print("  Top Outflows:")
                    for provider, amount in top_outflows[:3]:
                        print(f"    ↘️  {provider}: {amount:.2f} ETH")
            
            total_in = flows.get('total_inflow', 0)
            total_out = flows.get('total_outflow', 0)
            net = flows.get('net_flow', 0)
            
            print(f"\n  Totals:")
            print(f"    Inflow:  {total_in:,.2f} ETH")
            print(f"    Outflow: {total_out:,.2f} ETH")
            print(f"    Net:     {net:+,.2f} ETH")
            
            return {
                'total_inflow': total_in,
                'total_outflow': total_out,
                'net_flow': net,
                'top_inflows': top_inflows[:3],
                'top_outflows': top_outflows[:3],
            }
            
        except Exception as e:
            print(f"  Error: {e}")
            return {}
    
    def analyze_cex_flows(self, start_block: int, end_block: int) -> Dict:
        """Analyze CEX flows for a block range."""
        print("\n💱 CEX Flows:")
        print("-" * 40)
        
        try:
            flows = self.query.calculate_cex_flows_between_blocks(
                start_block, end_block
            )
            
            # Find top movers
            top_deposits = []
            top_withdrawals = []
            
            if 'exchanges' in flows:
                for exchange, data in flows['exchanges'].items():
                    if data.get('inflow_eth', 0) > 0:
                        top_deposits.append((exchange, data['inflow_eth']))
                    if data.get('outflow_eth', 0) > 0:
                        top_withdrawals.append((exchange, data['outflow_eth']))
                
                top_deposits.sort(key=lambda x: x[1], reverse=True)
                top_withdrawals.sort(key=lambda x: x[1], reverse=True)
                
                # Display top 3
                if top_deposits:
                    print("  Top Deposits:")
                    for exchange, amount in top_deposits[:3]:
                        print(f"    ↗️  {exchange}: {amount:.2f} ETH")
                
                if top_withdrawals:
                    print("  Top Withdrawals:")
                    for exchange, amount in top_withdrawals[:3]:
                        print(f"    ↘️  {exchange}: {amount:.2f} ETH")
            
            total_in = flows.get('total_inflow', 0)
            total_out = flows.get('total_outflow', 0)
            net = flows.get('net_flow', 0)
            
            print(f"\n  Totals:")
            print(f"    Deposits:    {total_in:,.2f} ETH")
            print(f"    Withdrawals: {total_out:,.2f} ETH")
            print(f"    Net:         {net:+,.2f} ETH")
            
            return {
                'total_deposits': total_in,
                'total_withdrawals': total_out,
                'net_flow': net,
                'top_deposits': top_deposits[:3],
                'top_withdrawals': top_withdrawals[:3],
            }
            
        except Exception as e:
            print(f"  Error: {e}")
            return {}
    
    def analyze_stablecoin_supply(self, start_block: int, end_block: int) -> Dict:
        """Analyze stablecoin supply changes for a block range."""
        print("\n💵 Stablecoin Supply Changes:")
        print("-" * 40)
        
        try:
            changes = self.query.calculate_stablecoin_supply_changes_between_blocks(
                start_block, end_block
            )
            
            # Find significant changes
            significant_changes = []
            
            if 'token_changes' in changes:
                for token, data in changes['token_changes'].items():
                    if abs(data.get('net_change', 0)) > 1000:
                        significant_changes.append((
                            token,
                            data.get('net_change', 0),
                            data.get('percent_change', 0)
                        ))
                
                significant_changes.sort(key=lambda x: abs(x[1]), reverse=True)
                
                # Display top changes
                if significant_changes:
                    print("  Significant Changes:")
                    for token, net_change, pct_change in significant_changes[:5]:
                        emoji = "📈" if net_change > 0 else "📉"
                        print(f"    {emoji} {token}: ${net_change:+,.0f} ({pct_change:+.2f}%)")
            
            total_minted = changes.get('total_minted', 0)
            total_burned = changes.get('total_burned', 0)
            net_change = changes.get('total_net_change', 0)
            
            print(f"\n  Totals:")
            print(f"    Minted: ${total_minted:,.0f}")
            print(f"    Burned: ${total_burned:,.0f}")
            print(f"    Net:    ${net_change:+,.0f}")
            
            return {
                'total_minted': total_minted,
                'total_burned': total_burned,
                'net_change': net_change,
                'significant_changes': significant_changes[:5],
            }
            
        except Exception as e:
            print(f"  Error: {e}")
            return {}
    
    def analyze_correlation(self, results: Dict) -> Dict:
        """Analyze correlation between different entity flows."""
        print("\n🔄 Cross-Entity Analysis:")
        print("-" * 40)
        
        etf_net = results.get('etf', {}).get('net_flow', 0)
        cex_net = results.get('cex', {}).get('net_flow', 0)
        stable_net = results.get('stablecoin', {}).get('net_change', 0)
        
        # Calculate institutional flow
        institutional_net = etf_net + cex_net
        
        print(f"  Institutional Flow: {institutional_net:+,.2f} ETH")
        print(f"    (ETF: {etf_net:+,.2f}, CEX: {cex_net:+,.2f})")
        print(f"  Stablecoin Response: ${stable_net:+,.0f}")
        
        # Determine correlation
        if abs(institutional_net) < 10 or abs(stable_net) < 10000:
            correlation = "Neutral (low activity)"
        elif (institutional_net > 0 and stable_net > 0):
            correlation = "Positive (both increasing)"
        elif (institutional_net < 0 and stable_net < 0):
            correlation = "Positive (both decreasing)"
        else:
            correlation = "Negative (opposite directions)"
        
        print(f"  Correlation: {correlation}")
        
        # Market sentiment
        if institutional_net > 100 and stable_net > 1_000_000:
            sentiment = "🟢 Bullish"
            description = "Institutional accumulation + stablecoin minting"
        elif institutional_net < -100 and stable_net < -1_000_000:
            sentiment = "🔴 Bearish"
            description = "Institutional distribution + stablecoin burning"
        elif institutional_net > 50:
            sentiment = "🟡 Cautiously Bullish"
            description = "Moderate institutional inflows"
        elif institutional_net < -50:
            sentiment = "🟡 Cautiously Bearish"
            description = "Moderate institutional outflows"
        else:
            sentiment = "⚪ Neutral"
            description = "Mixed signals"
        
        print(f"  Market Sentiment: {sentiment}")
        print(f"    {description}")
        
        return {
            'institutional_net': institutional_net,
            'stablecoin_net': stable_net,
            'correlation': correlation,
            'sentiment': sentiment,
            'description': description,
        }


def main():
    """Run comprehensive entity flow analysis."""
    print("🚀 Entity Flow Analysis with Time Conversion")
    print("=" * 60)
    
    analyzer = EntityFlowAnalyzer()
    
    # Define analysis periods
    now = datetime.now(timezone.utc)
    periods = [
        ("Last Hour", now - timedelta(hours=1), now),
        ("Last 24 Hours", now - timedelta(days=1), now),
        ("Last 7 Days", now - timedelta(days=7), now),
    ]
    
    all_results = []
    
    for period_name, start_time, end_time in periods:
        try:
            results = analyzer.analyze_period(period_name, start_time, end_time)
            all_results.append(results)
        except Exception as e:
            print(f"\n❌ Error analyzing {period_name}: {e}")
    
    # Summary
    print("\n" + "=" * 60)
    print("📊 Analysis Summary")
    print("=" * 60)
    
    for result in all_results:
        period = result['period']
        correlation = result.get('correlation', {})
        
        print(f"\n{period}:")
        print(f"  Blocks analyzed: {result['block_count']:,}")
        print(f"  Sentiment: {correlation.get('sentiment', 'Unknown')}")
        print(f"  ETF Net: {result.get('etf', {}).get('net_flow', 0):+,.2f} ETH")
        print(f"  CEX Net: {result.get('cex', {}).get('net_flow', 0):+,.2f} ETH")
        print(f"  Stablecoin Net: ${result.get('stablecoin', {}).get('net_change', 0):+,.0f}")
    
    # Save results to file
    output_file = f"entity_flows_{datetime.now():%Y%m%d_%H%M%S}.json"
    with open(output_file, 'w') as f:
        json.dump(all_results, f, indent=2, default=str)
    
    print(f"\n💾 Results saved to: {output_file}")
    print("\n✅ Analysis Complete!")


if __name__ == "__main__":
    main()