"""
Backtest Results Writer

Objective:
----------
This module writes backtesting results (strategy run metadata and aggregated token positions)
to a PostgreSQL database. It leverages our updated data models and the TokenPosition aggregate,
which serializes the complete token position (static plus dynamic history) via the to_full_dict() method.

Algorithm:
----------
1. Strategy Run Insertion:
   - write_strategy_run() inserts a record for the current run (including strategy name, parameters,
     start and end blocks, and timestamp) and returns the generated run id.
2. Token Position Persistence:
   - write_position_history() takes a dictionary mapping token addresses to aggregated token position data.
   - Each token position is assumed to be a dictionary as produced by TokenPosition.to_full_dict().
   - The writer inserts a record for each token position with:
       • strategy_run_id: Reference to the strategy run.
       • token_address: Unique token identifier.
       • token_position: A JSON-serialized comprehensive view of the token position (static data and dynamic snapshots).
3. The resulting records allow full historical retrieval, analysis via the API, and debugging.

This design ensures consistency and traceability by persisting the full evolution of each token's state.
"""

import json
from datetime import datetime
import psycopg2
from psycopg2.extras import RealDictCursor
import os


class BacktestResultsWriter:
    def __init__(self, logger):
        # Establish a direct database connection.
        self.conn = psycopg2.connect(
            dbname="backtest",
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )
        self.cur = self.conn.cursor(cursor_factory=RealDictCursor)
        self.current_run_id = None
        self.logger = logger
    
    def write_strategy_run(self, strategy_name: str, params: dict, start_block: int, end_block: int) -> int:
        """
        Inserts a new strategy run record into the database, returning the generated run ID.
        """
        try:
            self.cur.execute(
                """
                INSERT INTO strategy_runs (name, parameters, start_block, end_block, created_at)
                VALUES (%s, %s, %s, %s, %s)
                RETURNING id
                """,
                (strategy_name, json.dumps(params), start_block, end_block, datetime.now())
            )
            self.current_run_id = self.cur.fetchone()['id']
            self.conn.commit()
            self.logger.info(f"Created strategy run with ID: {self.current_run_id} and name: {strategy_name}")
            return self.current_run_id
        except Exception as e:
            self.conn.rollback()
            self.logger.error(f"Error writing strategy run: {e}")
            raise

    def write_position_history(self, token_history: dict):
        """
        Writes token positions to the database.

        Args:
            token_history: A dictionary mapping composite keys (token_address-pool_address) 
                           to TokenPosition objects.
        
        Process:
            - Iterates over all tokens in token_history.
            - Extracts token_address and pool_address from the composite key.
            - Serializes the token position to JSON.
            - Inserts each record with strategy_run_id, token_address, pool_address, and token_position.
        """
        try:
            insert_sql = """
                INSERT INTO token_positions (
                    strategy_run_id,
                    token_address,
                    pool_address,
                    currency,
                    token_position
                ) VALUES (
                    %s, %s, %s, %s, %s
                )
                ON CONFLICT (strategy_run_id, token_address, pool_address) 
                DO UPDATE SET token_position = EXCLUDED.token_position,
                              currency = EXCLUDED.currency
            """

            total_positions = 0
            for composite_key, token_position in token_history.items():
                # Extract token_address and pool_address from the composite key
                key_parts = composite_key.split('-')
                if len(key_parts) >= 2:
                    token_address = key_parts[0]
                    pool_address = key_parts[1]
                else:
                    # Handle legacy keys that might not have pool_address
                    token_address = composite_key
                    pool_address = None
                    
                # Get currency from token position if available
                currency = None
                if hasattr(token_position, 'static_data') and hasattr(token_position.static_data, 'currency'):
                    currency = token_position.static_data.currency
                    
                # Convert TokenPosition to dictionary
                position_dict = token_position.to_full_dict()
                    
                values = (
                    self.current_run_id,
                    token_address,
                    pool_address,
                    currency,
                    json.dumps(position_dict)
                )
                self.cur.execute(insert_sql, values)
                total_positions += 1

            self.conn.commit()
            self.logger.info(f"Successfully wrote {total_positions} token positions to database")
        except Exception as e:
            self.conn.rollback()
            self.logger.error(f"Error writing position history: {e}")
            raise

    def write_backtest_results(self, backtest_manager, start_block: int, end_block: int):
        """
        Writes backtest results to the database.
        """
        for strategy_name, strategy_position_manager in backtest_manager.strategy_position_managers.items():
            try:
                token_position_manager = strategy_position_manager.token_position_manager
                strategy_params = token_position_manager.investment_strategy.get_parameters()
                print(f"\nWriting results for strategy {strategy_name}")
                print(f"Parameters: {strategy_params}")
                num_positions = len(strategy_position_manager.token_positions_cache)
                print(f"Aggregated token positions count: {num_positions}")
                if num_positions == 0:
                    continue
                run_id = self.write_strategy_run(
                    strategy_name=strategy_name,
                    params=strategy_params,
                    start_block=start_block,
                    end_block=end_block
                )
                
                # Get token positions from the cache
                token_positions = strategy_position_manager.token_positions_cache.items()
                self.write_position_history(token_positions)
                self.logger.info(f"Successfully wrote strategy {strategy_name} (ID: {run_id})")
                
            except Exception as e:
                self.logger.error(f"Error writing results for strategy {strategy_name}: {e}")
                raise

    def __del__(self):
        if hasattr(self, 'cur'):
            self.cur.close()
        if hasattr(self, 'conn'):
            self.conn.close()
