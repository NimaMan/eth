from sqlalchemy import text
from eth_portfolio_manager.core.data_models import TOKEN_POSITION_COLUMNS

# Retrieve all raw strategy runs.
STRATEGY_LIST = """
    SELECT *
    FROM strategy_runs
    ORDER BY start_block DESC;
"""

# Get raw strategy run details.
STRATEGY_RUN_DETAILS = """
    SELECT *
    FROM strategy_runs
    WHERE id = %s;
"""

# Get current raw positions for a strategy run (no join on tokens).
STRATEGY_POSITIONS_LIST = """
    SELECT *
    FROM token_positions
    WHERE strategy_run_id = %s
    ORDER BY token_address, block_number DESC;
"""

# Get the list of unique tokens for a strategy run.
TOKEN_LIST_FOR_STRATEGY = """
    SELECT token_address, COUNT(*) AS position_count
    FROM token_positions
    WHERE strategy_run_id = %s
    GROUP BY token_address
    ORDER BY MAX(block_number) DESC;
"""

# Get complete raw position history for a specific token in a strategy.
TOKEN_POSITION_HISTORY = """
    SELECT *
    FROM token_positions
    WHERE strategy_run_id = %s 
      AND token_address = %s
    ORDER BY block_number ASC;
"""

STRATEGY_POSITIONS_LATEST = f"""
    WITH RankedPositions AS (
        SELECT *,
               ROW_NUMBER() OVER (PARTITION BY token_address 
                                ORDER BY block_number DESC) as rn
        FROM token_positions
        WHERE strategy_run_id = %s
    )
    SELECT *
    FROM RankedPositions
    WHERE rn = 1
    ORDER BY block_number DESC;
"""


CURRENCY_TOKENS_LIST = """
    SELECT DISTINCT currency
    FROM token_positions
    WHERE currency IS NOT NULL
"""

def get_strategy_runs_query() -> text:
    """Get all strategy runs with performance metrics."""
    return text(STRATEGY_LIST)

def get_strategy_run_details_query() -> text:
    """Get detailed information for a specific strategy run."""
    return text(STRATEGY_RUN_DETAILS)

def get_strategy_positions_query() -> text:
    """Get current positions for a strategy run."""
    return text(STRATEGY_POSITIONS_LIST)

def get_token_list_for_strategy_query() -> text:
    """Get list of unique tokens for a specific strategy run."""
    return text(TOKEN_LIST_FOR_STRATEGY)

def get_token_position_history_query() -> text:
    """Get full position history for a specific token in a strategy."""
    return text(TOKEN_POSITION_HISTORY)

def get_strategy_positions_latest_query() -> text:
    """Get latest positions for a strategy run."""
    return text(STRATEGY_POSITIONS_LATEST)