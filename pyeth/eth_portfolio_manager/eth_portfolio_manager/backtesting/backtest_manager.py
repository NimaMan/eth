"""
Backtest Execution Engine

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
import time

import tqdm

from eth_data.blockchain.block_processor import BlockProcessor
from eth_portfolio_manager.core.strategy_position_manager import StrategyPositionManager
from eth_portfolio_manager.backtesting.backtest_strategy_engine import BacktestStrategyEngine
from eth_token.token_manager.block_token_processor import BlockTokenProcessor


class BacktestExecutionEngine:
    def __init__(self, config, logger):
        self.config = config
        self.logger = logger
        # Initialize components
        self.block_processor = BlockProcessor(logger=self.logger)
        self.block_token_processor = BlockTokenProcessor(logger=self.logger)
        self.strategy_engines = {}
        for strategy_name, strategy in self.config.strategies.items():
            self.strategy_engines[strategy_name] = BacktestStrategyEngine(investment_strategy=strategy)
        self.strategy_position_managers = {strategy_name: StrategyPositionManager(strategy_engine=self.strategy_engines[strategy_name], logger=self.logger ) for strategy_name, strategy in self.config.strategies.items()}
    
    async def run_backtest(self):
        """Run complete backtest simulation with multiple strategies"""
        try:
            self.logger.info(f"Starting backtest from block {self.config.start_block} to {self.config.end_block}")
            
            # Process blocks sequentially
            current_block = self.config.start_block
            for current_block in tqdm.tqdm(range(self.config.start_block, self.config.end_block + 1)):
                
                # 1. Get block data
                block_result = await self.block_processor.process_block(block_number=current_block)
                
                start_token_process_time = time.time()
                # 2. Process tokens in this block
                await self.block_token_processor.process_block_token(block_result)
                token_updates = self.block_token_processor.updated_tokens
                token_process_time = time.time() - start_token_process_time
                # 3. Update positions for all strategies
                if token_updates:
                    start_strategy_update_time = time.time()
                    tasks = []
                    for strategy_name, position_manager in self.strategy_position_managers.items():
                        # Run all strategies concurrently
                        tasks.append(position_manager.update_updated_tokens_positions(token_updates))
                    await asyncio.gather(*tasks)
                    strategy_update_time = time.time() - start_strategy_update_time
                if not token_updates:
                    strategy_update_time = 0
                self.logger.info(f"{current_block} Tokens {len(token_updates)} | Strategies  {token_process_time:.2f} | {strategy_update_time:.2f}")                
                
                current_block += 1
        except Exception as e:
            self.logger.error(f"Backtest failed: {e}")
            raise
