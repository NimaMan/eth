import json
import asyncio
from datetime import datetime
import os
import psycopg2
from psycopg2.extras import RealDictCursor
from eth_portfolio_manager.core.data_models import get_token_position_columns_sql, TOKEN_POSITION_COLUMNS


class BacktestResultsWriter:
    def __init__(self):
        # Direct database connection
        self.conn = psycopg2.connect(
            dbname="backtest",
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )
        self.cur = self.conn.cursor(cursor_factory=RealDictCursor)
        self.current_run_id = None

    async def write_strategy_run(self, strategy_name: str, params: dict, start_block: int, end_block: int) -> int:
        try:
            self.cur.execute("""
                INSERT INTO strategy_runs (name, parameters, start_block, end_block, created_at)
                VALUES (%s, %s, %s, %s, %s)
                RETURNING id
            """, (strategy_name, json.dumps(params), start_block, end_block, datetime.now()))
            
            self.current_run_id = self.cur.fetchone()['id']
            self.conn.commit()
            print(f"Created strategy run with ID: {self.current_run_id} and name: {strategy_name}")
            return self.current_run_id
            
        except Exception as e:
            self.conn.rollback()
            print(f"Error writing strategy run: {e}")
            raise

    async def write_position_history(self, token_history: dict):
        try:
            columns, placeholders = get_token_position_columns_sql()
            insert_sql = f"""
                INSERT INTO token_positions (
                    {columns}
                ) VALUES (
                    {placeholders}
                )
            """
            
            total_positions = 0
            for token_address, positions in token_history.items():
                for position_dict in positions:
                    # Convert dictionary to database tuple format
                    values = tuple(
                        self.current_run_id if col == 'strategy_run_id' 
                        else position_dict.get(col)
                        for col in TOKEN_POSITION_COLUMNS
                    )
                    
                    self.cur.execute(insert_sql, values)
                    total_positions += 1
            
            self.conn.commit()
            print(f"Successfully wrote {total_positions} positions across {len(token_history)} tokens")
            
        except Exception as e:
            self.conn.rollback()
            print(f"Error writing position history: {e}")
            print(f"Failed token: {token_address}, position block: {position_dict.get('last_updated_block', 'N/A')}")
            raise

    def __del__(self):
        if hasattr(self, 'cur'):
            self.cur.close()
        if hasattr(self, 'conn'):
            self.conn.close()


if __name__ == "__main__":

    async def test_write_from_json():
        # Load JSON file
        base_dir = "/home/nima/code/crypto/logs/backtesting/"
        # Get json files in base_dir
        json_files = [f for f in os.listdir(base_dir) if f.endswith('.json')]
        writer = BacktestResultsWriter()
        try:
            for json_file in json_files:
                with open(base_dir + json_file, 'r') as f:
                    data = json.load(f)
                    print("\nLoaded JSON data:")
                    print(f"Strategy: {data['strategy_name']}")
                    print(f"Blocks: {data['start_block']} to {data['end_block']}")
                    print(f"Number of blocks with positions: {len(data['token_history'])}")
            
            
                # Write strategy run
                await writer.write_strategy_run(
                    strategy_name=data['strategy_name'],
                    params=data['parameters'],
                    start_block=data['start_block'],
                    end_block=data['end_block']
                )
                
                # Write position history
                await writer.write_position_history(data['token_history'])
                
                print("Successfully wrote backtest results to database")
                
        except Exception as e:
            print(f"Failed to write results: {e}")
        finally:
            del writer  # This will close connections
            
    asyncio.run(test_write_from_json())