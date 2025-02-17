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

# Define log directory
BACKTEST_LOG_DIR = "/home/nima/code/crypto/logs/backtesting"


def save_results_to_json(strategy_name: str, params: dict, start_block: int, end_block: int, token_history: dict):
    """Save backtest results to JSON file using new token-centric structure"""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"{strategy_name}_{start_block}_{end_block}_{timestamp}.json"
    filepath = os.path.join(BACKTEST_LOG_DIR, filename)
    
    # Convert to serializable format
    serializable_data = {
        "strategy_name": strategy_name,
        "parameters": params,
        "start_block": start_block,
        "end_block": end_block,
        "token_history": {
            token_address: [pos for pos in positions]
            for token_address, positions in token_history.items()
        }
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


async def main():
    # Configuration for backtesting
    start_time = time.time()
    w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
    latest_block = w3.eth.get_block_number()
    num_days = 14
    num_blocks_per_day = 24*60*60/12 # 1 block is 12 seconds
    num_blocks = int(num_days * num_blocks_per_day)
    block_range = num_blocks
    start_block = latest_block - block_range
    end_block = latest_block
    
    config = BacktestConfig(
        start_block=start_block,
        end_block=end_block,
        initial_balance=1.0
    )
    
    # Initialize BacktestManager with the given configuration
    backtest_manager = BacktestPortfolioManager(config=config)
    
    # Run the backtest
    await backtest_manager.run_backtest()
    
    
    saved_files = []
    # First save results to JSON
    for strategy_name, strategy in backtest_manager.strategy_position_managers.items():
        strategy_params = strategy.token_position_manager.investment_strategy.get_parameters()
        print(f"\nStrategy {strategy_name} parameters:")
        print(strategy_params)
        print(f"Position history length: {len(strategy.portfolio_tokens_position_history)}")
        
        # Save to JSON first
        json_file = save_results_to_json(
            strategy_name=strategy_name,
            params=strategy_params,
            start_block=start_block,
            end_block=end_block,
            token_history=strategy.portfolio_tokens_position_history
        )
        saved_files.append(json_file)
    
    # Then try to write to database
    results_writer = BacktestResultsWriter()
    # Write results for each strategy
    for strategy_name, strategy in backtest_manager.strategy_position_managers.items():
        try:
            # Get strategy parameters
            strategy_params = strategy.token_position_manager.investment_strategy.get_parameters()
            print(f"\nWriting results for strategy {strategy_name}")
            print(f"Parameters: {strategy_params}")
            print(f"Position history length: {len(strategy.portfolio_tokens_position_history)}")
            
            # Write strategy run and position history
            run_id = await results_writer.write_strategy_run(
                strategy_name=strategy_name,
                params=strategy_params,
                start_block=start_block,
                end_block=end_block
            )
            
            await results_writer.write_position_history(strategy.portfolio_tokens_position_history)
            print(f"Successfully wrote strategy {strategy_name} (ID: {run_id})")
            
        except Exception as e:
            print(f"Error writing results for strategy {strategy_name}: {e}")
            continue
    
    duration = (time.time() - start_time) / 3600
    print(f"\nBacktest completed and saved in {duration:.2f} hours")
    print(f"Results written to database for blocks {start_block} to {end_block}")


if __name__ == "__main__":
    # Run the main function in an event loop
    asyncio.run(main())