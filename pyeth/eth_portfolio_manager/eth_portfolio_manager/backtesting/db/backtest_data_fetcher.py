"""
Objective: Analyze backtest results for algorithmic trading strategies by providing hierarchical access to:
1. Strategy Run Overviews
2. Strategy Run Details
3. Positions Analysis for each strategy run
4. Token Metrics for each strategy run
5. Token Position History for each strategy run

Data Flow Architecture:
1. Strategy Runs List:
   - Retrieve all executed strategies with high-level performance metrics, start and end block, and total tokens.
   - Serves as the entry point for analysis.

2. Strategy Run Details:
   - Drill down into specific strategy execution parameters (e.g. start/end time, total trades, success rate).
   - Self-describing the strategy run, its performance and parameters. 

3. Token List Metrics:
   - Aggregate statistics per token per strategy run. Summerizes the performance of the token in the strategy run. 
   - Important for analyzing the performance of the strategy across different tokens. 

4. Token Position History:
   - Retrieve the full timeline of position-related events for an individual token.
   - Enables detailed trade pattern analysis and strategy validation.
   - Includes All the changes happening to the token positions in the strategy run in each block. 

5. Positions Analysis:
   - Retrieve current state of all token positions within a strategy run.
   - Includes All the changes happening to the token positions in the strategy run. 

Data Flow Pattern:
Frontend Request → API Route → DataFetcher (DB Access) → Formatter (Data Standardization) → API Response

Benefits of the Formatter Class:
- Centralizes and standardizes the transformation of raw database data into a frontend-friendly format.
- Ensures consistency in null handling, type conversion (e.g. formatting dates and converting numeric values), and naming conventions.
- Separates data access logic from presentation logic to simplify maintenance and scalability.
"""

import asyncio
from typing import Dict, Optional, List, Any
import psycopg2
from psycopg2.extras import RealDictCursor
from datetime import datetime
from eth_portfolio_manager.backtesting.db.backtest_queries import *
from eth_portfolio_manager.core.portfolio_metrics_calculator import *
from eth_portfolio_manager.utils.logger import get_monitoring_logger


class BacktestDataFrontEndFormatter:
    """Standardizes data formatting for frontend consumption"""

    @staticmethod
    def format_strategy_run(run: Dict) -> Dict:
        return {
            'id': run['id'],
            'strategy_name': run.get('strategy_name') or 'Unnamed Strategy',
            'created_at': run['created_at'].isoformat() if run.get('created_at') else None,
            'start_block': run.get('start_block') or 0,
            'end_block': run.get('end_block') or 0,
            'total_tokens': run.get('total_tokens') or 0,
            'closed_trades': run.get('closed_trades') or 0,
            'scammed_positions': run.get('scammed_positions') or 0,
            'total_profit': float(run.get('total_profit') or 0)
        }

    @staticmethod
    def format_strategy_run_details(run: Dict) -> Dict:
        """Format detailed information for a strategy run."""
        return {
            'id': run['id'],
            'start_time': run['start_time'].isoformat() if run.get('start_time') else None,
            'end_time': run['end_time'].isoformat() if run.get('end_time') else None,
            'total_trades': run.get('total_trades') or 0,
            'success_rate': float(run.get('success_rate') or 0),
            'start_block': run.get('start_block') or 0,
            'end_block': run.get('end_block') or 0,
            'strategy_name': run.get('strategy_name') or 'Unnamed Strategy'
        }

    @staticmethod
    def format_position(position: Dict) -> Dict:
        # drop :"id", "strategy_run_id"
        position.pop('id', None)
        position.pop('strategy_run_id', None)
        return position

    @staticmethod
    def format_token(token: Dict) -> Dict:
        return {
            'token_address': token['token_address'],
            'symbol': token.get('symbol') or 'UNKNOWN',
            'position_count': token.get('position_count', 0),
            'first_seen_block': token.get('first_seen_block', 0),
            'last_seen_block': token.get('last_seen_block', 0),
            'total_realized_profit': float(token.get('total_realized_profit', 0)),
            'num_greys': int(token.get('num_greys', 0)),
            'num_greens': int(token.get('num_greens', 0)),
            'scam_probability': float(token.get('scam_probability', 0)),
            'scam_reason': token.get('scam_reason', 'NA')
        }

    @staticmethod
    def format_history_record(record: Dict) -> Dict:
        """Format a record from the token position history."""
        formatted_record = {}
        for key, value in record.items():
            if isinstance(value, datetime):
                formatted_record[key] = value.isoformat()
            else:
                # If the value is numeric, cast to float; otherwise, leave as is (or empty string if None)
                if value is None:
                    formatted_record[key] = ''  # or 0 depending on the expected type
                elif isinstance(value, (int, float)):
                    formatted_record[key] = float(value)
                else:
                    formatted_record[key] = value
        return formatted_record


class BacktestDataFetcher:
    """Data access layer for backtest portfolio data in PostgreSQL"""

    def __init__(self, formatter=BacktestDataFrontEndFormatter):
        """Initialize with database connection and a formatter for data normalization."""
        self.formatter = formatter
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
        """Retrieve and format all strategy runs with performance metrics."""
        try:
            self.cursor.execute(STRATEGY_LIST)
            strategy_runs = self.cursor.fetchall()
            self.logger.info(f"Found {strategy_runs} strategy runs")
            return strategy_runs
        except Exception as e:
            self.logger.error(f"Error fetching strategy runs: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return []

    def fetch_strategy_run_details(self, strategy_run_id: int) -> Dict[str, Any]:
        """Retrieve and format detailed information for a specific strategy run."""
        try:
            self.cursor.execute(STRATEGY_RUN_DETAILS, (strategy_run_id,))
            result = self.cursor.fetchone()
            return result
        except Exception as e:
            self.logger.error(f"Error fetching strategy run details: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return {}

    def fetch_token_list_for_strategy(self, strategy_run_id: int) -> List[Dict[str, Any]]:
        """Retrieve and format the list of unique tokens with metrics for a specific strategy run."""
        try:
            self.cursor.execute(TOKEN_LIST_FOR_STRATEGY, (strategy_run_id,))
            tokens = self.cursor.fetchall()
            return tokens
        except Exception as e:
            self.logger.error(f"Error fetching token list: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return []

    def fetch_strategy_positions(self, strategy_run_id: int) -> List[Dict[str, Any]]:
        """Retrieve and format current positions for a specific strategy run."""
        try:
            self.cursor.execute(STRATEGY_POSITIONS_LIST, (strategy_run_id,))
            positions = self.cursor.fetchall()
            self.logger.info(f"Found {len(positions)} positions for strategy run {strategy_run_id}")
            return positions
        except Exception as e:
            self.logger.error(f"Error fetching positions from database: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return []

    def fetch_latest_strategy_positions(self, strategy_run_id: int) -> List[Dict[str, Any]]:
        """Retrieve and format the latest positions for a specific strategy run."""
        try:
            self.cursor.execute(STRATEGY_POSITIONS_LATEST, (strategy_run_id,))
            positions = self.cursor.fetchall()
            return positions
        except Exception as e:
            self.logger.error(f"Error fetching latest positions: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return []

    def fetch_token_position_history(self, strategy_run_id: int, token_address: str) -> List[Dict[str, Any]]:
        """Retrieve and format the complete position history for a specific token in a strategy."""
        try:
            self.cursor.execute(TOKEN_POSITION_HISTORY, (strategy_run_id, token_address))
            raw_history = self.cursor.fetchall()
            return [self.formatter.format_history_record(record) for record in raw_history]
        except Exception as e:
            self.logger.error(f"Error fetching position history: {e}", exc_info=True)
            if self.db_conn:
                self.db_conn.rollback()
            return []

    async def fetch_strategy_performance_metrics(self, strategy_run_id: int) -> PortfolioMetrics:
        """
        Retrieves strategy positions, groups them by token address,
        and then calculates portfolio metrics using the PortfolioMetricsCalculator.
        """
        try:
            loop = asyncio.get_event_loop()
            # Execute the synchronous fetch_strategy_positions in a thread.
            positions = await loop.run_in_executor(None, self.fetch_strategy_positions, strategy_run_id)
            
            # Select the latest position for each token (assuming positions are sorted in descending order by block number)
            latest_positions = {}
            for pos in positions:
                token_address = pos.get('token_address', '')
                # Using the first encountered position for each token (as query orders by block_number descending)
                if token_address not in latest_positions:
                    latest_positions[token_address] = pos
            
            # Calculate metrics using the latest positions for each token.
            metrics = await self.metrics_calculator.update_metrics(latest_positions)
            return metrics

        except Exception as e:
            self.logger.error(f"Error calculating portfolio metrics: {e}")
            raise e
            
        
    def __del__(self):
        """Ensure the database connection is closed on cleanup."""
        if hasattr(self, 'db_conn') and self.db_conn:
            self.db_conn.close() 