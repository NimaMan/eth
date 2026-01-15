"""
CEX (Centralized Exchange) query interface for accessing exchange balances and flow data.

This module provides Python interface to CEX-related chain queries,
getting all calculations done in Rust and formatting results for Python use.
"""

from typing import Dict, List, Optional, Tuple
from datetime import datetime
from .base_queries import BaseEntityQuery, TimePeriod


class CEXQueries(BaseEntityQuery):
    """
    CEX-specific queries for exchange balances, flows, and analysis.
    
    All calculations are performed in Rust for performance.
    Python only formats and presents the data.
    """
    
    def __init__(self, reth_instance=None):
        """Initialize CEX queries with shared Reth instance."""
        super().__init__(reth_instance)
    
    # Basic CEX queries
    
    def is_cex_address(self, address: str) -> bool:
        """Check if an address belongs to a centralized exchange."""
        return self.query.is_cex(address)
    
    def get_cex_info(self, address: str) -> Optional[Dict]:
        """
        Get CEX information for an address.
        
        Returns:
            Dict with exchange name and address type, or None if not CEX
        """
        return self.query.get_cex_info(address)
    
    # Balance queries
    
    def get_all_cex_balances(self, block_number: Optional[int] = None) -> Dict[str, float]:
        """
        Get current ETH balances for all centralized exchanges.
        
        Args:
            block_number: Block to query at (default: latest)
            
        Returns:
            Dict mapping exchange name to ETH amount
        """
        raw_data = self.query.get_cex_balances(block_number)
        # Convert list of exchange dicts to simple name->balance mapping
        if isinstance(raw_data, dict) and 'exchanges' in raw_data:
            exchanges_list = raw_data.get('exchanges', [])
            return {ex['name']: ex['total_balance'] for ex in exchanges_list}
        return {}
    
    def get_exchange_balance(
        self, 
        exchange: str, 
        block_number: Optional[int] = None
    ) -> float:
        """
        Get ETH balance for a specific exchange.
        
        Args:
            exchange: Exchange name (e.g., 'Binance', 'Coinbase')
            block_number: Block to query at
            
        Returns:
            Total ETH held by exchange
        """
        all_balances = self.get_all_cex_balances(block_number)
        return all_balances.get(exchange, 0.0)
    
    # Flow analysis
    
    def get_flows_between_blocks(
        self, 
        from_block: int, 
        to_block: int
    ) -> Dict:
        """
        Calculate CEX flows between two blocks.
        
        Args:
            from_block: Starting block number
            to_block: Ending block number
            
        Returns:
            Dict with flow data for each exchange and totals
        """
        return self.query.calculate_cex_flows_between_blocks(from_block, to_block)
    
    def get_exchange_flows(self, hours_back: int = 24) -> Dict:
        """
        Get CEX flows summary for dashboard.
        
        Returns formatted data with deposits, withdrawals, and sentiment.
        """
        start_block, end_block = self.convert_time_to_blocks(hours_back)
        flows = self.get_flows_between_blocks(start_block, end_block)
        
        # Extract exchange flows
        exchange_flows = flows.get('exchange_flows', {})
        
        # Calculate top deposits and withdrawals
        top_deposits = []
        top_withdrawals = []
        
        for exchange, data in exchange_flows.items():
            inflow = data.get('inflow_eth', 0)
            outflow = data.get('outflow_eth', 0)
            net = data.get('net_flow_eth', 0)
            
            if inflow > 0:
                top_deposits.append({
                    'exchange': exchange,
                    'amount': inflow,
                    'net': net
                })
            if outflow > 0:
                top_withdrawals.append({
                    'exchange': exchange,
                    'amount': outflow,
                    'net': abs(net)
                })
        
        # Sort by amount
        top_deposits.sort(key=lambda x: x['amount'], reverse=True)
        top_withdrawals.sort(key=lambda x: x['amount'], reverse=True)
        
        # Calculate sentiment
        net_flow = flows.get('total_net_flow_eth', 0)
        if net_flow > 1000:  # More than 1000 ETH net deposit
            sentiment = 'bearish'  # People moving to exchanges to sell
        elif net_flow < -1000:  # More than 1000 ETH net withdrawal
            sentiment = 'bullish'  # People moving off exchanges (hodling)
        else:
            sentiment = 'neutral'
        
        return {
            'net_flow': net_flow,
            'total_deposits': flows.get('total_inflow_eth', 0),
            'total_withdrawals': flows.get('total_outflow_eth', 0),
            'top_deposits': top_deposits[:5],
            'top_withdrawals': top_withdrawals[:5],
            'sentiment': sentiment,
            'sentiment_reason': self._get_sentiment_reason(net_flow),
            'block_range': {'from': start_block, 'to': end_block}
        }
    
    def _get_sentiment_reason(self, net_flow: float) -> str:
        """Get human-readable sentiment explanation."""
        if net_flow > 1000:
            return f"{self.format_eth(abs(net_flow))} moved to exchanges (potential selling pressure)"
        elif net_flow < -1000:
            return f"{self.format_eth(abs(net_flow))} withdrawn from exchanges (accumulation)"
        else:
            return "Balanced flow between deposits and withdrawals"
    
    def get_flows_for_period(
        self,
        period: TimePeriod,
        periods_back: int = 1,
        end_block: Optional[int] = None
    ) -> Dict:
        """
        Get CEX flows for a time period.
        
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
    
    def get_daily_flows(self, days_back: int = 1) -> Dict:
        """Get CEX flows for the past N days."""
        return self.get_flows_for_period(TimePeriod.DAILY, days_back)
    
    def get_weekly_flows(self, weeks_back: int = 1) -> Dict:
        """Get CEX flows for the past N weeks."""
        return self.get_flows_for_period(TimePeriod.WEEKLY, weeks_back)
    
    # Exchange competition analysis
    
    def get_market_share(self, block_number: Optional[int] = None) -> Dict[str, float]:
        """
        Calculate market share for each exchange.
        
        Returns:
            Dict mapping exchange to percentage of total CEX holdings
        """
        balances = self.get_all_cex_balances(block_number)
        total = sum(balances.values())
        
        if total == 0:
            return {}
        
        return {
            exchange: (amount / total) * 100
            for exchange, amount in balances.items()
        }
    
    def get_top_exchanges(
        self, 
        n: int = 10, 
        block_number: Optional[int] = None
    ) -> List[Tuple[str, float, float]]:
        """
        Get top N exchanges by ETH holdings.
        
        Returns:
            List of (exchange, eth_amount, market_share_pct) tuples
        """
        balances = self.get_all_cex_balances(block_number)
        market_share = self.get_market_share(block_number)
        
        sorted_exchanges = sorted(
            balances.items(), 
            key=lambda x: x[1], 
            reverse=True
        )[:n]
        
        return [
            (exchange, amount, market_share.get(exchange, 0))
            for exchange, amount in sorted_exchanges
        ]
    
    # Deposit/Withdrawal patterns
    
    def analyze_deposit_patterns(
        self,
        exchange: str,
        from_block: int,
        to_block: int,
        min_eth: float = 100.0
    ) -> Dict:
        """
        Analyze deposit patterns for an exchange.
        
        Args:
            exchange: Exchange name
            from_block: Starting block
            to_block: Ending block
            min_eth: Minimum ETH amount to consider
            
        Returns:
            Analysis of deposit patterns
        """
        flows = self.get_flows_between_blocks(from_block, to_block)
        exchange_flows = flows.get('exchanges', {}).get(exchange, {})
        
        return {
            'total_deposits': exchange_flows.get('inflow_eth', 0),
            'deposit_count': exchange_flows.get('inflow_count', 0),
            'avg_deposit_size': (
                exchange_flows.get('inflow_eth', 0) / exchange_flows.get('inflow_count', 1)
                if exchange_flows.get('inflow_count', 0) > 0 else 0
            ),
            'large_deposits': self._count_large_transfers(
                exchange_flows.get('inflows', []), min_eth
            )
        }
    
    def analyze_withdrawal_patterns(
        self,
        exchange: str,
        from_block: int,
        to_block: int,
        min_eth: float = 100.0
    ) -> Dict:
        """
        Analyze withdrawal patterns for an exchange.
        
        Args:
            exchange: Exchange name
            from_block: Starting block
            to_block: Ending block
            min_eth: Minimum ETH amount to consider
            
        Returns:
            Analysis of withdrawal patterns
        """
        flows = self.get_flows_between_blocks(from_block, to_block)
        exchange_flows = flows.get('exchanges', {}).get(exchange, {})
        
        return {
            'total_withdrawals': exchange_flows.get('outflow_eth', 0),
            'withdrawal_count': exchange_flows.get('outflow_count', 0),
            'avg_withdrawal_size': (
                exchange_flows.get('outflow_eth', 0) / exchange_flows.get('outflow_count', 1)
                if exchange_flows.get('outflow_count', 0) > 0 else 0
            ),
            'large_withdrawals': self._count_large_transfers(
                exchange_flows.get('outflows', []), min_eth
            )
        }
    
    # Inter-exchange flows
    
    def get_inter_exchange_flows(
        self,
        from_block: int,
        to_block: int
    ) -> Dict[str, Dict[str, float]]:
        """
        Analyze flows between different exchanges.
        
        Args:
            from_block: Starting block
            to_block: Ending block
            
        Returns:
            Matrix of flows between exchanges
        """
        # This would need implementation in Rust to track
        # transfers where both from and to are exchange addresses
        flows = self.get_flows_between_blocks(from_block, to_block)
        
        # Placeholder for inter-exchange flow matrix
        inter_flows = {}
        
        return inter_flows
    
    # Historical analysis
    
    def get_balance_history(
        self,
        exchange: Optional[str] = None,
        from_block: int = None,
        to_block: int = None,
        sample_interval: int = 7200  # Daily samples
    ) -> List[Dict]:
        """
        Get historical balance snapshots.
        
        Args:
            exchange: Specific exchange or None for all
            from_block: Starting block
            to_block: Ending block  
            sample_interval: Blocks between samples
            
        Returns:
            List of balance snapshots over time
        """
        if to_block is None:
            to_block = self.query.get_latest_block()
        if from_block is None:
            from_block = to_block - (30 * 7200)  # 30 days default
        
        balance_history = []
        current_block = from_block
        
        while current_block <= to_block:
            if exchange:
                balance = self.get_exchange_balance(exchange, current_block)
                balance_history.append({
                    'block': current_block,
                    'exchange': exchange,
                    'balance': balance
                })
            else:
                balances = self.get_all_cex_balances(current_block)
                balance_history.append({
                    'block': current_block,
                    'total_balance': sum(balances.values()),
                    'exchanges': balances
                })
            
            current_block += sample_interval
        
        return balance_history
    
    # Risk metrics
    
    def calculate_concentration_risk(
        self,
        block_number: Optional[int] = None
    ) -> Dict:
        """
        Calculate concentration risk metrics.
        
        Returns:
            Dict with HHI index and other concentration metrics
        """
        market_shares = self.get_market_share(block_number)
        
        # Herfindahl-Hirschman Index (HHI)
        hhi = sum(share ** 2 for share in market_shares.values())
        
        # Top 3 concentration
        top_3_share = sum(sorted(market_shares.values(), reverse=True)[:3])
        
        # Gini coefficient (simplified)
        shares = sorted(market_shares.values())
        n = len(shares)
        if n == 0:
            gini = 0
        else:
            cumsum = 0
            for i, share in enumerate(shares):
                cumsum += (n - i) * share
            gini = (n + 1 - 2 * cumsum / sum(shares)) / n if sum(shares) > 0 else 0
        
        return {
            'hhi': hhi,
            'top_3_concentration': top_3_share,
            'gini_coefficient': gini,
            'exchange_count': len(market_shares),
            'risk_level': self._classify_concentration_risk(hhi)
        }
    
    def _classify_concentration_risk(self, hhi: float) -> str:
        """Classify concentration risk based on HHI."""
        if hhi < 1500:
            return 'low'
        elif hhi < 2500:
            return 'moderate'
        else:
            return 'high'
    
    def _count_large_transfers(self, transfers: List, min_amount: float) -> int:
        """Count transfers above minimum amount."""
        # This would need the actual transfer list from Rust
        return 0  # Placeholder
    
    # Anomaly detection
    
    def detect_unusual_flows(
        self,
        from_block: int,
        to_block: int,
        std_dev_threshold: float = 2.0
    ) -> List[Dict]:
        """
        Detect unusual flow patterns.
        
        Args:
            from_block: Starting block
            to_block: Ending block
            std_dev_threshold: Number of standard deviations for outlier detection
            
        Returns:
            List of unusual flow events
        """
        # Would need historical baseline calculation in Rust
        flows = self.get_flows_between_blocks(from_block, to_block)
        
        unusual_events = []
        # Placeholder for anomaly detection logic
        
        return unusual_events
    
    def __repr__(self) -> str:
        """String representation."""
        return f"CEXQueries(exchanges={len(self.get_all_cex_balances())})"