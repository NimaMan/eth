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
import asyncio
from datetime import datetime
import psycopg2
from psycopg2.extras import RealDictCursor
import os


class BacktestResultsWriter:
    def __init__(self):
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
            print(f"Created strategy run with ID: {self.current_run_id} and name: {strategy_name}")
            return self.current_run_id
        except Exception as e:
            self.conn.rollback()
            print(f"Error writing strategy run: {e}")
            raise

    def write_position_history(self, token_history: dict):
        """
        Writes token positions to the database.

        Args:
            token_history: A dictionary mapping token_address (string) to an aggregated token position dictionary.
                           The token position is expected to be in the format returned by TokenPosition.to_full_dict(),
                           including both 'static' and 'dynamic_history' keys.
        
        Process:
            - Iterates over all tokens in token_history.
            - Serializes the token position to JSON.
            - Inserts each record into the token_positions table with (strategy_run_id, token_address, token_position).
        """
        try:
            insert_sql = """
                INSERT INTO token_positions (
                    strategy_run_id,
                    token_address,
                    token_position
                ) VALUES (
                    %s, %s, %s
                )
            """

            total_positions = 0
            for token_address, token_position in token_history.items():
                # token_position is expected to be a dict as produced by TokenPosition.to_full_dict()
                values = (
                    self.current_run_id,
                    token_address,
                    json.dumps(token_position)
                )
                self.cur.execute(insert_sql, values)
                total_positions += 1

            self.conn.commit()
            print(f"Successfully wrote {total_positions} token positions to database")
        except Exception as e:
            self.conn.rollback()
            print(f"Error writing position history: {e}")
            # If available, report the failed token
            raise

    def write_backtest_results(self, backtest_manager, start_block: int, end_block: int):
        """
        Writes backtest results to the database.
        """
        for strategy_name, strategy in backtest_manager.strategy_position_managers.items():
            try:
                strategy_params = strategy.token_position_manager.investment_strategy.get_parameters()
                print(f"\nWriting results for strategy {strategy_name}")
                print(f"Parameters: {strategy_params}")
                print(f"Aggregated token positions count: {len(strategy.token_positions)}")
                
                run_id = self.write_strategy_run(
                    strategy_name=strategy_name,
                    params=strategy_params,
                    start_block=start_block,
                    end_block=end_block
                )
                
                aggregated_history = {
                    token_address: token_position.to_full_dict()
                    for token_address, token_position in strategy.token_positions.items()
                }
                self.write_position_history(aggregated_history)
                print(f"Successfully wrote strategy {strategy_name} (ID: {run_id})")
                
            except Exception as e:
                print(f"Error writing results for strategy {strategy_name}: {e}")
                raise  # Changed from continue to raise to see errors

    def __del__(self):
        if hasattr(self, 'cur'):
            self.cur.close()
        if hasattr(self, 'conn'):
            self.conn.close()


if __name__ == "__main__":
    # Test database writing with sample JSON file
    writer = BacktestResultsWriter()
    BACKTEST_LOG_DIR = "/home/nima/code/crypto/logs/backtesting"
    json_path = os.path.join(BACKTEST_LOG_DIR, "BuyAll_21887626_21888346_20250220_160701.json")
    
    print(f"Loading test data from: {json_path}")
    with open(json_path, 'r') as f:
        data = json.load(f)
    
    try:
        # Write strategy run
        run_id = writer.write_strategy_run(
            strategy_name=data['strategy_name'],
            params=data['parameters'],
            start_block=data['start_block'],
            end_block=data['end_block']
        )
        print(f"Created strategy run with ID: {run_id}")
        
        # Write token positions
        writer.write_position_history(data['token_history'])
        
        # Verify the write
        writer.cur.execute("SELECT COUNT(*) FROM strategy_runs")
        strategy_count = writer.cur.fetchone()['count']
        
        writer.cur.execute("SELECT COUNT(*) FROM token_positions")
        position_count = writer.cur.fetchone()['count']
        
        print(f"\nVerification Results:")
        print(f"Strategy runs in database: {strategy_count}")
        print(f"Token positions in database: {position_count}")
        print(f"Token positions in JSON: {len(data['token_history'])}")
        
    except Exception as e:
        print(f"Error during test: {e}")
        raise
    finally:
        writer.conn.close()
