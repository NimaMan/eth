import os
import time 
import json
from datetime import datetime
from web3 import Web3
import asyncio
from eth_portfolio_manager.backtesting.backtest_manager import BacktestPortfolioManager
from eth_portfolio_manager.strategy.buy_everything import BuyAll
from eth_portfolio_manager.strategy.buy_scam import BuyScamStrategy
from eth_portfolio_manager.backtesting.db.backtest_results_writer import BacktestResultsWriter

# Define log directory for backtest result JSON files
BACKTEST_LOG_DIR = "/home/nima/code/crypto/logs/backtesting"

def serialize_token_position(token_position):
    """
    Helper function to convert a TokenPosition instance into its JSON-serializable dictionary.
    """
    if hasattr(token_position, "to_full_dict"):
        return token_position.to_full_dict()
    return token_position

def save_results_to_json(strategy_name: str, params: dict, start_block: int, end_block: int, token_positions: dict):
    """
    Save backtest results to a JSON file using the new token-centric structure.
    Each token's complete aggregated position (both static and dynamic history) is stored.
    
    Algorithm:
      1. For each token in token_positions (which is a mapping from token_address to a TokenPosition instance),
         convert it to a serializable dict using its `to_full_dict()` method.
      2. Assemble a serializable dictionary with:
           • strategy_name, parameters, start and end blocks,
           • and a mapping from token_address to the aggregated token position dict.
      3. Write the complete JSON to a file.
    """
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"{strategy_name}_{start_block}_{end_block}_{timestamp}.json"
    filepath = os.path.join(BACKTEST_LOG_DIR, filename)
    
    serializable_history = {
        token_address: serialize_token_position(token_position)
        for token_address, token_position in token_positions.items()
    }
    
    serializable_data = {
        "strategy_name": strategy_name,
        "parameters": params,
        "start_block": start_block,
        "end_block": end_block,
        "token_history": serializable_history
    }
    
    with open(filepath, 'w') as f:
        json.dump(serializable_data, f, indent=2, default=str)
    print(f"Saved results to {filepath}")
    return filepath


class BacktestConfig:
    def __init__(self, start_block: int, end_block: int, initial_balance: float = 1.0):
        self.start_block = start_block
        self.end_block = end_block
        self.initial_balance = initial_balance
        self.strategies = {
            "BuyAll": BuyAll(),
            "BuyScam": BuyScamStrategy()
        }       


async def main(num_days=2):
    # Configuration for backtesting
    start_time = time.time()
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    latest_block = w3.eth.get_block_number()
    num_blocks_per_day = 24 * 60 * 60 / 12  # 1 block is 12 seconds roughly 7200 blocks per day
    num_blocks = int(num_days * num_blocks_per_day)
    start_block = latest_block - num_blocks
    end_block = latest_block
    
    config = BacktestConfig(
        start_block=start_block,
        end_block=end_block,
        initial_balance=1.0
    )
    
    # Initialize BacktestManager with the given configuration 
    backtest_manager = BacktestPortfolioManager(config=config)
    
    # Run the backtest (this will update token positions and record their snapshots)
    await backtest_manager.run_backtest()
    
    # Write results to the database using the BacktestResultsWriter
    results_writer = BacktestResultsWriter()
    results_writer.write_backtest_results(backtest_manager, start_block, end_block)
    
    duration = (time.time() - start_time) / 3600
    print(f"\nBacktest completed and saved in {duration:.2f} hours")
    print(f"Results written to database for blocks {start_block} to {end_block}")


if __name__ == "__main__":
    # Run the main backtest function using an event loop
    asyncio.run(main(num_days=14))