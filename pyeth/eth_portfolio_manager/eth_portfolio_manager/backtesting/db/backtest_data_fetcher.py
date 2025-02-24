"""
Backtest Data Fetcher Module

Objective:
----------
This module provides hierarchical access to backtest results by leveraging the TokenPosition aggregate pattern. It retrieves and reconstructs token position data (both static metadata and dynamic snapshots) from JSON-serialized database records, enabling comprehensive analysis of strategy performance across multiple dimensions.

Key Concepts:
------------
1. Strategy Run Metadata:
   - Represents the high-level execution context of a trading strategy.
   - Contains:
       • Strategy name and parameters
       • Execution timeframe (start/end blocks)
       • Creation timestamp
       • Performance summary metrics

2. Token Position Data:
   - Stored as JSON in the database (via BacktestResultsWriter)
   - Comprises two main components:
       a) Static Data:
          • Token identifiers (address, symbol)
          • Creation/trading enablement blocks
          • Entry/exit price ratios and timestamps
          • Purchase values and transaction fees
       b) Dynamic History:
          • Time-series of position snapshots
          • Price evolution and ROI calculations
          • Scam detection metrics
          • Position state transitions

Data Access Patterns:
-------------------
1. Strategy Overview Access:
   - Retrieves all strategy runs with their metadata
   - Entry point for drilling down into specific strategies

2. Strategy Details Access:
   - Fetches complete information about a specific strategy run
   - Includes execution parameters and aggregate performance metrics

3. Token Position Access:
   - Three levels of granularity:
       a) Token List: Summary of all tokens in a strategy
       b) Latest Positions: Current state of each token position
       c) Position History: Complete evolution of individual token positions

4. Performance Metrics Access:
   - Calculates strategy-wide metrics from token position data
   - Aggregates ROI, profit/loss, and risk metrics across positions

Integration Points:
-----------------
1. Database Layer:
   - Reads JSON-serialized TokenPosition data
   - Reconstructs TokenPosition instances via from_dict factory method

2. API Layer:
   - Provides formatted data to API routes
   - Maintains consistent structure for frontend consumption

3. Analysis Layer:
   - Supports both real-time monitoring and historical analysis
   - Enables strategy comparison and optimization

Usage Guidelines:
---------------
1. Strategy Run Queries:
   - Use fetch_strategy_runs() for overview of all strategies
   - Use fetch_strategy_run_details() for specific strategy metadata

2. Token Position Queries:
   - Use fetch_token_list_for_strategy() for token summaries
   - Use fetch_latest_strategy_positions() for current position states
   - Use fetch_token_position_history() for complete position evolution

3. Performance Analysis:
   - Use fetch_strategy_performance_metrics() for aggregate statistics
   - Metrics are computed from the latest position snapshots
"""

from typing import Dict, Optional, List, Any
import psycopg2
from psycopg2.extras import RealDictCursor
from eth_portfolio_manager.core.portfolio_metrics_calculator import *
from eth_portfolio_manager.utils.logger import get_monitoring_logger


# Retrieve all strategy runs with minute-level timestamp
STRATEGY_LIST = """
    SELECT 
        id,
        name,
        parameters,
        start_block,
        end_block,
        DATE_TRUNC('minute', created_at) as created_at
    FROM strategy_runs
    ORDER BY created_at DESC;
"""


STRATEGY_POSITIONS_LIST = """
    SELECT 
        token_address,
        token_position->'static' as static_data,
        token_position->'dynamic_history'->-1 as latest_snapshot
    FROM token_positions
    WHERE strategy_run_id = %s
    ORDER BY (token_position->'static'->>'creation_block')::integer DESC;
"""


# Get complete raw position history for a specific token in a strategy.
TOKEN_POSITION_HISTORY = """
    SELECT 
        token_address,
        token_position->'static' as static_data,
        token_position->'dynamic_history' as dynamic_history
    FROM token_positions
    WHERE strategy_run_id = %s 
      AND token_address = %s;
"""


CURRENCY_TOKENS_LIST = """
    SELECT DISTINCT currency
    FROM token_positions
    WHERE currency IS NOT NULL
"""


class BacktestDataFetcher:
    """Data access layer for backtest portfolio data in PostgreSQL"""

    def __init__(self, ):
        """Initialize with database connection and a formatter for data normalization."""
        self.db_conn = self._create_default_connection()
        self.cursor = self.db_conn.cursor(cursor_factory=RealDictCursor)
        self.logger = get_monitoring_logger()
        self.metrics_calculator = PortfolioMetricsCalculator()

    def _create_default_connection(self) -> psycopg2.extensions.connection:
        """Create the default database connection."""
        return psycopg2.connect(
            dbname="backtest",
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )

    def fetch_strategy_runs(self) -> List[dict]:
        """Retrieve and format all strategy runs."""
        try:
            self.cursor.execute(STRATEGY_LIST)
            strategy_runs = self.cursor.fetchall()
            self.logger.info(f"Found {len(strategy_runs)} strategy runs")
            # Format the output to match expected structure
            formatted_runs = {}
            for run in strategy_runs:
                formatted_runs[run['id']] = {
                    'id': run['id'],
                    'name': run['name'],
                    'parameters': run['parameters'],
                    'start_block': run['start_block'],
                    'end_block': run['end_block'],
                    'created_at': run['created_at']
                }
            self.strategy_runs = formatted_runs
            return formatted_runs
        except Exception as e:
            self.logger.error(f"Error fetching strategy runs: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return {}

    def fetch_strategy_run_details(self, strategy_run_id: int) -> Dict[str, Any]:
        """Retrieve and format detailed information for a specific strategy run."""
        if not hasattr(self, 'strategy_runs') or self.strategy_runs is None:
            self.fetch_strategy_runs()
        return self.strategy_runs[strategy_run_id]
            
    def fetch_strategy_token_positions(self, strategy_run_id: int) -> List[Dict[str, Any]]:
        """Retrieve and format current positions for a specific strategy run."""
        if hasattr(self, 'strategy_token_positions') and self.strategy_token_positions is not None:
            if strategy_run_id in self.strategy_token_positions:
                return self.strategy_token_positions[strategy_run_id]
        try:
            self.strategy_token_positions = {}
            self.cursor.execute(STRATEGY_POSITIONS_LIST, (strategy_run_id,))
            positions = self.cursor.fetchall()
            self.logger.info(f"Found {len(positions)} positions for strategy run {strategy_run_id}")
            formatted_positions = {}
            for pos in positions:
                formatted_positions[pos['token_address']] = {
                    'static_data': pos['static_data'],
                    'latest_snapshot': pos['latest_snapshot']
                }
            self.strategy_token_positions[strategy_run_id] = formatted_positions
            return formatted_positions
        except Exception as e:
            self.logger.error(f"Error fetching positions from database: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return {}

    def fetch_token_list_for_strategy(self, strategy_run_id: int) -> List[Dict[str, Any]]:
        """Retrieve and format the list of unique tokens with metrics for a specific strategy run."""
        if not hasattr(self, 'strategy_token_positions') or self.strategy_token_positions is None:
            self.fetch_strategy_token_positions(strategy_run_id)    
        return list(self.strategy_token_positions[strategy_run_id].keys())

    def fetch_token_position_history(self, strategy_run_id: int, token_address: str) -> List[Dict[str, Any]]:
        """Retrieve and format the complete position history for a specific token in a strategy."""            
        try:
            self.cursor.execute(TOKEN_POSITION_HISTORY, (strategy_run_id, token_address))
            token_position_history = self.cursor.fetchall()[0]
            formatted_history = {
                'static_data': token_position_history['static_data'],
                'dynamic_history': token_position_history['dynamic_history']
            }
            return formatted_history
        
        except Exception as e:
            self.logger.error(f"Error fetching token position history: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return []

    def fetch_strategy_performance_metrics(self, strategy_run_id: int) -> PortfolioMetrics:
        """
        Retrieves strategy positions, groups them by token address,
        and then calculates portfolio metrics using the PortfolioMetricsCalculator.
        """
        try:
            latest_positions = self.fetch_strategy_token_positions(strategy_run_id)
            # Calculate metrics using the latest positions for each token.
            metrics = self.metrics_calculator.calculate_portfolio_metrics(latest_positions)
            return metrics

        except Exception as e:
            self.logger.error(f"Error calculating portfolio metrics: {e}")
            raise e
            
        
    def __del__(self):
        """Ensure the database connection is closed on cleanup."""
        if hasattr(self, 'db_conn') and self.db_conn:
            self.db_conn.close() 