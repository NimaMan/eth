"""
LiveTokenManager: Manages real-time token data processing and tracking

Objective:
---------
1. Process New Blocks and track live tokens and their data
2. Provide access to current token states

Architecture & Flow:
------------------

Message Flow:
-----------
BlockProcessor -> RabbitMQ -> BlockSubscriber -> Queue -> BlockLiveTokenProcessor -> Token Events
                                                         |                     |
                                AlertSubscriber <- RabbitMQ <- AlertProcessor  |
                                                                               v
                                                                          RabbitMQ -> Token Event Consumers

Events:
- Wait for new blocks to be processed
- Get the updated tokens from the block processor
- Check for alerts and publish them

"""

import asyncio

from eth_tokens_live.token_manager.block_live_token_processor import BlockLiveTokenProcessor
from eth_tokens_live.token_manager.live_token_alert_processor import AlertProcessor
from eth_tokens_live.utils.logger import get_logger


class LiveTokenManager:
    def __init__(self, rabbitmq_url: str = None, logger=None):
        if logger is None:
            self.logger = get_logger(name="token_manager", log_folder="tokens_live")
        else:
            self.logger = logger
        
        # Initialize processors
        self.token_processor = BlockLiveTokenProcessor(rabbitmq_url=rabbitmq_url, logger=self.logger)
        self.alert_processor = AlertProcessor(rabbitmq_url=rabbitmq_url)
        
        # Track processed blocks for alerts
        self.last_alert_block = 0
        self._shutdown_event = asyncio.Event()
        self._monitor_task = None
        self._is_shutting_down = False  # Add shutdown flag
        
    async def start(self):
        """Start token and alert processing"""
        try:
            # Start block processing
            await self.token_processor.block_subscriber.start()
            
            # Start alert monitoring
            self._monitor_task = asyncio.create_task(self._monitor_token_updates())
            
            self.logger.info("Token manager started successfully")
            
            # Keep running until shutdown event is set
            while not self._shutdown_event.is_set():
                await asyncio.sleep(1)
                
        except KeyboardInterrupt:
            self.logger.info("Received keyboard interrupt...")
            await self.stop()
        except Exception as e:
            self.logger.error(f"Error in token manager: {e}")
            await self.stop()
            raise
            
    async def stop(self):
        """Stop all processing"""
        if self._is_shutting_down:
            return
            
        self._is_shutting_down = True
        self._shutdown_event.set()
        
        # Cancel monitoring task if running
        if self._monitor_task and not self._monitor_task.done():
            self._monitor_task.cancel()
            try:
                await self._monitor_task
            except asyncio.CancelledError:
                pass
        
        # Stop block subscriber
        if hasattr(self.token_processor, 'block_subscriber'):
            await self.token_processor.block_subscriber.stop()
            
        self.logger.info("Token manager stopped")

    async def _monitor_token_updates(self):
        """Monitor for token updates using event notification"""
        while not self._shutdown_event.is_set():
            try:
                # Wait for block processing to complete
                await self.token_processor.block_processed_event.wait()
                
                # Process alerts for updated tokens
                if self.token_processor.updated_tokens:
                    current_block = self.token_processor.latest_processed_block
                    await self.alert_processor.process_block_updates(
                        block_number=current_block,
                        updated_tokens=self.token_processor.updated_tokens
                    )
                    self.last_alert_block = current_block
                
                # Clear the event for next block
                self.token_processor.block_processed_event.clear()
                
            except Exception as e:
                self.logger.error(f"Error processing alerts: {e}")