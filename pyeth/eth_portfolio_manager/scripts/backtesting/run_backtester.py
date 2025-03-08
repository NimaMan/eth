import os
import time 
import json
from datetime import datetime
from web3 import Web3
import asyncio
from eth_portfolio_manager.backtesting.backtest_manager import BacktestPortfolioManager
from eth_portfolio_manager.strategy import BuyAll, BuyScamStrategy, MarketTracker
from eth_portfolio_manager.backtesting.db.backtest_results_writer import BacktestResultsWriter
from eth_portfolio_manager.utils.logger import get_logger


logger = get_logger(name="backtester", log_folder="backtesting")


class BacktestConfig:
    def __init__(self, start_block: int, end_block: int, initial_balance: float = 1.0):
        self.start_block = start_block
        self.end_block = end_block
        self.initial_balance = initial_balance
        self.strategies = {
            "MarketTracker": MarketTracker(),
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
    backtest_manager = BacktestPortfolioManager(
        config=config,
        logger=logger
    )
    
    # Run the backtest (this will update token positions and record their snapshots)
    await backtest_manager.run_backtest()
    
    # Write results to the database using the BacktestResultsWriter
    results_writer = BacktestResultsWriter(
        logger=logger
    )
    results_writer.write_backtest_results(backtest_manager, start_block, end_block)
    
    duration = (time.time() - start_time) / 3600
    print(f"\nBacktest completed and saved in {duration:.2f} hours")
    print(f"Results written to database for blocks {start_block} to {end_block}")


if __name__ == "__main__":
    # Run the main backtest function using an event loop
    asyncio.run(main(num_days=14))