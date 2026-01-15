"""
Portfolio Metrics Calculator

Objective:
---------
Calculate comprehensive portfolio performance metrics from both live TokenPosition objects 
and JSON-structured database records, providing unified analytics across real-time and historical data.

Key Features:
------------
1. Dual Input Support:
   - Live TokenPosition objects
   - JSON database records (static + dynamic history)
2. Consistent Metric Calculation
3. Historical Analysis
4. Performance Tracking

Metric Categories:
----------------
1. Value Metrics:
   - Total Portfolio Value
   - Value per Position
   - Value Distribution
   - Historical Value Changes
   - Value at Risk (VaR)

2. Performance Metrics:
   - Total Returns (%)
   - Returns per Position
   - Realized vs Unrealized P&L
   - Time-weighted Returns
   - Risk-adjusted Returns
   - Sharpe/Sortino Ratios

3. Risk Metrics:
   - Portfolio Volatility
   - Position Concentration
   - Drawdown Analysis
   - Risk Exposure per Token
   - Correlation Analysis
   - Maximum Drawdown

4. Trading Metrics:
   - Position Count
   - Win/Loss Ratio
   - Average Hold Time
   - Success Rate by Token Type
   - Entry/Exit Timing Analysis
   - Trading Volume

5. Token-specific Metrics:
   - Token Age Analysis
   - Liquidity Metrics
   - Price Impact Analysis
   - Volume Profile
   - Holder Distribution
   - Smart Money Flow

6. Historical Analysis:
   - Portfolio State History
   - Performance Timeline
   - Risk Evolution
   - Strategy Performance
   - Market Condition Impact

Implementation Notes:
------------------
1. Real-time Calculations:
   - Efficient metric updates
   - Incremental calculations
   - Cache management
   - Priority-based updates

2. Data Management:
   - Historical data storage
   - Time series handling
   - Data compression
   - State recovery

3. Analysis Features:
   - Custom time windows
   - Comparative analysis
   - Scenario testing
   - Risk forecasting

4. Integration Points:
   - Position Manager Updates
   - Strategy Feedback
   - Risk Management
   - Reporting System
"""

from dataclasses import dataclass
from typing import Dict, List, Union, Any, Optional
from datetime import datetime
import logging
import numpy as np
import pandas as pd

from eth_portfolio_manager.core.token_position import TokenPosition

logger = logging.getLogger(__name__)

@dataclass
class CurrencyMetrics:
    """Metrics for a specific currency"""
    currency: str
    total_value: float = 0.0
    realized_profit: float = 0.0
    unrealized_profit: float = 0.0
    total_profit_loss: float = 0.0
    position_count: int = 0
    active_position_count: int = 0
    inactive_sold_position_count: int = 0
    inactive_init_position_count: int = 0
    scammed_position_count: int = 0


@dataclass
class PortfolioMetrics:
    """Core portfolio metrics"""
    # Metrics by currency
    currency_metrics: Dict[str, CurrencyMetrics] = None
    
    # Position metrics (currency-independent)
    total_position_count: int = 0
    init_state_count: int = 0
    buy_state_count: int = 0
    sell_state_count: int = 0
    scammed_state_count: int = 0
    
    # Trading metrics
    win_count: int = 0
    loss_count: int = 0
    success_rate: float = 0.0
    
    # Timestamp
    last_updated: datetime = None
    
    def __post_init__(self):
        if self.currency_metrics is None:
            self.currency_metrics = {}


class PortfolioMetricsCalculator:
    def __init__(self):
        self.metrics = PortfolioMetrics()
            
    def calculate_portfolio_metrics(self, positions_data: Dict[str, List[Dict[str, Any]]]) -> PortfolioMetrics:
        """
        Calculate portfolio metrics from positions data grouped by state
        
        Args:
            positions_data: Dictionary with keys 'init_positions', 'buy_positions', 
                           'sell_positions', 'scam_positions'
        
        Returns:
            PortfolioMetrics object with calculated metrics
        """
        metrics = PortfolioMetrics(last_updated=datetime.now())
        
        # Initialize with position counts
        metrics.init_state_count = len(positions_data.get('init_positions', []))
        metrics.buy_state_count = len(positions_data.get('buy_positions', []))
        metrics.sell_state_count = len(positions_data.get('sell_positions', []))
        metrics.scammed_state_count = len(positions_data.get('scam_positions', []))
        metrics.total_position_count = (metrics.init_state_count + metrics.buy_state_count + 
                                       metrics.sell_state_count + metrics.scammed_state_count)
        
        # Process all positions together for currency-specific metrics
        all_positions = []
        for state_positions in positions_data.values():
            all_positions.extend(state_positions)
        
        # Track win/loss for success rate
        win_count = 0
        loss_count = 0
        
        for position in all_positions:
            # Get currency (default to "Unknown" if not available)
            currency = position.get('currency', 'Unknown')
            if currency is None or currency == '':
                currency = 'Unknown'
            
            # Initialize currency metrics if needed
            if currency not in metrics.currency_metrics:
                metrics.currency_metrics[currency] = CurrencyMetrics(currency=currency)
            
            curr_metrics = metrics.currency_metrics[currency]
            curr_metrics.position_count += 1
            
            # Update currency-specific state counts
            position_state = position.get('position_state', 'Unknown')
            if position_state == 'Init':
                curr_metrics.inactive_init_position_count += 1
            elif position_state in ['Buy Submitted', 'Buy Confirmed']:
                curr_metrics.active_position_count += 1
            elif position_state in ['Sell Submitted', 'Sell Confirmed']:
                curr_metrics.inactive_sold_position_count += 1
                # Track win/loss for completed trades
                realized_profit = float(position.get('realized_profit', 0.0))
                if realized_profit > 0:
                    win_count += 1
                else:
                    loss_count += 1
            elif position_state == 'Scammed':
                curr_metrics.scammed_position_count += 1
                # Scammed positions count as losses
                loss_count += 1
            
            # Update value metrics
            current_value = float(position.get('current_value', 0.0) or 0.0)
            realized_profit = float(position.get('realized_profit', 0.0) or 0.0)
            unrealized_profit = float(position.get('unrealized_profit', 0.0) or 0.0)
            
            curr_metrics.total_value += current_value
            curr_metrics.realized_profit += realized_profit
            curr_metrics.unrealized_profit += unrealized_profit
        
        # Calculate totals and performance metrics
        for curr_metrics in metrics.currency_metrics.values():
            curr_metrics.total_profit_loss = curr_metrics.realized_profit + curr_metrics.unrealized_profit
        
        # Calculate success rate
        total_completed = win_count + loss_count
        metrics.win_count = win_count
        metrics.loss_count = loss_count
        metrics.success_rate = (win_count / total_completed * 100) if total_completed > 0 else 0.0
        
        return metrics
    
    def calculate_metrics_for_api(self, positions_data: Dict[str, List[Dict[str, Any]]]) -> Dict[str, Any]:
        """
        Calculate metrics and format them for API response
        
        Args:
            positions_data: Dictionary with positions grouped by state
            
        Returns:
            Dictionary with metrics formatted for API response
        """
        metrics = self.calculate_portfolio_metrics(positions_data)
        
        # Convert to dictionary format for API
        return {
            "metrics": {
                "currency_metrics": {
                    currency: {
                        "currency": cm.currency,
                        "total_value": cm.total_value,
                        "realized_profit": cm.realized_profit,
                        "unrealized_profit": cm.unrealized_profit,
                        "total_profit_loss": cm.total_profit_loss,
                        "position_count": cm.position_count,
                        "active_position_count": cm.active_position_count
                    } for currency, cm in metrics.currency_metrics.items()
                },
                "total_position_count": metrics.total_position_count,
                "init_state_count": metrics.init_state_count,
                "buy_state_count": metrics.buy_state_count,
                "sell_state_count": metrics.sell_state_count,
                "scammed_state_count": metrics.scammed_state_count,
                "win_count": metrics.win_count,
                "loss_count": metrics.loss_count,
                "success_rate": metrics.success_rate,
                "last_updated": metrics.last_updated.isoformat() if metrics.last_updated else None
            }
        }