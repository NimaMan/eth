"""
ETF query interface for accessing ETF holdings and flow data from Rust.

This module provides Python interface to ETF-related chain queries,
getting all calculations done in Rust and formatting results for Python use.
"""

from typing import Dict, List, Optional, Tuple
from datetime import datetime
from .base_queries import BaseEntityQuery, TimePeriod


class ETFQueries(BaseEntityQuery):
    """
    ETF-specific queries for holdings, flows, and provider analysis.
    
    All calculations are performed in Rust for performance.
    Python only formats and presents the data.
    """
    
    def __init__(self, reth_instance=None):
        """Initialize ETF queries with shared Reth instance."""
        super().__init__(reth_instance)
    
    # Basic ETF queries
    
    def is_etf_address(self, address: str) -> bool:
        """Check if an address belongs to an ETF provider."""
        return self.query.is_etf(address)
    
    def get_etf_info(self, address: str) -> Optional[Dict]:
        """
        Get ETF provider information for an address.
        
        Returns:
            Dict with provider name and address type, or None if not ETF
        """
        return self.query.get_etf_info(address)
    
    # Holdings queries
    
    def get_all_etf_holdings(self, block_number: Optional[int] = None) -> Dict[str, float]:
        """
        Get current ETH holdings for all ETF providers.
        
        Args:
            block_number: Block to query at (default: latest)
            
        Returns:
            Dict mapping provider name to ETH amount
        """
        raw_data = self.query.get_etf_holdings(block_number)
        # Convert list of provider dicts to simple name->balance mapping
        if isinstance(raw_data, dict) and 'providers' in raw_data:
            providers_list = raw_data.get('providers', [])
            return {p['name']: p['total_balance'] for p in providers_list}
        return {}
    
    def get_provider_holdings(
        self, 
        provider: str, 
        block_number: Optional[int] = None
    ) -> float:
        """
        Get ETH holdings for a specific provider.
        
        Args:
            provider: Provider name (e.g., 'BlackRock', 'Grayscale')
            block_number: Block to query at
            
        Returns:
            Total ETH held by provider
        """
        all_holdings = self.get_all_etf_holdings(block_number)
        return all_holdings.get(provider, 0.0)
    
    # Flow analysis
    
    def get_flows_between_blocks(
        self, 
        from_block: int, 
        to_block: int
    ) -> Dict:
        """
        Calculate ETF flows between two blocks.
        
        Args:
            from_block: Starting block number
            to_block: Ending block number
            
        Returns:
            Dict with flow data for each provider and totals
        """
        return self.query.calculate_etf_flows_between_blocks(from_block, to_block)
    
    def get_flows_for_period(
        self,
        period: TimePeriod,
        periods_back: int = 1,
        end_block: Optional[int] = None
    ) -> Dict:
        """
        Get ETF flows for a time period.
        
        Args:
            period: Time period (hourly, daily, weekly)
            periods_back: Number of periods to go back
            end_block: End block (default: latest)
            
        Returns:
            Flow data aggregated by period
        """
        if end_block is None:
            end_block = self.query.get_latest_block()
        
        blocks_per_period = period.blocks_per_period()
        from_block = end_block - (blocks_per_period * periods_back)
        
        return self.get_flows_between_blocks(from_block, end_block)
    
    def get_flows_summary(self, hours_back: int = 24) -> Dict:
        """
        Get ETF flows summary for dashboard.
        
        Returns formatted data with net flows, top providers, and trends.
        """
        start_block, end_block = self.convert_time_to_blocks(hours_back)
        flows = self.get_flows_between_blocks(start_block, end_block)
        
        # Extract provider flows
        provider_flows = flows.get('provider_flows', {})
        
        # Calculate top inflows and outflows
        top_inflows = []
        top_outflows = []
        
        for provider, data in provider_flows.items():
            inflow = data.get('inflow_eth', 0)
            outflow = data.get('outflow_eth', 0)
            net = data.get('net_flow_eth', 0)
            
            if net > 0:
                top_inflows.append({
                    'provider': provider,
                    'amount': inflow,
                    'net': net
                })
            elif net < 0:
                top_outflows.append({
                    'provider': provider,
                    'amount': outflow,
                    'net': abs(net)
                })
        
        # Sort by net flow
        top_inflows.sort(key=lambda x: x['net'], reverse=True)
        top_outflows.sort(key=lambda x: x['net'], reverse=True)
        
        return {
            'net_flow': flows.get('total_net_flow_eth', 0),
            'total_inflow': flows.get('total_inflow_eth', 0),
            'total_outflow': flows.get('total_outflow_eth', 0),
            'top_inflows': top_inflows[:3],
            'top_outflows': top_outflows[:3],
            'sentiment': 'bullish' if flows.get('total_net_flow_eth', 0) > 0 else 'bearish',
            'block_range': {'from': start_block, 'to': end_block}
        }
    
    def get_daily_flows(self, days_back: int = 1) -> Dict:
        """Get ETF flows for the past N days."""
        return self.get_flows_for_period(TimePeriod.DAILY, days_back)
    
    def get_weekly_flows(self, weeks_back: int = 1) -> Dict:
        """Get ETF flows for the past N weeks."""
        return self.get_flows_for_period(TimePeriod.WEEKLY, weeks_back)
    
    # Provider competition analysis
    
    def get_market_share(self, block_number: Optional[int] = None) -> Dict[str, float]:
        """
        Calculate market share for each ETF provider.
        
        Returns:
            Dict mapping provider to percentage of total ETF market
        """
        holdings = self.get_all_etf_holdings(block_number)
        total = sum(holdings.values())
        
        if total == 0:
            return {}
        
        return {
            provider: (amount / total) * 100
            for provider, amount in holdings.items()
        }
    
    def get_top_providers(
        self, 
        n: int = 10, 
        block_number: Optional[int] = None
    ) -> List[Tuple[str, float, float]]:
        """
        Get top N ETF providers by holdings.
        
        Returns:
            List of (provider, eth_amount, market_share_pct) tuples
        """
        holdings = self.get_all_etf_holdings(block_number)
        market_share = self.get_market_share(block_number)
        
        sorted_providers = sorted(
            holdings.items(), 
            key=lambda x: x[1], 
            reverse=True
        )[:n]
        
        return [
            (provider, amount, market_share.get(provider, 0))
            for provider, amount in sorted_providers
        ]
    
    # Flow statistics
    
    def get_flow_statistics(
        self,
        from_block: int,
        to_block: int,
        period: TimePeriod = TimePeriod.DAILY
    ) -> Dict:
        """
        Calculate flow statistics over a range.
        
        Returns:
            Dict with mean, median, std dev, and other stats
        """
        # Get base flow data
        flow_data = self.get_flows_between_blocks(from_block, to_block)
        
        # This would use the Rust aggregation module
        # For now, return the raw flow data
        return flow_data
    
    # Whale movement tracking
    
    def get_large_transfers(
        self,
        from_block: int,
        to_block: int,
        min_eth_amount: float = 1000.0
    ) -> List[Dict]:
        """
        Find large ETF transfers in a block range.
        
        Args:
            from_block: Starting block
            to_block: Ending block
            min_eth_amount: Minimum ETH amount to consider
            
        Returns:
            List of large transfer events
        """
        # This would query for specific large transfers
        # Implementation would be in Rust
        flow_data = self.get_flows_between_blocks(from_block, to_block)
        
        # Extract large transfers from flow data
        large_transfers = []
        for provider, data in flow_data.get('providers', {}).items():
            if data.get('inflow_eth', 0) >= min_eth_amount:
                large_transfers.append({
                    'provider': provider,
                    'type': 'inflow',
                    'amount': data['inflow_eth'],
                    'block_range': (from_block, to_block)
                })
            if data.get('outflow_eth', 0) >= min_eth_amount:
                large_transfers.append({
                    'provider': provider,
                    'type': 'outflow',
                    'amount': data['outflow_eth'],
                    'block_range': (from_block, to_block)
                })
        
        return large_transfers
    
    # Comparison methods
    
    def compare_providers(
        self,
        providers: List[str],
        from_block: int,
        to_block: int
    ) -> Dict:
        """
        Compare flow metrics between specific providers.
        
        Args:
            providers: List of provider names to compare
            from_block: Starting block
            to_block: Ending block
            
        Returns:
            Comparison data for specified providers
        """
        flow_data = self.get_flows_between_blocks(from_block, to_block)
        provider_data = flow_data.get('providers', {})
        
        comparison = {}
        for provider in providers:
            if provider in provider_data:
                comparison[provider] = provider_data[provider]
        
        return comparison
    
    # Historical analysis
    
    def get_cumulative_flows(
        self,
        provider: Optional[str] = None,
        from_block: int = None,
        to_block: int = None,
        sample_interval: int = 7200  # Daily samples
    ) -> List[Dict]:
        """
        Get cumulative flow data over time.
        
        Args:
            provider: Specific provider or None for all
            from_block: Starting block
            to_block: Ending block
            sample_interval: Blocks between samples
            
        Returns:
            List of cumulative flow snapshots
        """
        if to_block is None:
            to_block = self.query.get_latest_block()
        if from_block is None:
            from_block = to_block - (30 * 7200)  # 30 days default
        
        cumulative_data = []
        current_block = from_block
        cumulative_inflow = 0.0
        cumulative_outflow = 0.0
        
        while current_block < to_block:
            next_block = min(current_block + sample_interval, to_block)
            
            flows = self.get_flows_between_blocks(current_block, next_block)
            
            if provider:
                provider_flows = flows.get('providers', {}).get(provider, {})
                cumulative_inflow += provider_flows.get('inflow_eth', 0)
                cumulative_outflow += provider_flows.get('outflow_eth', 0)
            else:
                cumulative_inflow += flows.get('total_inflow', 0)
                cumulative_outflow += flows.get('total_outflow', 0)
            
            cumulative_data.append({
                'block': next_block,
                'cumulative_inflow': cumulative_inflow,
                'cumulative_outflow': cumulative_outflow,
                'cumulative_net': cumulative_inflow - cumulative_outflow
            })
            
            current_block = next_block
        
        return cumulative_data
    
    def __repr__(self) -> str:
        """String representation."""
        return f"ETFQueries(providers={len(self.get_all_etf_holdings())})"