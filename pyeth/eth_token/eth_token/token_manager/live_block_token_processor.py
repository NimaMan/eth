"""
LiveBlockTokenProcessor: Manages real-time token data processing and tracking with warm-up phase

Objective:
---------
1. Process new blocks and track live tokens and their data
2. Generate token update events for downstream consumers
3. Provide access to current token states

Architecture & Flow:
------------------
1. Warm-up Phase:
   - HistoricalBlockTokenProcessor processes historical blocks
   - Builds initial token state in LiveTokensCache
   - Ensures smooth transition to live processing 

2. Live Processing Phase:
   - Continues from last warm-up block
   - Subscribes to new blocks in real-time
   - Maintains token states and generates token update events

Event Flow:
-----------
1. Block Processing:
   - LiveBlockSnapshotSubscriber receives new block -> process_block_live()
   - process_block_live() processes block -> sets block_processed_event

2. Token Update Monitoring:
   - _monitor_token_updates() waits for block_processed_event
   - When triggered, extracts updated_tokens from processed block
   - Places updates in unprocessed_token_updates queue
   - Sets new_updates_event to notify consumers

3. Consumer Processing:
   - External systems wait on new_updates_event
   - When triggered, consume updates from unprocessed_token_updates queue
   - Process token updates through their own pipelines (e.g., strategy execution)

Historical (Warm-up) -> Live Transition:
----------------------------------------
- HistoricalBlockTokenProcessor processes blocks for warm-up
- Upon completion, LiveBlockTokenProcessor begins real-time processing
- Token state maintained consistently across transition
"""

import asyncio
from typing import Any, Dict
from eth_data.live_data_registry import LiveDataPublisher
from eth_token.erc20_token.token_snapshot import build_token_snapshot_map
from eth_token.token_manager.redis_block_subscriber import RedisBlockSubscriber
from eth_token.token_manager.block_token_processor import BlockTokenProcessor
from eth_token.token_manager.block_token_processor import HistoricalBlockTokenProcessor
from eth_token.utils.logger import get_logger


class LiveBlockTokenProcessor(BlockTokenProcessor):
    def __init__(self,
                 warmup_blocks: int = 1000,
                 logger=None,
                 add_pnl_to_db: bool = False):
        super().__init__(logger=logger, add_pnl_to_db=add_pnl_to_db)  
        # Initialize subscriber with our callback and block_token_processor
        self.block_subscriber = RedisBlockSubscriber(
            callback=self.process_block_live,
            logger=self.logger,
        )

        self.block_range_token_processor = HistoricalBlockTokenProcessor(
            block_token_processor=self,
            logger=self.logger
        )
        self.warmup_blocks = warmup_blocks

        self.unprocessed_token_updates = asyncio.PriorityQueue(maxsize=100)  # Queue for unprocessed token 
        self.block_processed_event = asyncio.Event() # Event to signal block processed is complete
        self.new_updates_event = asyncio.Event()  # Event to signal new updates is available
        self._shutdown_event = asyncio.Event() # Event to signal shutdown
        self._is_shutting_down = False
        self._monitor_task = None
        self._watcher_task = None
        self.token_snapshot_publisher = LiveDataPublisher()

    async def _on_task_done(self, name: str, task: asyncio.Task):
        """Handle unexpected background task completion by logging and initiating shutdown."""
        try:
            exc = task.exception()
        except asyncio.CancelledError:
            self.logger.info(f"{name} task cancelled.")
            return
        except Exception as e:
            self.logger.error(f"Error retrieving exception from {name} task: {e}", exc_info=True)
            exc = None

        if exc:
            self.logger.error(f"{name} task exited with error: {exc}", exc_info=True)
        else:
            self.logger.warning(f"{name} task exited unexpectedly without error.")

        # Trigger processor shutdown
        try:
            await self.stop()
        except Exception as e:
            self.logger.error(f"Error during shutdown after {name} task exit: {e}", exc_info=True)

    async def process_block_live(self, processed_block_result):
        """Process incoming blocks"""
        if self._is_shutting_down:
            return
        try:
            self.latest_processed_block = self.process_block_tokens(
                processed_block_result,
                block_number=processed_block_result["block_number"],
            )
            self.block_processed_event.set() # Signal block processed            
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing live block: {e}", exc_info=True)
    
    async def _schedule_pnl_writes_for_updated_tokens(self, current_block: int):
        """Schedules PnL writes for tokens updated in the current block."""
        if not (self.add_pnl_to_db and self.live_tokens_cache.pnl_writer):
            return # PnL writing disabled or writer not available
        pnl_tasks = []
        updated_addresses = list(self.updated_tokens.keys())
        for token_address in updated_addresses:
            pnl_tasks.append(
                asyncio.to_thread(
                    self.live_tokens_cache._write_token_pnl,
                    token_address
                )
            )

        if pnl_tasks:
            try:
                await asyncio.gather(*pnl_tasks, return_exceptions=True)
            except Exception as gather_err:
                  self.logger.error(f"Error gathering PnL write tasks via cache for block {current_block}: {gather_err}", exc_info=True)

    async def _publish_token_snapshots(self, block_number: int, updated_tokens: dict) -> Dict[str, Dict[str, Any]]:
        """
        Serialize updated tokens and persist their snapshots to Redis.

        Returns a dict mapping token_address -> serialized snapshot for reuse
        by downstream publishers (so we don't re-serialize the same data twice).
        """
        if not updated_tokens:
            return {}

        snapshot_payloads = build_token_snapshot_map(
            updated_tokens,
            on_error=lambda addr, exc: self.logger.error(
                "Failed to serialize token snapshot for %s at block %s: %s",
                addr,
                block_number,
                exc,
                exc_info=True,
            ),
        )

        async def _write_snapshot(token_address, snapshot):
            try:
                await self.token_snapshot_publisher.publish_token(token_address, snapshot)
            except Exception as exc:
                self.logger.error(
                    "Failed to publish token snapshot for %s at block %s: %s",
                    token_address,
                    block_number,
                    exc,
                    exc_info=True,
                )

        await asyncio.gather(
            *[_write_snapshot(addr, snapshot) for addr, snapshot in snapshot_payloads.items()],
            return_exceptions=True,
        )
        return snapshot_payloads

    async def _monitor_token_updates(self):
        """Monitor for token updates using event notification"""
        while not self._is_shutting_down:
            try:
                # Wait for block processing to complete
                await self.block_processed_event.wait()
                current_block = self.latest_processed_block
                # Log after a block has finished processing to avoid confusion with pre-processing
                self.logger.info(f"Processed block {current_block} ({len(self.updated_tokens)} tokens)")
                if self.updated_tokens:
                    updated_snapshot_tokens = dict(self.updated_tokens)
                    await self._publish_token_snapshots(
                        current_block, updated_snapshot_tokens
                    )
                    await self.unprocessed_token_updates.put(
                        (current_block, updated_snapshot_tokens)
                    )
                    self.new_updates_event.set()
                # Clear the event for next block
                self.block_processed_event.clear()
            except asyncio.CancelledError:
                 self.logger.info("Token update monitoring task cancelled.")
                 break
            except Exception as e:
                self.logger.error(f"Error processing token updates: {e}", exc_info=True)
                await asyncio.sleep(0.1)

    async def start(self):
        """Start processing live blocks"""
        try:
            self.logger.info("Starting block subscriber")
            # 1. Process historical blocks until we catch up
            if self.warmup_blocks:
                self.logger.info(f"Starting historical processing for {self.warmup_blocks} blocks")
                try:
                    latest_processed_block = await self.block_range_token_processor.process_range_until_live(block_range=self.warmup_blocks)
                    self.logger.info(f"Historical processing complete. Processed up to block {latest_processed_block}")
                except Exception as e:
                    self.logger.error(f"Error during historical processing: {e}")
                    raise

            # Switch into live mode for subsequent block processing
            self.is_live_mode = True

            # 2. Subscribe to live processed blocks (listener keeps running internally)
            await self.block_subscriber.start()
            
            # 3. Start monitoring for updates
            self._monitor_task = asyncio.create_task(self._monitor_token_updates())
            self.logger.info("Token update monitoring started")
            # Watch for unexpected termination of monitoring task
            self._monitor_task.add_done_callback(
                lambda t: asyncio.create_task(self._on_task_done("TokenUpdateMonitor", t))
            )
            
            # Keep running until shutdown
            while not self._shutdown_event.is_set():
                await asyncio.sleep(1)
            
        except Exception as e:
            self.logger.error(f"Error starting live processor: {e}")
            try:
                await self.stop()
            except Exception:
                pass
            raise
        finally:
            # Ensure we shutdown cleanly whenever start() exits
            if not self._is_shutting_down:
                try:
                    await self.stop()
                except Exception:
                    pass

    async def stop(self):
        """Stop processing live blocks"""
        if self._is_shutting_down:
            return
            
        self._is_shutting_down = True
        self.logger.info("Stopping live block processor...")
        
        try:
            # Stop the subscriber first
            await self.block_subscriber.stop()
            self.logger.info("Live block processor stopped successfully")

            # Stop monitoring task if running
            if self._monitor_task and not self._monitor_task.done():
                self.logger.info("Stopping monitor task...")
                self._monitor_task.cancel()
                try:
                    await self._monitor_task
                except asyncio.CancelledError:
                    pass

            self.logger.info("LiveBlockTokenProcessor stopped successfully")
            self.new_updates_event.clear()
            # Clear queue
            while not self.unprocessed_token_updates.empty():
                try:
                    item = self.unprocessed_token_updates.get_nowait()
                    self.unprocessed_token_updates.task_done()
                    self.logger.debug(f"Unprocessed token updates: {item}")
                except asyncio.QueueEmpty:
                    break
            self._shutdown_event.set()
            self.logger.info("LiveBlockTokenProcessor shutdown complete")
        except Exception as e:
            self.logger.error(f"Error during LiveBlockTokenProcessor shutdown: {e}")
            raise
