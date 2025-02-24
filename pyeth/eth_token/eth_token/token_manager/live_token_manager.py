"""
LiveTokenManager: Manages real-time token data processing and tracking

Objective:
---------
1. Process new blocks and track live tokens and their data
2. Provide access to current token states
3. Generate and publish alerts for token events

Architecture & Flow:
------------------
1. Warm-up Phase:
   - BlockRangeTokenProcessor processes historical blocks
   - Builds initial token state in LiveTokenObjectsCache of the live token processor using the block token processor
   - Processes alerts for historical tokens
   - Ensures smooth transition to live processing

2. Live Processing Phase:
   - Continues from last warm-up block
   - Processes new blocks in real-time
   - Maintains token states and generates alerts

Message Flow:
-----------
Historical (Warm-up):
BlockRangeTokenProcessor -> LiveTokenObjectsCache -> AlertProcessor -> RabbitMQ
                                                                          |
Live Processing:                                                         v
BlockProcessor -> RabbitMQ -> BlockSubscriber -> Queue -> BlockLiveTokenProcessor -> Token Events
                                                         |                     |
                                AlertSubscriber <- RabbitMQ <- AlertProcessor  |
                                                                              v
                                                                         RabbitMQ -> Token Event Consumers

Components:
----------
- BlockRangeTokenProcessor: Processes historical blocks for warm-up
- BlockLiveTokenProcessor: Processes real-time blocks
- UpdateLiveTokenAlertProcessor: Generates alerts for both phases
- TokenManager: Orchestrates warm-up and live processing phases

Events:
------
- Process historical blocks during warm-up
- Wait for new blocks to be processed
- Get the updated tokens from block processors
- Check for alerts and publish them
"""

import asyncio
from web3 import Web3
from eth_token.token_manager.block_token_processor import BlockRangeTokenProcessor, BlockTokenProcessor
from eth_token.token_manager.live_block_token_processor import LiveBlockTokenProcessor
from eth_token.alert.token_alert_processor import UpdateLiveTokenAlertProcessor
from eth_token.utils.logger import get_logger


class LiveTokenManager:
    def __init__(self, logger=None, warmup_blocks: int = 1000):
        self.logger = logger or get_logger(name="token_manager", log_folder="token_manager")
        self.web3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        self.warmup_blocks = warmup_blocks
        
        self.block_token_processor = BlockTokenProcessor(logger=self.logger)
        # Initialize processors
        
        self.block_range_token_processor = BlockRangeTokenProcessor(
            block_token_processor=self.block_token_processor,
            logger=self.logger
        )
        
        self.live_token_processor = LiveBlockTokenProcessor(
            block_token_processor=self.block_token_processor,
            logger=self.logger
        )

        self.alert_processor = UpdateLiveTokenAlertProcessor(logger=self.logger)
        
        # Track processed blocks for alerts
        self.last_alert_block = 0
        self._shutdown_event = asyncio.Event()
        self._monitor_task = None
        self._is_shutting_down = False
        self.unprocessed_updates = asyncio.PriorityQueue(maxsize=100)  # Queue for updates
        self.new_updates_event = asyncio.Event()  # Event to signal new updates
    
    async def _monitor_token_updates(self):
        """Monitor for token updates using event notification"""
        while not self._shutdown_event.is_set():
            try:
                # Wait for block processing to complete
                await self.live_token_processor.block_processed_event.wait()
                self.logger.info(f"Updated {len(self.live_token_processor.updated_tokens)} tokens in block {self.live_token_processor.latest_processed_block}")
                # Process alerts for updated tokens
                if self.live_token_processor.updated_tokens:
                    current_block = self.live_token_processor.latest_processed_block
                    await self.unprocessed_updates.put((current_block, self.live_token_processor.updated_tokens))
                    self.new_updates_event.set()
                    await self.alert_processor.process_block_token_updates(
                        block_number=current_block,
                        updated_tokens=self.live_token_processor.updated_tokens
                    )
                    self.last_alert_block = current_block
                
                # Clear the event for next block
                self.live_token_processor.block_processed_event.clear()
            
            except Exception as e:
                self.logger.error(f"Error processing alerts: {e}")
                
    async def start(self):
        """Start token and alert processing"""
        try:
            self.logger.info(f"Starting LiveTokenManager")
            
            # 1. Process historical blocks until we catch up
            if self.warmup_blocks:
                self.logger.info(f"Starting historical processing for {self.warmup_blocks} blocks")
                try:
                    latest_processed_block = await self.block_range_token_processor.process_range_until_live(
                        block_range=self.warmup_blocks
                    )
                    self.logger.info(f"Historical processing complete. Processed up to block {latest_processed_block}")
                    self.last_alert_block = latest_processed_block
                except Exception as e:
                    self.logger.error(f"Error during historical processing: {e}")
                    raise
                
            # 2. Start live processing
            self.logger.info(f"Starting live processing from block {self.live_token_processor.latest_processed_block}")
            try:
                await self.live_token_processor.start()
            except Exception as e:
                self.logger.error(f"Error starting live processor: {e}")
                raise
            
            # 3. Start monitoring for updates
            self._monitor_task = asyncio.create_task(self._monitor_token_updates())
            self.logger.info("Token update monitoring started")
            
            # Keep running until shutdown
            while not self._shutdown_event.is_set():
                await asyncio.sleep(1)
                
        except Exception as e:
            self.logger.error(f"Error in token manager: {e}")
            await self.stop()
            raise

    async def stop(self):
        """Stop the token manager and cleanup resources"""
        if self._shutdown_event.is_set():
            return
            
        try:
            self.logger.info("=== Stopping LiveTokenManager ===")
            self._shutdown_event.set()
            
            # Stop live processor first (includes block subscriber)
            if self.live_token_processor:
                self.logger.info("Stopping live token processor...")
                await self.live_token_processor.stop()
            
            # Stop monitoring task if running
            if self._monitor_task and not self._monitor_task.done():
                self.logger.info("Stopping monitor task...")
                self._monitor_task.cancel()
                try:
                    await self._monitor_task
                except asyncio.CancelledError:
                    pass
            
            # Clear events
            self.new_updates_event.clear()
            
            # Clear queue
            while not self.unprocessed_updates.empty():
                try:
                    await self.unprocessed_updates.get_nowait()
                    self.unprocessed_updates.task_done()
                except asyncio.QueueEmpty:
                    break
            
            self.logger.info("LiveTokenManager stopped successfully")
            
        except Exception as e:
            self.logger.error(f"Error during LiveTokenManager shutdown: {e}")
            raise
