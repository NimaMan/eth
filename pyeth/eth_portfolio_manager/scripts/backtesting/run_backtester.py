import os
import orjson as json
from web3 import Web3
import asyncio
from eth_portfolio_manager.backtesting.backtest_manager import BacktestManager, BacktestConfig


# Default log directory; can be customized as needed
ETH_LOG_DIR = os.getenv('ETH_LOG_DIR', '/home/nima/code/crypto/logs')


async def main():
    # Configuration for backtesting

    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    latest_block = w3.eth.get_block_number()
    block_range = 10000
    start_block = latest_block - block_range
    end_block = latest_block
    
    config = BacktestConfig(
        start_block=start_block,  # Example start block
        end_block=end_block,    # Example end block
        initial_balance=1.0    # Starting balance in ETH
    )
    
    # Initialize BacktestManager with the given configuration
    backtest_manager = BacktestManager(config=config)
    
    # Run the backtest
    results = await backtest_manager.run_backtest()
    
    # covert dict keys to str 
    results = {str(k): v for k, v in results.items()}
    # Save the results to a file
    log_dir = os.path.join(ETH_LOG_DIR, "backtesting")
    log_file = os.path.join(log_dir, f"backtest_results_{start_block}_{end_block}.json")
    with open(log_file, "wb") as f:
        f.write(json.dumps(results))


if __name__ == "__main__":
    # Run the main function in an event loop
    asyncio.run(main())