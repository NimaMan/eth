"""
BlockSubscriber: Handles block data subscription and queuing

Objective:
---------
1. Subscribe to block data from RabbitMQ
2. Maintain ordered block processing queue
3. Provide processed blocks to token manager
"""
import asyncio
import orjson
from typing import Optional, Callable, Dict, List
import aio_pika
from eth_tokens_live.subscribers.base_subscriber import BaseSubscriber
from eth_tokens_live.utils.logger import get_logger


class BlockSubscriber(BaseSubscriber):
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        callback: Optional[Callable] = None,
        max_queue_size: int = 1000,
        logger=None
    ):
        super().__init__(
            rabbitmq_url=rabbitmq_url,
            queue_name="token_analyzer_blocks",
            exchange_name="blocks_exchange",
            routing_key=""
        )
        self.callback = callback
        self.block_queue = asyncio.Queue(maxsize=max_queue_size)
        self.processed_blocks: Dict[int, List[Dict]] = {}
        self._processing = False
        self._processor_task = None
        self.logger = logger
        if logger is None:
            self.logger = get_logger(name="tokens_live", log_folder="tokens_live")
        
    async def start(self):
        """Start consuming messages and processing blocks"""
        self._processing = True
        await self.connect()
        
        # Start block processor
        self._processor_task = asyncio.create_task(self._process_blocks())
        
        # Start consuming messages
        async with self.queue.iterator() as queue_iter:
            async for message in queue_iter:
                await self.process_message(message)
        
        self.logger.info(f"Block subscriber started")

    async def stop(self):
        """Stop consuming messages and processing blocks"""
        self._processing = False
        await self.block_queue.join()
        await self.disconnect()
        self.logger.info(f"Block subscriber stopped")

    async def process_message(self, message: aio_pika.Message):
        """Add block to processing queue"""
        try:
            async with message.process():
                block_data = orjson.loads(message.body.decode())
                await self.block_queue.put(block_data)
                
        except Exception as e:
            self.logger.error(f"Error processing block message: {e}")

    async def _process_blocks(self):
        """Process blocks in order"""
        while self._processing:
            try:
                block_data = await self.block_queue.get()
                block_number = block_data[0].get("block_number")
                
                # Skip if already processed
                if block_number in self.processed_blocks:
                    continue
                    
                if self.callback:
                    await self.callback(block_data)
                    
                # Update tracking
                self.processed_blocks[block_number] = block_data
                self.logger.debug(f"Processed block {block_number}")
                self.block_queue.task_done()
                
            except Exception as e:
                self.logger.error(f"{__name__} Error processing block: {e}")
