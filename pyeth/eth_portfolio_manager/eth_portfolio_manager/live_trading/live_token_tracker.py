"""
LiveTokenTracker: Real-time Token Processing and Pool Level Tracking

Purpose:
--------
The LiveTokenTracker is the main entry point for processing real-time blockchain data,
extracting pool liquidity levels, and coordinating with trading strategies. It bridges
the gap between raw blockchain data and actionable trading signals while sharing
critical pool information with the Rust mempool processor.

Key Responsibilities:
--------------------
1. **Process Blockchain Data**: Consume new blocks and extract token events
2. **Track Pool Levels**: Monitor ETH reserves in liquidity pools
3. **Execute Strategies**: Run trading strategies on confirmed data
4. **Publish Pool Data**: Share pool states with Rust for scam detection

Architecture:
-------------
```
┌─────────────────────────┐     Block Data       ┌─────────────────────┐
│                         │     with Events      │                     │
│  Ethereum Node          ├────────────────────► │  LiveBlockToken     │
│  (via RPC)              │                      │  Processor          │
└─────────────────────────┘                      └─────────┬───────────┘
                                                           │
                                                           │ Token Updates
                                                           ▼
┌─────────────────────────┐                     ┌─────────────────────┐
│                         │     Trading         │                     │
│  LiveTradingAdapter     │◄───────────────────┤  LiveTokenTracker   │
│  or ResultsWriter       │     Results         │  (This Class)       │
└─────────────────────────┘                      └─────────┬───────────┘
                                                           │
                                                           │ Pool Updates
                                                           ▼
┌─────────────────────────┐                     ┌─────────────────────┐
│                         │                     │                     │
│  TokenUpdateCache       │◄───────────────────┤  Pool Reserve       │
│                         │                     │  Extraction         │
└─────────────┬───────────┘                     └─────────────────────┘
              │
              │ ETH Reserves
              ▼
┌─────────────────────────┐     ZeroMQ          ┌─────────────────────┐
│                         │     PUB (5557)      │                     │
│  TokenUpdateNotifier    ├────────────────────►│  Rust Mempool       │
│                         │                     │  Processor          │
└─────────────────────────┘                     └─────────────────────┘
```

Event Flow:
-----------
1. **Block Reception**: New blocks arrive from Ethereum node
2. **Event Extraction**: LiveBlockTokenProcessor extracts token transfers and pool events
3. **Queue Processing**: Token updates are queued for processing
4. **Strategy Execution**: Updates trigger strategy evaluation
5. **Update Tracking**: TokenUpdateCache records which tokens changed per block
6. **Data Publishing**: Pool levels published to Rust via ZMQ

Integration Points:
------------------
- **Input**: Ethereum node RPC (blocks and events)
- **Output**: Trading results (via adapter/writer) and pool levels (via ZMQ)
- **Strategies**: Pluggable strategy system for different trading approaches
- **Database**: Results persistence through writer interface

Configuration:
--------------
- **warmup_blocks**: Blocks to process before trading begins
- **save_strategy_results**: Enable database persistence
- **add_pnl_to_db**: Track profit/loss in database
- **min_eth_threshold**: Minimum pool liquidity to track

Usage:
------
```python
# Create tracker with trading adapter
tracker = LiveTokenTracker(
    config=strategy_config,
    warmup_blocks=1000,
    save_strategy_results=True
)

# Set up results writer (adapter or legacy)
tracker.results_writer = LiveTradingAdapter(...)

# Start processing
await tracker.start()
```

This class coordinates the entire Python-side live trading pipeline, from
blockchain data ingestion to pool level publishing for Rust-based scam detection.
"""

import asyncio
from datetime import datetime

from eth_token.token_manager.live_block_token_processor import LiveBlockTokenProcessor
from eth_portfolio_manager.core.strategy_position_manager import StrategyPositionManager
from eth_portfolio_manager.live_trading.live_strategy_engine import LiveStrategyEngine
from eth_portfolio_manager.notifications.trade_signal_publisher import TradeSignalPublisher
from eth_data.database.writers.live_token_position_results_writer import LiveResultsWriter
from eth_portfolio_manager.utils.logger import get_logger
from eth_portfolio_manager.notifications.token_update_cache import TokenUpdateCache
from eth_portfolio_manager.notifications.token_update_notifier import TokenUpdateNotifier
from eth_data.database.writers.token_status_writer import TokenStatusWriter


class LiveTokenTracker:
    """
    Live execution engine with pool level tracking.
    
    Handles strategy execution using LiveStrategyEngine, results writing, 
    and pool level extraction. Can optionally publish trading signals to 
    eth_kartal when a signal publisher is configured.
    
    This class serves as a bridge between the older backtest-style architecture
    and the newer live trading system, allowing gradual migration.
    """
    
    def __init__(
        self,
        config,
        logger=None,
        warmup_blocks=1000,
        save_strategy_results=True,
        add_pnl_to_db=True,
        add_status_to_db=False,
        min_eth_threshold=0.01,
    ):
        """Initialize the live execution engine with pool tracking.
        
        Args:
            config: Configuration object with strategies
            logger: Optional logger instance
            warmup_blocks: Number of blocks to process before starting strategy
            save_strategy_results: Whether to save results to database
            add_pnl_to_db: Whether to add PnL data to database
            min_eth_threshold: Minimum ETH reserve to keep pool when at capacity (default 0.01)
            
        Note:
            ZMQ endpoints are hardcoded to ensure consistent communication:
            - Publisher endpoint: tcp://*:5557
            - Full token state and startup discovery are read from Redis snapshots
        """
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
        self.add_pnl_to_db = add_pnl_to_db
        self.add_status_to_db = add_status_to_db

        # Initialize signal publisher (optional - can be None for compatibility)
        self.signal_publisher = None
        
        # Initialize strategy engines and position managers
        self.strategy_engines = {}
        for strategy_name, strategy in self.config.strategies.items():
            # Use LiveStrategyEngine for live trading
            self.strategy_engines[strategy_name] = LiveStrategyEngine(
                investment_strategy=strategy,
                signal_publisher=self.signal_publisher
            )
        self.strategy_position_managers = {
            strategy_name: StrategyPositionManager(strategy_engine=engine, logger=self.logger)
            for strategy_name, engine in self.strategy_engines.items()
        }

        # Initialize results writer if saving is enabled
        self.results_writer = None
        if self.save_strategy_results:
            self.results_writer = LiveResultsWriter(logger=self.logger)
            self.strategy_run_ids = {}

        # Token info tracking components
        self.token_update_cache = TokenUpdateCache(logger=self.logger)
        self.token_update_notifier = TokenUpdateNotifier(
            pub_endpoint="tcp://*:5557",
            logger=self.logger,
        )
        # Connect cache to publisher
        self.token_update_notifier.set_cache(self.token_update_cache)

        # Token status persistence
        self.token_status_writer = TokenStatusWriter(logger=self.logger)

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

        # Start the token tracking publisher
        self.logger.info("Starting token update notifier...")
        await self.token_update_notifier.start()
        self.logger.info("Token update notifier started")
        
        # Publish initial state
        await self.token_update_notifier.publish_all()

        # Start the main token processing task
        self.logger.info("Starting main token processing task...")
        self._main_token_processing_task = asyncio.create_task(self._process_token_updates())
        self.logger.info("Main token processing task started")

        # Watch core tasks and stop engine if any exits unexpectedly
        async def _watch_core_tasks():
            tasks = [t for t in [self.token_processor_task, self._main_token_processing_task] if t]
            if not tasks:
                return
            done, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
            for t in done:
                try:
                    exc = t.exception()
                except asyncio.CancelledError:
                    self.logger.info("A core task was cancelled (as part of shutdown).")
                    return
                except Exception as e:
                    self.logger.error(f"Error retrieving exception from core task: {e}", exc_info=True)
                    exc = None
                if exc:
                    self.logger.error(f"A core task exited with error: {exc}", exc_info=True)
                else:
                    self.logger.warning("A core task exited unexpectedly without error.")
            # Initiate shutdown of the engine so callers see shutdown logs
            await self.stop()

        asyncio.create_task(_watch_core_tasks())

    async def stop(self):
        """Stop all components of the engine."""
        self.logger.info("Stopping live engine...")
        self._is_shutting_down = True

        # Stop the token info publisher
        try:
            await self.token_update_notifier.stop()
            self.logger.info("Token update notifier stopped.")
        except Exception as e:
            self.logger.error(f"Error stopping token tracking publisher: {e}", exc_info=True)

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
                        block_number, updated_tokens = result[:2]
                        
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
                            
                            # Update token tracking cache
                            await self.token_update_cache.update_from_token_objects(
                                updated_tokens, block_number
                            )
                            
                            # Persist token and pool status to database (controlled via flag)
                            if self.add_status_to_db:
                                await self._persist_token_and_pool_status(updated_tokens)
                            
                            # Publish updated token info to Rust
                            await self.token_update_notifier.publish_block_updates(block_number)
                            
                            # Get and publish wallet positions
                            wallet_positions = self.get_wallet_positions()
                            if wallet_positions:
                                # TODO: Implement wallet position publishing if needed
                                # The token_update_notifier only broadcasts token addresses currently
                                pass
                            
                            # Save Strategy Results
                            if self.save_strategy_results and self.results_writer:
                                await self._save_strategy_results_to_database(block_number)
                                
                            # Add PnL writing if enabled (can be disabled via add_pnl_to_db)
                            if self.live_token_processor.add_pnl_to_db:
                                updated_token_addresses = list(updated_tokens.keys())
                                # self.logger.debug(f"Scheduling PnL writes for {len(updated_token_addresses)} tokens")
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
        # Respect global setting; skip if PnL writing is disabled
        if not (
            getattr(self.live_token_processor, 'add_pnl_to_db', False)
            and getattr(self.live_token_processor.live_tokens_cache, 'pnl_writer', None)
        ):
            return
        for token_address in updated_token_addresses:
            try:    
                # Schedule PnL writing in a separate thread to avoid blocking
                await asyncio.to_thread(
                    self.live_token_processor.live_tokens_cache._write_token_pnl, 
                    token_address
                )
            except Exception as e:
                self.logger.error(f"Error scheduling PnL write for token {token_address}, block {block_number}: {e}")
                    
    def get_wallet_positions(self):
        """Get position information from wallet tracker strategy."""
        positions = {}
        for strategy_name, manager in self.strategy_position_managers.items():
            strategy = manager.strategy_engine.investment_strategy
            if hasattr(strategy, 'get_position_info'):
                positions[strategy_name] = strategy.get_position_info()
        return positions
    
    async def _persist_token_and_pool_status(self, updated_tokens):
        """Persist token and pool status information to the database."""
        if not updated_tokens:
            return
        
        # self.logger.debug(f"Persisting {len(updated_tokens)} tokens to database")
        
        for token_address, token in updated_tokens.items():
            try:
                # DEBUG: Log what's in the token data
                # self.logger.debug(f"Token {token_address} data: creation_tx={getattr(token, 'creation_tx', 'NOT_FOUND')}, trading_enabled_tx={getattr(token, 'trading_enabled_tx', 'NOT_FOUND')}")
                
                # Prepare token data for persistence
                token_data = {
                    "contract_address": token.contract_address,
                    "creator_address": token.creator_address,
                    "is_scam": token.is_scam,
                    "scam_label": token.scam_label,
                    "creation_tx": token.creation_tx,
                    "trading_enabled_tx": token.trading_enabled_tx
                }
                
                # Prepare pools data
                pools_data = {}
                pool_addresses = token.pool_addresses or []
                pool_info_dict = token.get_pool_info_dict()
                
                # Get pool health stats for scam information
                pool_health_stats = {}
                pool_health_stats = token.pool_manager.get_pool_health_stats()
                
                for pool_address in pool_addresses:
                    if pool_address in pool_info_dict:
                        pool_info = pool_info_dict[pool_address]
                        denom_reserve = token.get_pool_reserve(pool_address)
                        token_reserve = token.get_pool_token_reserve(pool_address)
                        
                        # Get scam info from health stats
                        pool_health = pool_health_stats.get(pool_address, {})
                        
                        # Get pool object for trading_enabled info
                        pool_obj = None
                        if token.pool_manager:
                            pool_obj = token.pool_manager.get_pool(pool_address)
                        
                        if denom_reserve is not None and token_reserve is not None:
                            pools_data[pool_address] = {
                                "pool_type": pool_info['pool_type'],
                                "denom_address": pool_info['denom_address'],
                                "denom_reserve": float(denom_reserve),
                                "token_reserve": float(token_reserve),
                                "pool_id": pool_info.get('pool_id'),
                                "fee_tier": pool_info.get('fee_tier'),
                                "is_scam": pool_health.get('is_scam', False),
                                "scam_label": pool_health.get('scam_label'),
                                "scam_block": pool_health.get('scam_block'),
                                "scam_tx_hash": pool_health.get('scam_tx_hash'),
                                # Add per-pool trading_enabled fields
                                "trading_enabled": pool_obj.trading_enabled if pool_obj else False,
                                "trading_enabled_block": pool_obj.trading_enabled_block if pool_obj else None,
                                "trading_enabled_tx": pool_obj.trading_enabled_tx if pool_obj else None
                            }
                
                # Persist token and pools to database
                success = self.token_status_writer.create_or_update_token_with_pools(
                    token_data, pools_data
                )
                
                if not success:
                    self.logger.error(f"Failed to persist token {token_address} and pools to database")
                    
            except Exception as e:
                self.logger.error(f"Error persisting token {token_address} status: {e}")
    
    def set_signal_publisher(self, signal_publisher: TradeSignalPublisher):
        """Set the signal publisher for all strategy engines.
        
        This allows LiveTokenTracker to send trading signals to eth_kartal
        when used in a live trading context.
        
        Args:
            signal_publisher: The TradeSignalPublisher instance to use
        """
        self.signal_publisher = signal_publisher
        
        # Update all existing strategy engines with the new publisher
        for engine in self.strategy_engines.values():
            engine.signal_publisher = signal_publisher 
