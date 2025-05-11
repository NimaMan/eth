"""
BlockSubscriber: Handles block data subscription and queuing

Objective:
---------
1. Subscribe to block data from RabbitMQ
2. Maintain ordered block processing queue
3. Process blocks in strict sequence
"""
import asyncio
import orjson
from typing import Optional, Callable, Dict, List
import aio_pika
from eth_token.utils.logger import get_logger
from eth_token.subscribers.base_subscriber import BaseSubscriber
from eth_token.token_manager.block_token_processor import BlockTokenProcessor


class BlockSubscriber(BaseSubscriber):
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        callback: Optional[Callable] = None,
        max_queue_size: int = 1000,
        logger=None,
        block_token_processor: Optional[BlockTokenProcessor] = None,
        ):
        super().__init__(
            rabbitmq_url=rabbitmq_url,
            queue_name="token_analyzer_blocks",
            exchange_name="blocks_exchange",
            routing_key=""
        )
        self.callback = callback
        self.block_queue = asyncio.PriorityQueue(maxsize=max_queue_size)
        self.block_token_processor = block_token_processor
        self._processing = False
        self._processor_task = None
        self.logger = logger or get_logger(name="subscriber", log_folder="tokens_live")
        
    async def start(self, start_from_block: Optional[int] = None):
        """Start consuming messages and processing blocks"""
        self._processing = True
        await self.connect()
        
        # Start block processor
        self._processor_task = asyncio.create_task(self._process_blocks())
        
        # Start consuming messages
        async with self.queue.iterator() as queue_iter:
            async for message in queue_iter:
                if not self._processing:
                    break
                await self.process_message(message, start_from_block=start_from_block)
        
        self.logger.info("Block subscriber started")

    async def stop(self):
        """Stop consuming messages and processing blocks"""
        try:
            self.logger.info("Stopping block subscriber...")
            self._processing = False
            
            # Cancel processor task first
            if self._processor_task and not self._processor_task.done():
                self._processor_task.cancel()
                try:
                    await asyncio.wait_for(self._processor_task, timeout=2.0)
                except (asyncio.CancelledError, asyncio.TimeoutError):
                    self.logger.warning("Processor task cancelled or timed out")
            
            if self.connection and not self.connection.is_closed:
                try:
                    await asyncio.wait_for(self.connection.close(), timeout=2.0)
                except (asyncio.TimeoutError, Exception) as e:
                    self.logger.warning(f"Connection close timed out or failed: {e}")
            
            self.logger.info("Block subscriber stopped successfully")
            
        except Exception as e:
            self.logger.error(f"Error during block subscriber shutdown: {e}")
            raise

    async def process_message(self, message: aio_pika.Message, start_from_block: Optional[int] = None):
        """Add block to processing queue"""
        try:
            async with message.process():
                block_data = orjson.loads(message.body.decode())
                block_number = block_data[0].get("block_number")
                if block_number > start_from_block:
                    # Add to priority queue with block number as priority
                    await self.block_queue.put((block_number, block_data))
                else:
                    self.logger.debug(f"Skipping block {block_number} as it is less than start_from_block {start_from_block}")
        except Exception as e:
            self.logger.error(f"Error processing block in BlockSubscriber: {e}", exc_info=True)

    async def _process_blocks(self):
        """Process blocks in strict sequence using priority queue"""
        while self._processing:
            try:
                # Get lowest block number from queue
                block_number, block_data = await self.block_queue.get()
                
                # Process block
                if self.callback:
                    await self.callback(block_data)
                    self.block_token_processor.processed_blocks[block_number] = True 
                    
                self.block_queue.task_done()
                
            except Exception as e:
                self.logger.error(f"Error processing block {block_number} in BlockSubscriber: {e}", exc_info=True)
                if 'block_data' in locals():
                    self.block_queue.task_done()
