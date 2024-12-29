"""
LiveBlockTokenProcessor: Processes Live blocks to maintain live token states

Objective:
---------
1. Subscribe to processed blocks from RabbitMQ
2. Handle warm-up phase by queuing blocks without processing
3. Maintain consistent token state between warm-up and live phases
4. Provide access to current token states

Architecture & Flow:
------------------
1. Initialization & State Management:
   - Uses shared BlockTokenProcessor for token state management
   - Maintains warm-up state flag
   - Tracks processed blocks and events
   - Coordinates with BlockRangeTokenProcessor during warm-up

2. Block Subscription (via RabbitMQ):
   - Connects to 'blocks_exchange' with routing_key='blocks'
   - Uses BlockSubscriber for block queuing and ordering
   - Queues blocks during warm-up phase
   - Processes queued blocks after warm-up completes

3. Processing Phases:
   a. Warm-up Phase:
      - Blocks are queued but not processed
      - BlockRangeTokenProcessor handles historical blocks
      - Maintains queue order for later processing
   
   b. Live Phase:
      - Processes queued blocks from warm-up
      - Continues with real-time block processing
      - Updates shared token state

4. Token State Coordination:
   - Uses shared BlockTokenProcessor for consistency
   - Ensures no duplicate processing between phases
   - Maintains proper block ordering
   - Signals block processing completion for alerts

Message Flow:
-----------
Warm-up Phase:
BlockProcessor -> RabbitMQ -> BlockSubscriber -> Queue (blocks stored)
                                                         |
Historical Processing:                                   |
BlockRangeTokenProcessor -> Shared BlockTokenProcessor   |
                                                        v
Live Phase:                                    Process Queued Blocks
BlockSubscriber -> Queue -> BlockLiveTokenProcessor -> Token Events
                                      |
                                      v
                            Shared BlockTokenProcessor

Components:
----------
1. BlockSubscriber:
   - Handles block queuing and ordering
   - Maintains queue during warm-up
   - Ensures no blocks are missed

2. BlockTokenProcessor (shared):
   - Core token processing logic
   - Used by both live and historical processing
   - Maintains consistent token state

3. Event Management:
   - Tracks block processing completion
   - Signals for alert generation
   - Coordinates phase transitions

State Transitions:
---------------
1. Warm-up -> Live:
   - Complete historical processing
   - Signal warm-up completion
   - Process queued blocks
   - Continue with live processing
"""

import asyncio
from typing import Dict, List
from eth_tokens_live.subscribers.block_subscriber import BlockSubscriber
from eth_tokens_live.token_manager.block_token_processor import BlockTokenProcessor
from eth_tokens_live.utils.logger import get_logger


class LiveBlockTokenProcessor:
    def __init__(self, 
                 rabbitmq_url: str = None, 
                 block_token_processor: BlockTokenProcessor = None,
                 logger=None):
        
        self.logger = logger or get_logger(name="tokens_manager", log_folder="tokens_live")
        self.block_token_processor = block_token_processor or BlockTokenProcessor(logger=self.logger)
        self.block_processed_event = asyncio.Event()

        # Initialize subscriber with our callback and block_token_processor
        self.block_subscriber = BlockSubscriber(
            callback=self._process_block,
            rabbitmq_url=rabbitmq_url,
            logger=self.logger,
            block_token_processor=self.block_token_processor,  # Pass the processor
        )
        
        self._is_shutting_down = False
        self._subscriber_task = None

    async def start(self):
        """Start processing live blocks"""
        try:
            self._is_shutting_down = False
            self.logger.info("Starting block subscriber")
            # Create a task instead of awaiting
            self._subscriber_task = asyncio.create_task(
                self.block_subscriber.start(
                    start_from_block=self.block_token_processor.latest_processed_block
                )
            )
            
        except Exception as e:
            self.logger.error(f"Error starting live processor: {e}")
            await self.stop()
            raise

    async def stop(self):
        """Stop processing live blocks"""
        if self._is_shutting_down:
            return
            
        self._is_shutting_down = True
        self.logger.info("Stopping live block processor...")
        
        try:
            # Stop the subscriber first
            await self.block_subscriber.stop()
            
            # Cancel subscriber task if running
            if self._subscriber_task and not self._subscriber_task.done():
                self._subscriber_task.cancel()
                try:
                    await self._subscriber_task
                except asyncio.CancelledError:
                    pass
                
            self.logger.info("Live block processor stopped successfully")
            
        except Exception as e:
            self.logger.error(f"Error during live processor shutdown: {e}")
            raise

    async def _process_block(self, block_data: List[Dict]):
        """Process incoming blocks"""
        if self._is_shutting_down:
            return
            
        try:
            block_number = block_data[0]['block_number']
            
            # Skip if already processed
            if block_number in self.block_token_processor.processed_blocks:
                self.logger.info(f"Block {block_number} already processed, skipping")
                return
            # Process block
            await self.block_token_processor.process_block(block_data)
            
            # Update tracking
            self.block_token_processor.processed_blocks[block_number] = True
            
            self.block_token_processor.latest_processed_block = max(
                self.block_token_processor.latest_processed_block,
                block_number
            )
            
            # Signal block processed
            self.block_processed_event.set()
            
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing live block: {e}")
            # Don't mark as processed if there was an error
        
    @property
    def updated_tokens(self):
        """Return updated tokens"""
        return self.block_token_processor.updated_tokens
    
    @property
    def latest_processed_block(self):
        """Return latest processed block"""
        return self.block_token_processor.latest_processed_block