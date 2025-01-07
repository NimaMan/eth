"""
Portfolio Metrics Calculator

Objective:
---------
1. Calculate comprehensive portfolio performance metrics
2. Track historical portfolio states
3. Generate risk analytics
4. Provide real-time portfolio insights

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
from typing import Dict, List
from datetime import datetime
import numpy as np
import pandas as pd

from eth_portfolio_manager.utils.logger import get_logger
from eth_portfolio_manager.core.data_models import TokenPositionData, TokenPositionState
from eth_portfolio_manager.state.portfolio_state_server import PortfolioStateServer


@dataclass
class PortfolioMetrics:
    """Core portfolio metrics"""
    # Value metrics
    total_value: float = 0.0
    active_value: float = 0.0
    inactive_value: float = 0.0
    
    # Performance metrics
    total_profit_loss: float = 0.0
    realized_profit: float = 0.0
    unrealized_profit: float = 0.0
    
    # Position metrics
    total_position_count: int = 0
    active_position_count: int = 0
    inactive_position_count: int = 0
    
    # Risk metrics
    largest_position_value: float = 0.0
    largest_position_pct: float = 0.0
    
    # Timestamp
    last_updated: datetime = None


class PortfolioMetricsCalculator:
    def __init__(self, state_server: PortfolioStateServer, logger=None):
        self.state_server = state_server
        self.logger = logger or get_logger(name="portfolio_manager")
        self.metrics = PortfolioMetrics()
        self.position_history: Dict[str, List[TokenPositionData]] = {}
        
    async def update_metrics(self, positions: Dict[str, TokenPositionData]) -> PortfolioMetrics:
        """Calculate all portfolio metrics from current positions"""
        self.metrics = await self._calculate_core_metrics(positions)
        return self.metrics
        
    async def _calculate_core_metrics(self, positions: Dict[str, TokenPositionData]) -> PortfolioMetrics:
        """Calculate core portfolio metrics"""
        metrics = PortfolioMetrics(last_updated=datetime.now())
        
        for position in positions.values():
            # Update value metrics
            if position.has_active_position:
                metrics.active_value += position.current_value
                metrics.active_position_count += 1
            else:
                metrics.inactive_value += position.current_value
                metrics.inactive_position_count += 1
            
            # Update profit metrics
            metrics.realized_profit += position.realized_profit
            metrics.unrealized_profit += position.unrealized_profit
            
            # Track largest position
            if position.current_value > metrics.largest_position_value:
                metrics.largest_position_value = position.current_value
        
        # Calculate totals
        metrics.total_value = metrics.active_value + metrics.inactive_value
        metrics.total_profit_loss = metrics.realized_profit + metrics.unrealized_profit
        metrics.total_position_count = metrics.active_position_count + metrics.inactive_position_count
        
        # Calculate percentages
        if metrics.total_value > 0:
            metrics.largest_position_pct = metrics.largest_position_value / metrics.total_value
            
        return metrics 