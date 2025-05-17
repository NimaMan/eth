"""
LiveBacktestEngineWithPools: Real-time Strategy Execution with Pool Level Tracking

**Objective:**
----------
Create a streamlined trading system that:
1. **Processes blockchain data** for real-time strategy execution
2. **Tracks token pool levels** for sharing with the Rust mempool processor
3. **Manages portfolio positions** based on trading strategies
4. **Communicates pool states** to enable scam detection in Rust

**Architecture Overview:**
---------------------
This system implements a hybrid Python-Rust architecture with separation of concerns:
- **Python Component (This Module)**: Processes confirmed blocks, extracts token data, 
  updates pool ETH reserve levels, and executes trading strategies
- **Rust Component**: Monitors mempool for pending transactions, simulates execution,
  and detects potential scam transactions that could drain liquidity pools

**Event Sequence:**
----------------
1. **Block Processing** (Python):
   a. New block arrives from Ethereum node
   b. LiveBlockTokenProcessor extracts token updates
   c. Updates are sent to the unprocessed_token_updates queue
   d. Main processing loop processes each update

2. **Token & Strategy Processing** (Python):
   a. For each token update, strategy positions are updated
   b. Position changes are tracked for database writes
   c. PnL calculations are performed and logged
   d. Strategy results are saved to the database

3. **Pool Level Extraction** (Python):
   a. ETH reserve levels are extracted from token updates
   b. PoolLevelExtractor maintains in-memory cache of latest pool states
   c. Updates trigger callbacks to registered listeners

4. **Pool Level Communication** (Python → Rust):
   a. PoolLevelPublisher receives pool updates via callback
   b. Updates are serialized to JSON format
   c. Data is published via ZeroMQ PUB/SUB socket
   d. REQ/REP socket handles on-demand queries from Rust

5. **Mempool Monitoring** (Rust):
   a. Rust monitors mempool for new transactions
   b. Transactions are simulated to calculate state changes
   c. Pool state changes are compared against thresholds
   d. Suspicious transactions that could drain pools are flagged

**Component Interaction:**
----------------------
```
┌─────────────────────────┐     Block Data       ┌─────────────────────┐
│                         │     with Tokens      │                     │
│  Ethereum Node          ├────────────────────► │  LiveBlockToken     │
│                         │                      │  Processor          │
└─────────────────────────┘                      └─────────┬───────────┘
                                                           │
                                                           │ Token Updates
                                                           ▼
┌─────────────────────────┐     Strategy         ┌─────────────────────┐
│                         │     Execution        │                     │
│  Strategy Position      │◄───────────────────► │  LiveBacktest       │
│  Manager                │                      │  Engine (This)      │
└─────────────────────────┘                      └─────────┬───────────┘
                                                           │
                                                           │ Pool Updates
                                                           ▼
┌─────────────────────────┐                     ┌─────────────────────┐
│                         │                     │                     │
│  PoolLevelExtractor     │◄───────────────────┤  Token Pool Data    │
│                         │                     │  Extraction         │
└─────────────┬───────────┘                     └─────────────────────┘
              │
              │ Pool ETH Levels
              ▼
┌─────────────────────────┐     ZeroMQ          ┌─────────────────────┐
│                         │     PUB/SUB         │                     │
│  PoolLevelPublisher     ├────────────────────►│  Rust Mempool       │
│  (Python)               │     REQ/REP         │  Processor          │
└─────────────────────────┘                     └─────────────────────┘
```

**Configuration Options:**
----------------------
- **warmup_blocks**: Number of blocks to process before starting strategy execution
- **save_strategy_results**: Whether to save strategy results to database
- **add_pnl_to_db**: Whether to write PnL data to database
- **zmq_pub_endpoint**: ZeroMQ publisher endpoint for pool updates
- **zmq_rep_endpoint**: ZeroMQ reply endpoint for pool state queries

**Key Components:**
---------------
- **LiveBlockTokenProcessor**: Processes new blocks and extracts token updates
- **StrategyPositionManager**: Manages and updates strategy positions
- **PoolLevelExtractor**: Extracts and maintains pool ETH reserve levels
- **PoolLevelPublisher**: Publishes pool levels to the Rust component

This version separates concerns:
- Python side focuses on confirmed block processing and pool level tracking
- Rust side handles mempool monitoring and scam detection
"""

import asyncio
from datetime import datetime

from eth_token.token_manager.live_block_token_processor import LiveBlockTokenProcessor
from eth_portfolio_manager.core.strategy_position_manager import StrategyPositionManager
from eth_portfolio_manager.backtesting.backtest_strategy_engine import BacktestStrategyEngine
from eth_portfolio_manager.db.live_token_position_results_writer import LiveResultsWriter
from eth_portfolio_manager.utils.logger import get_logger
from eth_portfolio_manager.pool_level.pool_level_extractor import PoolLevelExtractor
from eth_portfolio_manager.pool_level.pool_level_publisher import PoolLevelPublisher


class LiveBacktestEngineWithPools:
    """
    Live execution engine with pool level tracking.
    Handles strategy execution, results writing, and pool level extraction.
    """
    
    def __init__(
        self,
        config,
        logger=None,
        warmup_blocks=1000,
        save_strategy_results=True,
        add_pnl_to_db=True,
        zmq_pub_endpoint="tcp://*:5557",
        zmq_rep_endpoint="tcp://*:5558",
    ):
        """Initialize the live execution engine with pool tracking."""
        self.logger = logger or get_logger("portfolio_live_pools")
        self.config = config
        self.warmup_blocks = warmup_blocks
        self.save_strategy_results = save_strategy_results

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

        # Pool level tracking components
        self.pool_level_extractor = PoolLevelExtractor(logger=self.logger)
        self.pool_level_publisher = PoolLevelPublisher(
            pub_endpoint=zmq_pub_endpoint,
            rep_endpoint=zmq_rep_endpoint,
            logger=self.logger
        )

        # Runtime state
        self._is_shutting_down = False
        self.updated_positions_by_strategy = {}

        # Tasks
        self.token_processor_task = None
        self._main_token_processing_task = None

    async def start(self):
        """Start the live engine with pool tracking."""
        self.logger.info("Starting live engine with pool tracking")
        self._is_shutting_down = False

        # Start token processor
        self.logger.info("Starting token processor...")
        self.token_processor_task = asyncio.create_task(self.live_token_processor.start())
        self.logger.info("Token processor started")

        # Start the pool level publisher
        self.logger.info("Starting pool level publisher...")
        await self.pool_level_publisher.start()
        self.logger.info("Pool level publisher started")

        # Start the main token processing task
        self.logger.info("Starting main token processing task...")
        self._main_token_processing_task = asyncio.create_task(self._process_token_updates())
        self.logger.info("Main token processing task started")

    async def stop(self):
        """Stop all components of the engine."""
        self.logger.info("Stopping live engine...")
        self._is_shutting_down = True

        # Stop the pool level publisher
        try:
            await self.pool_level_publisher.stop()
            self.logger.info("Pool level publisher stopped.")
        except Exception as e:
            self.logger.error(f"Error stopping pool level publisher: {e}", exc_info=True)

        # Stop the token processor
        try:
            if hasattr(self, 'live_token_processor'):
                 await self.live_token_processor.stop()
                 self.logger.info("Live token processor stopped.")
        except Exception as e:
            self.logger.error(f"Error stopping live token processor: {e}", exc_info=True)

        # Cancel tasks
        tasks_to_cancel = [
            getattr(self, '_main_token_processing_task', None),
            getattr(self, 'token_processor_task', None)
        ]

        for task in tasks_to_cancel:
            if task and not task.done():
                task_name = task.get_name() if hasattr(task, 'get_name') else 'Unnamed Task'
                self.logger.debug(f"Cancelling task: {task_name}")
                task.cancel()
                try:
                    await task
                except asyncio.CancelledError:
                    self.logger.debug(f"Task {task_name} cancelled successfully.")
                except Exception as e:
                    self.logger.error(f"Error cancelling task {task_name} during shutdown: {e}", exc_info=True)

        # Final save of strategy results if enabled
        if self.save_strategy_results and self.results_writer:
            try:
                await self._save_strategy_results_to_database(None)
                self.logger.info("Final strategy results saved.")
            except Exception as e:
                 self.logger.error(f"Error saving final results: {e}", exc_info=True)

        self.logger.info("Live engine shutdown complete.")

    async def _process_token_updates(self):
        """Process token updates, execute strategies, save results, and update pool levels."""
        self.logger.info("Starting main token update processing loop")
        
        while not self._is_shutting_down:
            try:
                await self.live_token_processor.new_updates_event.wait()
                self.live_token_processor.new_updates_event.clear()

                while not self.live_token_processor.unprocessed_token_updates.empty():
                    try:
                        result = await self.live_token_processor.unprocessed_token_updates.get()
                        block_number, updated_tokens = result
                        
                        if updated_tokens:
                            # Process strategy updates
                            tasks = []
                            for strategy_name, position_manager in self.strategy_position_managers.items():
                                # Ensure the set exists and clear it for the current block's updates
                                if strategy_name not in self.updated_positions_by_strategy:
                                    self.updated_positions_by_strategy[strategy_name] = set()
                                else:
                                    self.updated_positions_by_strategy[strategy_name].clear()

                                tasks.append(self._update_strategy_positions(
                                    strategy_name,
                                    position_manager,
                                    updated_tokens,
                                    self.updated_positions_by_strategy
                                ))
                            await asyncio.gather(*tasks)
                            
                            # Update pool levels
                            updated_pools = await self.pool_level_extractor.update_pool_levels(
                                updated_tokens, block_number
                            )
                            
                            # Publish updated pool levels to Rust
                            await self.pool_level_publisher.update_pool_levels(updated_pools)
                            
                            # Save Strategy Results
                            if self.save_strategy_results and self.results_writer:
                                await self._save_strategy_results_to_database(block_number)
                                
                            # Add PnL writing if enabled
                            if self.live_token_processor.add_pnl_to_db:
                                updated_token_addresses = list(updated_tokens.keys())
                                await self._schedule_pnl_writes_for_tokens(updated_token_addresses, block_number)
                            
                        self.live_token_processor.unprocessed_token_updates.task_done()

                    except Exception as e:
                        self.logger.error(f"Error processing individual token update item: {e}", exc_info=True)
                        if not self.live_token_processor.unprocessed_token_updates.empty():
                             try:
                                 self.live_token_processor.unprocessed_token_updates.task_done()
                             except ValueError: 
                                 pass

            except asyncio.CancelledError:
                break
            except Exception as e:
                self.logger.error(f"Error in main token update processing loop: {e}", exc_info=True)
                await asyncio.sleep(1)

    async def _update_strategy_positions(self, strategy_name, position_manager, updated_tokens, updated_positions_by_strategy):
        """Update positions for a single strategy based on new token data."""
        try:
            updated_positions_dict = await position_manager.update_updated_tokens_positions(updated_tokens)

            if updated_positions_dict:
                for token_address, pool_positions in updated_positions_dict.items():
                    for pool_address, position in pool_positions.items():
                        if position:
                            composite_key = f"{token_address}-{pool_address}"
                            updated_positions_by_strategy[strategy_name].add(composite_key)

            return updated_positions_dict
        except Exception as e:
            self.logger.error(f"Error updating positions for strategy {strategy_name}: {e}", exc_info=True)
            return {}

    async def _save_strategy_results_to_database(self, block_number):
        """Save current strategy results to the database (if enabled)."""
        if not self.save_strategy_results or not self.results_writer:
            return

        try:
            if block_number is None:
                block_number = self.live_token_processor.latest_processed_block
            start_block = max(1, block_number - self.warmup_blocks) if self.warmup_blocks else block_number

            for strategy_name, strategy_position_manager in self.strategy_position_managers.items():
                try:
                    strategy_params = strategy_position_manager.strategy_engine.strategy_parameters

                    live_params = {
                        **strategy_params,
                        "mode": "LIVE",
                        "updated_at": datetime.now().isoformat(),
                        "current_block": block_number
                    }

                    live_strategy_name = f"LIVE_{strategy_name}"
                    run_id = self.results_writer.create_or_update_strategy_run(
                        strategy_name=live_strategy_name,
                        params=live_params,
                        start_block=start_block,
                        end_block=block_number
                    )

                    if strategy_name in self.updated_positions_by_strategy and self.updated_positions_by_strategy[strategy_name]:
                        updated_token_keys = self.updated_positions_by_strategy[strategy_name]
                        updated_positions = {
                            key: position for key, position in
                            strategy_position_manager.token_positions_cache.items()
                            if key in updated_token_keys
                        }
                        if updated_positions:
                            try:
                                self.results_writer.update_token_positions(run_id, updated_positions)
                            except Exception as e:
                                self.logger.error(f"Error writing position history for {live_strategy_name}: {e}", exc_info=True)
                except Exception as e:
                    self.logger.error(f"Error processing strategy {strategy_name} results for block {block_number}: {e}", exc_info=True)
        except Exception as e:
            self.logger.error(f"Error saving strategy results for block {block_number}: {e}", exc_info=True)

    async def _schedule_pnl_writes_for_tokens(self, updated_token_addresses, block_number):
        """Schedule PnL writes for tokens after strategy processing is complete."""
        for token_address in updated_token_addresses:
            try:    
                # Schedule PnL writing in a separate thread to avoid blocking
                await asyncio.to_thread(
                    self.live_token_processor.live_tokens_cache._write_token_pnl, 
                    token_address
                )
            except Exception as e:
                self.logger.error(f"Error scheduling PnL write for token {token_address}, block {block_number}: {e}")
                    
    def get_pool_level(self, pool_address):
        """Get the current ETH level for a specific pool."""
        return self.pool_level_extractor.get_pool_level(pool_address)
        
    def get_all_pool_levels(self):
        """Get all current pool ETH levels."""
        return self.pool_level_extractor.get_all_pool_levels()
        
    def get_pool_token(self, pool_address):
        """Get the token address associated with a pool."""
        pool_data = self.pool_level_extractor.get_pool_data(pool_address)
        return pool_data['token_address'] if pool_data else None 