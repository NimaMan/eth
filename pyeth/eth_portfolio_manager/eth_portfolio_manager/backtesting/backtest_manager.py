"""
Strategy Backtesting Manager

Objective:
---------
1. Test trading strategies on historical blockchain data
2. Simulate portfolio management with past token data
3. Generate performance analytics
4. Compare strategy results

Backtesting Flow:
---------------
1. Historical Data Processing:
   - Process blocks sequentially using BlockTokenProcessor
   - Track token states and updates
   - Maintain chronological order

2. Strategy Simulation:
   - Feed token updates to strategy
   - Generate and process trading signals
   - Track position changes
   - Calculate P&L

3. Portfolio Simulation:
   - Manage simulated positions
   - Track portfolio value
   - Calculate metrics
   - Store trade history

4. Performance Analysis:
   - Calculate returns
   - Generate trade statistics
   - Analyze risk metrics
   - Compare strategies

Implementation Notes:
------------------
1. Uses BlockRangeTokenProcessor for historical data
2. Simulates portfolio management without actual trades
3. Maintains chronological order of events
4. Tracks complete trading history
"""
import asyncio
import tqdm
import time

from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_portfolio_manager.core.portfolio_position_manager import PortfolioPositionManager
from eth_portfolio_manager.backtesting.backtest_token_position_managers import TokenPositionManagerBacktest
from eth_token_monitor.token_manager.block_token_processor import BlockTokenProcessor
from eth_portfolio_manager.utils.logger import get_logger


class BacktestPortfolioManager:
    def __init__(self, config):
        self.config = config
        self.logger = get_logger(name="backtester", log_folder="backtesting")
        # Initialize components
        self.block_processor = BlockProcessor(logger=self.logger)
        self.block_token_processor = BlockTokenProcessor(logger=self.logger)
        self.token_position_managers = {}
        for strategy_name, strategy in self.config.strategies.items():
            self.token_position_managers[strategy_name] = TokenPositionManagerBacktest(investment_strategy=strategy)
        self.strategy_position_managers = {strategy_name: PortfolioPositionManager(token_position_manager=self.token_position_managers[strategy_name], logger=self.logger ) for strategy_name, strategy in self.config.strategies.items()}
    
    async def run_backtest(self):
        """Run complete backtest simulation with multiple strategies"""
        try:
            self.logger.info(f"Starting backtest from block {self.config.start_block} to {self.config.end_block}")
            
            # Process blocks sequentially
            current_block = self.config.start_block
            for current_block in tqdm.tqdm(range(self.config.start_block, self.config.end_block + 1)):
                
                # 1. Get block data
                block_data = await self.block_processor.process_block(block_number=current_block)
                
                start_token_process_time = time.time()
                # 2. Process tokens in this block
                await self.block_token_processor.process_block(block_data)
                token_updates = self.block_token_processor.updated_tokens
                token_process_time = time.time() - start_token_process_time
                self.logger.info(f"[Performance] Token processing took {token_process_time:.2f}s for block {current_block}")
                # 3. Update positions for all strategies
                if token_updates:
                    start_strategy_update_time = time.time()
                    tasks = []
                    for strategy_name, position_manager in self.strategy_position_managers.items():
                        # Run all strategies concurrently
                        tasks.append(position_manager.update_portfolio_tokens_positions(token_updates))
                    await asyncio.gather(*tasks)
                    strategy_update_time = time.time() - start_strategy_update_time
                    self.logger.info(f"[Performance] Strategy updates took {strategy_update_time:.2f}s for block {current_block}")                
                
                current_block += 1
        except Exception as e:
            self.logger.error(f"Backtest failed: {e}")
            raise
