"""
Live Results Writer

Objective:
----------
This module efficiently writes live trading results to PostgreSQL on a block-by-block basis.
It optimizes database operations by only updating positions that have changed in each block.

Algorithm:
----------
1. Strategy Run Management:
   - Creates a single strategy run record per live strategy
   - Updates end_block and parameters as new blocks are processed
   
2. Efficient Token Position Updates:
   - Maintains a set of already saved token addresses
   - For new tokens: Performs INSERT operations
   - For existing tokens: Performs UPDATE operations
   - Uses bulk operations where possible to minimize database roundtrips
"""

import json
from datetime import datetime
import psycopg2
from psycopg2.extras import RealDictCursor


class LiveResultsWriter:
    def __init__(self, logger):
        # Establish database connection
        self.conn = psycopg2.connect(
            dbname="backtest",
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )
        self.cur = self.conn.cursor(cursor_factory=RealDictCursor)
        self.logger = logger
        
        # Track tokens we've already saved to determine INSERT vs UPDATE
        self.saved_tokens = {}  # {strategy_run_id: {token_address-pool_address}}
    
    def create_or_update_strategy_run(self, strategy_name, params, start_block, end_block):
        """
        Creates a new strategy run or updates an existing one.
        Each run with a different start_block should be treated as a new run.
        """
        # Check if this strategy run exists for this specific start block
        self.cur.execute(
            "SELECT id FROM strategy_runs WHERE name = %s AND start_block = %s ORDER BY created_at DESC LIMIT 1",
            (strategy_name, start_block)
        )
        result = self.cur.fetchone()
        
        if result:
            # Update existing run
            run_id = result['id']
            self.cur.execute(
                """
                UPDATE strategy_runs 
                SET parameters = %s, end_block = %s, created_at = %s
                WHERE id = %s
                """,
                (json.dumps(params), end_block, datetime.now(), run_id)
            )
            self.conn.commit()
            return run_id
        else:
            # Create new run
            self.cur.execute(
                """
                INSERT INTO strategy_runs (name, parameters, start_block, end_block, created_at)
                VALUES (%s, %s, %s, %s, %s)
                RETURNING id
                """,
                (strategy_name, json.dumps(params), start_block, end_block, datetime.now())
            )
            run_id = self.cur.fetchone()['id']
            self.conn.commit()
            
            # Initialize the saved tokens set for this run
            self.saved_tokens[run_id] = set()
            
            return run_id
    
    def create_strategy_run(self, strategy_name, params, start_block, created_at):
        """
        Create a new strategy run (wrapper for create_or_update_strategy_run).
        """
        # Use current block as end_block for initial creation
        return self.create_or_update_strategy_run(strategy_name, params, start_block, start_block)
    
    def update_token_positions(self, run_id, token_positions):
        """
        Efficiently updates token positions, performing inserts for new tokens
        and updates for existing ones.
        
        Args:
            run_id: The strategy run ID
            token_positions: Dictionary mapping token keys to position objects
        """
        if not token_positions:
            return
            
        # Initialize saved tokens set if not present
        if run_id not in self.saved_tokens:
            self.saved_tokens[run_id] = set()
            
        # Separate tokens into new and existing
        new_tokens = {}
        existing_tokens = {}
        
        for token_key, position in token_positions.items():
            if token_key in self.saved_tokens[run_id]:
                existing_tokens[token_key] = position
            else:
                new_tokens[token_key] = position
                self.saved_tokens[run_id].add(token_key)
        
        # Process new tokens with INSERT
        if new_tokens:
            self._insert_new_tokens(run_id, new_tokens)
            
        # Process existing tokens with UPDATE
        if existing_tokens:
            self._update_existing_tokens(run_id, existing_tokens)
    
    def _insert_new_tokens(self, run_id, token_positions):
        """Insert new token positions"""
        insert_values = []
        
        for token_key, position in token_positions.items():
            # Parse token_key (expected format: token_address-pool_address)
            key_parts = token_key.split('-')
            token_address = key_parts[0]
            pool_address = key_parts[1] if len(key_parts) > 1 else None
            
            # Get currency from position if available
            currency = None
            if hasattr(position, 'static_data') and hasattr(position.static_data, 'currency'):
                currency = position.static_data.currency
                
            # Convert to dictionary
            position_dict = position.to_full_dict()
            
            insert_values.append((
                run_id,
                token_address,
                pool_address,
                currency,
                json.dumps(position_dict)
            ))
        
        # Bulk insert
        if insert_values:
            insert_sql = """
                INSERT INTO token_positions (
                    strategy_run_id, token_address, pool_address, currency, token_position
                ) VALUES (%s, %s, %s, %s, %s)
            """
            self.cur.executemany(insert_sql, insert_values)
            self.conn.commit()
    
    def _update_existing_tokens(self, run_id, token_positions):
        """Update existing token positions"""
        for token_key, position in token_positions.items():
            # Parse token_key
            key_parts = token_key.split('-')
            token_address = key_parts[0]
            pool_address = key_parts[1] if len(key_parts) > 1 else None
            
            # Get currency from position if available
            currency = None
            if hasattr(position, 'static_data') and hasattr(position.static_data, 'currency'):
                currency = position.static_data.currency
                
            # Convert to dictionary
            position_dict = position.to_full_dict()
            
            # Update
            update_sql = """
                UPDATE token_positions
                SET token_position = %s, currency = %s
                WHERE strategy_run_id = %s AND token_address = %s AND pool_address IS NOT DISTINCT FROM %s
            """
            self.cur.execute(
                update_sql,
                (json.dumps(position_dict), currency, run_id, token_address, pool_address)
            )
        
        self.conn.commit()
    
    def __del__(self):
        if hasattr(self, 'cur'):
            self.cur.close()
        if hasattr(self, 'conn'):
            self.conn.close() 