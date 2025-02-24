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
from typing import Dict, List, Union, Any
from datetime import datetime
import numpy as np
import pandas as pd

from eth_portfolio_manager.core.token_position import TokenPosition


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


@dataclass
class PortfolioMetrics:
    """Core portfolio metrics"""
    # Metrics by currency
    currency_metrics: Dict[str, CurrencyMetrics] = None
    
    # Position metrics (currency-independent)
    total_position_count: int = 0
    init_position_count: int = 0
    buy_position_count: int = 0
    sell_position_count: int = 0
    
    # Timestamp
    last_updated: datetime = None
    
    def __post_init__(self):
        if self.currency_metrics is None:
            self.currency_metrics = {}


class PortfolioMetricsCalculator:
    def __init__(self):
        self.metrics = PortfolioMetrics()
        self.position_history: Dict[str, List[Any]] = {}
            
    def calculate_portfolio_metrics(self, token_positions: Dict[str, Union[Dict, Any]]) -> PortfolioMetrics:
        """Calculate core portfolio metrics grouped by currency"""
        metrics = PortfolioMetrics(last_updated=datetime.now())
        
        for token_address, position in token_positions.items():
            if isinstance(position, dict):
                static_data = position['static_data']
                latest_snapshot = position['latest_snapshot']
                
                # Handle currency
                currency = static_data.get('currency', 'Unknown')
                if currency is None:
                    currency = 'Unknown'
                
                # Extract position state and values
                position_state = latest_snapshot.get('position_state', 'Unknown')
                current_value = float(latest_snapshot.get('current_value', 0.0))
                realized_profit = float(latest_snapshot.get('realized_profit', 0.0))
                unrealized_profit = float(latest_snapshot.get('unrealized_profit', 0.0))
                
                # Initialize currency metrics if needed
                if currency not in metrics.currency_metrics:
                    metrics.currency_metrics[currency] = CurrencyMetrics(currency=currency)
                
                curr_metrics = metrics.currency_metrics[currency]
                curr_metrics.position_count += 1
                
                # Update state-based counts
                if position_state == 'Init':
                    metrics.init_position_count += 1
                    curr_metrics.inactive_init_position_count += 1
                elif position_state in ['BuySubmitted', 'BuyConfirmed']:
                    metrics.buy_position_count += 1
                    curr_metrics.active_position_count += 1
                elif position_state in ['SellSubmitted', 'SellConfirmed']:
                    metrics.sell_position_count += 1
                    curr_metrics.inactive_sold_position_count += 1
                
                # Update value metrics
                curr_metrics.total_value += current_value
                curr_metrics.realized_profit += realized_profit
                curr_metrics.unrealized_profit += unrealized_profit
                
                metrics.total_position_count += 1
            
            else:
                # Handle TokenPosition object format if needed
                continue
        
        # Calculate totals for each currency
        for curr_metrics in metrics.currency_metrics.values():
            curr_metrics.total_profit_loss = curr_metrics.realized_profit + curr_metrics.unrealized_profit
        
        return metrics