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
import time
from dataclasses import dataclass
from typing import Dict
from collections import defaultdict, OrderedDict

from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_portfolio_manager.backtesting.backtest_portfolio_position_manager import PortfolioPositionManagerBacktest
from eth_token_monitor.token_manager.block_token_processor import BlockTokenProcessor
from eth_portfolio_manager.utils.logger import get_logger
from eth_portfolio_manager.strategy.buy_everything import JustBuyEverythingStrategy
from eth_portfolio_manager.strategy.buy_scam import BuyScamStrategy


STRATEGY_NAME = "LiveTokenPositionManager"
BUY_EVERYTHING_STRATEGY = JustBuyEverythingStrategy
BUY_SCAM_STRATEGY = BuyScamStrategy



@dataclass
class BacktestConfig:
    start_block: int
    end_block: int
    initial_balance: float = 1.0  # ETH


class BacktestManager:
    def __init__(self, config: BacktestConfig):
        self.config = config
        self.logger = get_logger(name="backtester")
        self.portfolio_performance_history = defaultdict(OrderedDict)
        # Initialize components
        self.block_processor = BlockProcessor(logger=self.logger)
        self.block_token_processor = BlockTokenProcessor(logger=self.logger)
        self.buy_everything_position_manager = PortfolioPositionManagerBacktest( investment_strategy_class=BUY_EVERYTHING_STRATEGY, logger=self.logger )
        self.buy_scam_position_manager = PortfolioPositionManagerBacktest( investment_strategy_class=BUY_SCAM_STRATEGY, logger=self.logger )
        self.strategies = [self.buy_everything_position_manager, self.buy_scam_position_manager]
    
    async def run_backtest(self):
        """Run complete backtest simulation with multiple strategies"""
        try:
            self.logger.info(f"Starting backtest from block {self.config.start_block} to {self.config.end_block}")
            
            # Initialize performance history for each strategy
            for strategy in self.strategies:
                self.portfolio_performance_history[strategy.strategy_name] = OrderedDict()
            
            # Process blocks sequentially
            current_block = self.config.start_block
            while current_block <= self.config.end_block:
                start_time = time.time()
                
                # 1. Get block data
                block_data = await self.block_processor.process_block(block_number=current_block)
                
                # 2. Process tokens in this block
                await self.block_token_processor.process_block(block_data)
                token_updates = self.block_token_processor.updated_tokens
                if token_updates:
                    # Run all strategies concurrently
                    tasks = [self.run_strategy_for_block(position_manager, token_updates, current_block) for position_manager in self.strategies]
                    await asyncio.gather(*tasks)
                
                self.logger.info(f"Backtest: Processed block {current_block} in {time.time() - start_time:.2f} seconds")
                current_block += 1
                
            return self.portfolio_performance_history
            
        except Exception as e:
            self.logger.error(f"Backtest failed: {e}")
            raise

    async def run_strategy_for_block(self, position_manager, token_updates, current_block):
        """Run a single strategy for a given block and record its performance."""
        await position_manager.update_token_positions(token_updates)
        # Record state for this strategy
        self.portfolio_performance_history[position_manager.strategy_name][current_block] = await position_manager.get_portfolio_metrics()
        