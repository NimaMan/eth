"""
LiveBacktestEngineWithPools: Real-time Strategy Execution with Pool Tracking and Publishing

Objective:
---------
1. Process confirmed blockchain blocks and update token states
2. Execute trading strategies based on token updates
3. Extract and publish pool information with creator tracking
4. Enable comprehensive mempool monitoring via ZeroMQ
5. Track creator behavior patterns for risk assessment
"""

import asyncio
from typing import Dict, Any, Optional
from web3 import Web3

from eth_token.token_manager.live_block_token_processor import LiveBlockTokenProcessor
from eth_portfolio_manager.core.strategy_position_manager import StrategyPositionManager
from eth_portfolio_manager.backtesting.backtest_strategy_engine import BacktestStrategyEngine
from eth_data.database.writers.live_token_position_results_writer import LiveResultsWriter
from eth_portfolio_manager.notifications.token_info_extractor import TokenInfoExtractor
from eth_portfolio_manager.notifications.token_info_publisher import TokenInfoPublisher
from eth_portfolio_manager.utils.logger import get_logger


class LiveBacktestEngineWithPools:
    """
    Live execution engine with pool tracking and publishing.
    Handles strategy execution, pool tracking, and ZeroMQ publishing.
    """
    
    def __init__(self,
                 config,
                 logger=None,
                 warmup_blocks=1000,
                 save_strategy_results: bool = True,
                 add_pnl_to_db: bool = True,
                 zmq_pub_endpoint: str = "tcp://*:5557",
                 zmq_rep_endpoint: str = "tcp://*:5558",
                 max_pools: int = 2000,
                 min_eth_threshold: float = 0.01):
        """
        Initialize the live engine with pool tracking.
        
        Args:
            config: Configuration with strategies
            logger: Logger instance
            warmup_blocks: Number of blocks to warm up
            save_strategy_results: Whether to save results to DB
            add_pnl_to_db: Whether to add PnL to DB
            zmq_pub_endpoint: ZeroMQ PUB endpoint
            zmq_rep_endpoint: ZeroMQ REP endpoint
            max_pools: Maximum number of pools to track
            min_eth_threshold: Minimum ETH threshold for pools
        """
        self.logger = logger or get_logger("live_backtest_pools")
        self.config = config
        self.warmup_blocks = warmup_blocks
        self.save_strategy_results = save_strategy_results
        self.max_pools = max_pools
        self.min_eth_threshold = min_eth_threshold

        # Initialize token processor
        self.live_token_processor = LiveBlockTokenProcessor(
            logger=self.logger,
            warmup_blocks=self.warmup_blocks,
            add_pnl_to_db=add_pnl_to_db
        )

        # Initialize strategy engines and position managers
        self.strategy_engines = {}
        for strategy_name, strategy in self.config.strategies.items():
            self.strategy_engines[strategy_name] = BacktestStrategyEngine(investment_strategy=strategy)
        
        self.strategy_position_managers = {
            strategy_name: StrategyPositionManager(strategy_engine=engine, logger=self.logger)
            for strategy_name, engine in self.strategy_engines.items()
        }

        # Initialize results writer if saving is enabled
        self.results_writer = None
        if self.save_strategy_results:
            self.results_writer = LiveResultsWriter(logger=self.logger)
            self.strategy_run_ids = {}

        # Initialize pool tracking components
        self.pool_level_extractor = TokenInfoExtractor(
            logger=self.logger,
            use_pool_cache=True,
            track_creators=True  # Enable creator tracking
        )
        
        self.pool_level_publisher = TokenInfoPublisher(
            pub_endpoint=zmq_pub_endpoint,
            rep_endpoint=zmq_rep_endpoint,
            logger=self.logger,
            min_eth_threshold=self.min_eth_threshold
        )
        
        # Link extractor to publisher for mempool data access
        self.pool_level_publisher.set_token_info_extractor(self.pool_level_extractor)

        # Register callback for pool updates
        self.pool_level_extractor.register_update_callback(
            'publisher',
            self._handle_pool_updates
        )

        self._is_shutting_down = False
        self._main_task = None
        self.updated_positions_by_strategy = {}

    async def start(self):
        """Start the live engine with pool tracking."""
        self.logger.info("Starting live engine with pool tracking")
        self._is_shutting_down = False

        # Start token processor
        self.logger.info("Starting token processor...")
        token_processor_task = asyncio.create_task(self.live_token_processor.start())
        
        # Start pool level publisher
        self.logger.info("Starting pool level publisher...")
        await self.pool_level_publisher.start()
        
        # Start main processing task
        self.logger.info("Starting main processing task...")
        self._main_task = asyncio.create_task(self._process_token_updates())
        
        self.logger.info("Live engine with pool tracking started successfully")

    async def stop(self):
        """Stop the engine and clean up resources."""
        if self._is_shutting_down:
            return
            
        self.logger.info("Stopping live engine with pool tracking")
        self._is_shutting_down = True
        
        # Stop token processor
        if self.live_token_processor:
            await self.live_token_processor.stop()
        
        # Stop pool level publisher
        if self.pool_level_publisher:
            await self.pool_level_publisher.stop()
        
        # Cancel main task
        if self._main_task and not self._main_task.done():
            self._main_task.cancel()
            try:
                await self._main_task
            except asyncio.CancelledError:
                pass
        
        self.logger.info("Live engine stopped")

    async def _process_token_updates(self):
        """Main loop to process token updates from confirmed blocks."""
        self.logger.info("Token update processing task started")
        
        while not self._is_shutting_down:
            try:
                # Wait for new updates
                await self.live_token_processor.new_updates_event.wait()
                
                if self._is_shutting_down:
                    break
                
                # Process all available updates
                while not self.live_token_processor.unprocessed_token_updates.empty():
                    try:
                        block_number, updated_tokens = await self.live_token_processor.unprocessed_token_updates.get()
                        
                        if not updated_tokens:
                            continue
                        
                        self.logger.info(f"Processing {len(updated_tokens)} token updates from block {block_number}")
                        
                        # Update strategies
                        await self._update_strategy_positions(updated_tokens)
                        
                        # Save results if enabled
                        if self.save_strategy_results:
                            await self._save_strategy_results_to_database(updated_tokens)
                        
                        # Extract and publish pool levels with creator info
                        updated_pools = await self.pool_level_extractor.update_token_info(
                            updated_tokens, block_number
                        )
                        
                        # Publish block sync data
                        if updated_pools:
                            # Get active creators for this block
                            active_creators = {}
                            for pool_data in updated_pools.values():
                                creator_addr = pool_data.get('creator_address')
                                if creator_addr:
                                    profile = self.pool_level_extractor.get_creator_profile(creator_addr)
                                    if profile:
                                        active_creators[creator_addr] = profile.to_dict()
                            
                            # Publish block synchronization
                            await self.pool_level_publisher.publish_block_sync(
                                block_number,
                                list(updated_pools.keys()),
                                active_creators
                            )
                        
                        # Mark task as done
                        self.live_token_processor.unprocessed_token_updates.task_done()
                        
                    except Exception as e:
                        self.logger.error(f"Error processing token update: {e}", exc_info=True)
                
                # Clear the event
                self.live_token_processor.new_updates_event.clear()
                
            except asyncio.CancelledError:
                break
            except Exception as e:
                self.logger.error(f"Error in token update processing: {e}", exc_info=True)
                await asyncio.sleep(1)  # Brief pause before retrying
        
        self.logger.info("Token update processing task stopped")

    async def _handle_pool_updates(self, updated_pools: Dict[str, Dict[str, Any]]):
        """Handle pool updates from the extractor."""
        # Publish updates via ZeroMQ
        await self.pool_level_publisher.update_token_info(updated_pools)

    async def _update_strategy_positions(self, updated_tokens: Dict[str, Any]):
        """Update strategy positions based on token updates."""
        for strategy_name, position_manager in self.strategy_position_managers.items():
            try:
                updated_positions = position_manager.update_positions(updated_tokens)
                self.updated_positions_by_strategy[strategy_name] = updated_positions
                
                if updated_positions:
                    self.logger.info(
                        f"Strategy {strategy_name} updated {len(updated_positions)} positions"
                    )
            except Exception as e:
                self.logger.error(
                    f"Error updating positions for strategy {strategy_name}: {e}",
                    exc_info=True
                )

    async def _save_strategy_results_to_database(self, updated_tokens: Dict[str, Any]):
        """Save strategy results to database."""
        if not self.results_writer or not self.updated_positions_by_strategy:
            return
        
        try:
            # Save run information if not already saved
            for strategy_name in self.strategy_position_managers:
                if strategy_name not in self.strategy_run_ids:
                    run_id = await self.results_writer.save_strategy_run(
                        strategy_name=strategy_name,
                        strategy_params=self.config.strategies[strategy_name].get_params(),
                        initial_balance=self.config.initial_balance
                    )
                    self.strategy_run_ids[strategy_name] = run_id
            
            # Save token positions
            await self.results_writer.save_token_positions(
                self.strategy_run_ids,
                self.updated_positions_by_strategy,
                updated_tokens
            )
            
        except Exception as e:
            self.logger.error(f"Error saving strategy results: {e}", exc_info=True)
