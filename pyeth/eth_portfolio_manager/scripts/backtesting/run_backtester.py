import os
import orjson as json
from web3 import Web3
import asyncio
from eth_portfolio_manager.backtesting.backtest_manager import BacktestPortfolioManager
from eth_portfolio_manager.strategy.buy_everything import JustBuyEverythingStrategy
from eth_portfolio_manager.strategy.buy_scam import BuyScamStrategy


# Default log directory; can be customized as needed
ETH_LOG_DIR = os.getenv('ETH_LOG_DIR', '/home/nima/code/crypto/logs')


class BacktestConfig:
    def __init__(self, start_block: int, end_block: int, initial_balance: float = 1.0):
        self.start_block = start_block
        self.end_block = end_block
        self.initial_balance = initial_balance
        self.strategies = [JustBuyEverythingStrategy, BuyScamStrategy]       


async def main():
    # Configuration for backtesting

    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    latest_block = w3.eth.get_block_number()
    num_days = 10
    num_blocks_per_day = int(24*60*60/12)  # 1 block is 12 seconds
    num_blocks = num_days * num_blocks_per_day
    block_range = num_blocks
    start_block = latest_block - block_range
    end_block = latest_block
    
    config = BacktestConfig(
        start_block=start_block,  # Example start block
        end_block=end_block,    # Example end block
        initial_balance=1.0    # Starting balance in ETH
    )
    
    # Initialize BacktestManager with the given configuration
    backtest_manager = BacktestPortfolioManager(config=config)
    
    # Run the backtest
    results = await backtest_manager.run_backtest()
    
    # covert dict keys of each strategy to str 
    results = {str(k): {str(k2): v2 for k2, v2 in v.items()} for k, v in results.items()}
    # Save the results to a file
    log_dir = os.path.join(ETH_LOG_DIR, "backtesting")
    os.makedirs(log_dir, exist_ok=True)
    log_file = os.path.join(log_dir, f"backtest_results_{start_block}_{end_block}.json")
    with open(log_file, "wb") as f:
        f.write(json.dumps(results))

    print(f"Backtest results saved to {log_file}")

if __name__ == "__main__":
    # Run the main function in an event loop
    asyncio.run(main())