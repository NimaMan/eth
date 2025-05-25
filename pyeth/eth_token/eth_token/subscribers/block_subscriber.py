"""
BlockSubscriber: Handles block data subscription and queuing

Objective:
---------
1. Subscribe to block data from RabbitMQ using exclusive queues for broadcast
2. Maintain only the latest block (drop older blocks automatically)
3. Process blocks as they arrive without strict ordering requirements

Algorithm:
---------
1. Create exclusive, auto-delete queue with unique name per consumer instance
2. Configure queue to keep only 1 message (latest block) using x-max-length=1
3. Bind queue to FANOUT exchange to receive all published blocks
4. Process blocks immediately as they arrive
5. Each consumer instance gets its own copy of every published block
"""
import asyncio
import orjson
import uuid
from typing import Optional, Callable, Dict, List
import aio_pika
from eth_token.utils.logger import get_logger
from eth_token.token_manager.block_token_processor import BlockTokenProcessor


class BlockSubscriber():
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        callback: Optional[Callable] = None,
        logger=None,
        block_token_processor: Optional[BlockTokenProcessor] = None,
        ):
        self.logger = logger or get_logger(name="subscriber", log_folder="tokens_live")
        self.rabbitmq_url = rabbitmq_url
        self.connection = None
        self.channel = None
        self.queue = None
        self.exchange = None
        self._processing = False
    
        # Create unique queue name for this consumer instance
        self.queue_name = f"token_analyzer_blocks_{uuid.uuid4().hex[:8]}"
        self.exchange_name = "blocks_exchange"
        self.routing_key = ""

        self.callback = callback
        # Remove priority queue since we only keep latest block
        self.block_token_processor = block_token_processor
        
    async def connect(self):
        """Establish connection to RabbitMQ with exclusive queue for broadcast"""
        try:
            self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
            self.channel = await self.connection.channel()
            self.exchange = await self.channel.declare_exchange(
                self.exchange_name,
                aio_pika.ExchangeType.FANOUT,
                durable=True
            )
            
            # Create exclusive, auto-delete queue that keeps only latest block
            self.queue = await self.channel.declare_queue(
                self.queue_name,
                exclusive=True,  # Only this connection can access this queue
                auto_delete=True,  # Queue deleted when connection closes
                arguments={
                    'x-max-length': 1,  # Keep only 1 message (latest block)
                    'x-overflow': 'drop-head'  # Drop oldest when new arrives
                }
            )
            await self.queue.bind(self.exchange, routing_key=self.routing_key)
            self.logger.info(f"Connected to RabbitMQ exchange: {self.exchange_name} with exclusive queue: {self.queue_name}")
        except Exception as e:
            self.logger.error(f"Failed to connect to RabbitMQ: {e}")
            raise

    async def disconnect(self):
        """Close RabbitMQ connection"""
        if self.connection and not self.connection.is_closed:
            await self.connection.close()
            self.logger.info("Disconnected from RabbitMQ")

    async def process_message(self, message: aio_pika.Message):
        """Override this method in derived classes"""
        raise NotImplementedError

    async def start(self):
        """Start consuming messages and processing blocks immediately"""
        self._processing = True
        await self.connect()
        await self.queue.purge()
        self.logger.info(f"Purged messages from queue and starting immediate block processing")
        
        # Start consuming messages - process immediately, no internal queue
        async with self.queue.iterator() as queue_iter:
            async for message in queue_iter:
                if not self._processing:
                    break
                await self.process_message(message)
        
        self.logger.info("Block subscriber started")

    async def stop(self):
        """Stop consuming messages"""
        try:
            self.logger.info(f"Stopping block subscriber for queue '{self.queue_name}'...")
            self._processing = False
            
            # Give the start() loop a moment to react to _processing = False
            await asyncio.sleep(0.05)  # 50 milliseconds
            
            if self.connection and not self.connection.is_closed:
                self.logger.info(f"Closing RabbitMQ connection for '{self.queue_name}'...")
                try:
                    await asyncio.wait_for(self.connection.close(), timeout=2.0)
                    self.logger.info(f"RabbitMQ connection for '{self.queue_name}' closed successfully.")
                except asyncio.TimeoutError:
                    self.logger.warning(f"RabbitMQ connection close for '{self.queue_name}' timed out.")
                except Exception as e:
                    self.logger.warning(f"Error closing RabbitMQ connection for '{self.queue_name}': {type(e).__name__} - {e}", exc_info=True)
            else:
                self.logger.info(f"RabbitMQ connection for '{self.queue_name}' already closed or not established.")
            
            self.logger.info(f"Block subscriber for queue '{self.queue_name}' stopped successfully.")
            
        except Exception as e:
            self.logger.error(f"Error during block subscriber shutdown for queue '{self.queue_name}': {e}", exc_info=True)

    async def process_message(self, message: aio_pika.Message):
        """Process block immediately without queuing"""
        try:
            async with message.process():
                block_data = orjson.loads(message.body.decode())
                block_number = block_data[0].get("block_number")
                self.logger.info(f"Processing block {block_number} immediately")
                
                # Process block immediately
                if self.callback:
                    await self.callback(block_data)
                elif self.block_token_processor:
                    await self.block_token_processor.process_block(block_data)
                    
        except Exception as e:
            self.logger.error(f"Error processing block in BlockSubscriber: {e}", exc_info=True)
