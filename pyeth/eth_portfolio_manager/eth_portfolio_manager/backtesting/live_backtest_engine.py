"""
Live Execution Engine

Objective:
---------
1. Process live blockchain data in real-time
2. Apply trading strategies to token updates
3. Maintain portfolio positions
4. Store strategy results after each processed block
5. Support trading strategies in real-time environments

Architecture & Flow:
------------------
1. Initialization & Setup:
   - Initialize LiveTokenStateTracker for token updates
   - Configure portfolio position managers for each strategy
   - Set up result persistence mechanism

2. Token Processing Flow:
   - Subscribe to token updates from LiveTokenStateTracker
   - Process token updates through configured strategies
   - Update portfolio positions based on strategy signals
   - Persist updated positions for API access

3. Lifecycle Management:
   - Handle warm-up phase with historical data
   - Process live updates once warm-up completes
   - Enable graceful shutdown with proper resource cleanup

Components:
----------
1. LiveTokenStateTracker:
   - Provides real-time token updates
   - Handles block subscription and processing

2. PortfolioPositionManager:
   - Manages token positions for each strategy
   - Processes strategy signals

3. ResultWriter:
   - Persists positions and performance data
   - Supports API queries for frontend

State Transitions:
---------------
1. Warmup -> Live:
   - Process historical data for initial state
   - Transition to live processing
   - Begin tracking real-time metrics
"""

import asyncio
import time
from datetime import datetime

from eth_token.token_manager.live_block_token_processor import LiveBlockTokenProcessor
from eth_portfolio_manager.core.strategy_position_manager import StrategyPositionManager
from eth_portfolio_manager.backtesting.backtest_strategy_engine import BacktestStrategyEngine
from eth_data.database.writers.live_token_position_results_writer import LiveResultsWriter
from eth_portfolio_manager.utils.logger import get_logger


class LiveBacktestEngine:
    def __init__(self, config, logger=None, warmup_blocks=1000):
        self.config = config
        self.logger = logger or get_logger(name="portfolio_live")
        self.warmup_blocks = warmup_blocks
        
        # Initialize token manager to handle both warm-up and live phases
        self.live_token_processor = LiveBlockTokenProcessor(
            logger=self.logger,
            warmup_blocks=self.warmup_blocks
        )
        
        # Initialize Token Position managers for each strategy
        self.strategy_engines = {}
        for strategy_name, strategy in self.config.strategies.items():
            self.strategy_engines[strategy_name] = BacktestStrategyEngine(investment_strategy=strategy)
        self.strategy_position_managers = {strategy_name: StrategyPositionManager(strategy_engine=self.strategy_engines[strategy_name], logger=self.logger ) for strategy_name, strategy in self.config.strategies.items()}
        
        # Results writer for live database updates
        self.results_writer = LiveResultsWriter(logger=self.logger)
        
        # Store strategy run IDs to update the same run each time
        self.strategy_run_ids = {}
        
        # Monitoring and control
        self._is_shutting_down = False
        self._update_task = None
            
    async def _process_token_updates(self):
        """Process token updates as they come in and save results after each block"""
        while not self._is_shutting_down:
            try:
                # Wait for new token updates
                await self.live_token_processor.new_updates_event.wait()
                self.live_token_processor.new_updates_event.clear()
                
                # Process all pending updates in order
                while not self.live_token_processor.unprocessed_token_updates.empty():
                    # Get the next update (block_number, updated_tokens)
                    block_number, updated_tokens = await self.live_token_processor.unprocessed_token_updates.get()
                    
                    if updated_tokens:                        
                        # Process through each strategy concurrently
                        tasks = []
                        self.updated_positions_by_strategy = {}  # Track updated positions per strategy
                        for strategy_name, position_manager in self.strategy_position_managers.items():
                            self.updated_positions_by_strategy[strategy_name] = set()
                            tasks.append(self._update_strategy_positions(strategy_name, position_manager, updated_tokens, self.updated_positions_by_strategy))
                        
                        await asyncio.gather(*tasks)
                        
                        # Save results immediately after processing the block
                        save_start_time = time.time()
                        await self._save_strategy_results_to_database(block_number)
                        save_time = time.time() - save_start_time
                        self.logger.info(f"{block_number}: Saved positions in {save_time:.2f}s")
                    
                    # Mark task as done
                    self.live_token_processor.unprocessed_token_updates.task_done()
                
            except asyncio.CancelledError:
                break
            except Exception as e:
                self.logger.error(f"Error processing token updates: {e}")
                await asyncio.sleep(1)  # Avoid tight loop on persistent errors
    
    async def _update_strategy_positions(self, strategy_name, position_manager, updated_tokens, updated_positions_by_strategy):
        """Update positions for a strategy and track which tokens were updated"""
        # Get the updated token positions from the strategy engine {token_address: {pool_address: TokenPosition}}
        updated_positions_dict = await position_manager.update_updated_tokens_positions(updated_tokens)
        
        # Store the composite keys of updated positions (token_address-pool_address)
        if updated_positions_dict:
            # For each token address and its pool positions
            for token_address, pool_positions in updated_positions_dict.items():
                # For each pool address with a position
                for pool_address, position in pool_positions.items():
                    if position:  # Only track non-None positions
                        # Create the composite key used in token_positions_cache
                        composite_key = f"{token_address}-{pool_address}"
                        updated_positions_by_strategy[strategy_name].add(composite_key)
        
        return updated_positions_dict
    
    async def _save_strategy_results_to_database(self, block_number):
        """
        Save current strategy results to database
        
        Args:
            block_number: Optional block number for logging. If None, gets latest from token manager.
        """
        try:
            # If block_number not provided, get latest from token manager
            if block_number is None:
                block_number = self.live_token_processor.latest_processed_block            
            start_block = max(1, block_number - self.warmup_blocks)
            
            # Save each strategy to database
            for strategy_name, strategy_position_manager in self.strategy_position_managers.items():
                try:
                    strategy_params = strategy_position_manager.strategy_engine.strategy_parameters
                    
                    # Add live flag and timestamp to parameters
                    live_params = {
                        **strategy_params, 
                        "mode": "LIVE", 
                        "updated_at": datetime.now().isoformat(),
                        "current_block": block_number
                    }
                    
                    # Use the LIVE_ prefix for this strategy run
                    live_strategy_name = f"LIVE_{strategy_name}"
                    # Create or update a running live strategy and obtain the run ID
                    run_id = self.results_writer.create_or_update_strategy_run(
                        strategy_name=live_strategy_name,
                        params=live_params,
                        start_block=start_block,
                        end_block=block_number
                    )
                    self.strategy_run_ids[live_strategy_name] = run_id
                    
                    # Get only the positions that were updated in this block
                    if strategy_name in self.updated_positions_by_strategy and self.updated_positions_by_strategy[strategy_name]:
                        updated_token_keys = self.updated_positions_by_strategy[strategy_name]
                        # Extract only the updated positions from the cache
                        updated_positions = {key: position for key, position in 
                                            strategy_position_manager.token_positions_cache.items() 
                                            if key in updated_token_keys}
                        if updated_positions:
                            try:
                                # Update token positions for this run; the writer handles INSERT vs UPDATE internally
                                self.results_writer.update_token_positions(run_id, updated_positions)
                            except Exception as e:
                                self.logger.error(f"Error writing position history for {strategy_name}: {e}")
                except Exception as e:
                    # Isolate errors to individual strategies
                    self.logger.error(f"Error processing strategy {strategy_name} for block {block_number}: {e}")
        except Exception as e:
            self.logger.error(f"Error saving strategy results for block {block_number}: {e}")

    async def start(self):
        """Start the live execution engine"""
        try:
            self.logger.info("Starting live execution engine")
            self._is_shutting_down = False
            
            # Start token manager (this handles warm-up and live phases)
            token_manager_task = asyncio.create_task(self.live_token_processor.start())
            # Create task for position updates and saving
            self._update_task = asyncio.create_task(self._process_token_updates())
            
            # Keep running until shutdown
            while not self._is_shutting_down:
                await asyncio.sleep(1)
                
        except Exception as e:
            self.logger.error(f"Error starting live execution engine: {e}")
            await self.stop()
            raise
    
    async def stop(self):
        """Stop the live execution engine"""
        if self._is_shutting_down:
            return
            
        self._is_shutting_down = True
        self.logger.info("Stopping live execution engine...")
        
        try:
            # Stop the token manager
            await self.live_token_processor.stop()
            
            # Cancel update task
            if self._update_task and not self._update_task.done():
                self._update_task.cancel()
                try:
                    await self._update_task
                except asyncio.CancelledError:
                    pass
            
            # Final save of strategy results
            await self._save_strategy_results_to_database()
                
            self.logger.info("Live execution engine stopped successfully")
            
        except Exception as e:
            self.logger.error(f"Error during live execution engine shutdown: {e}")
            raise
