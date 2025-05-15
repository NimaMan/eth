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
from eth_token.token_manager.block_token_processor import BlockTokenProcessor


class BlockSubscriber():
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        callback: Optional[Callable] = None,
        max_queue_size: int = 1000,
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
        self._processor_task = None
    
        self.queue_name = "token_analyzer_blocks"
        self.exchange_name = "blocks_exchange"
        self.routing_key = ""

        self.callback = callback
        self.block_queue = asyncio.PriorityQueue(maxsize=max_queue_size)
        self.block_token_processor = block_token_processor
        
    async def connect(self):
        """Establish connection to RabbitMQ"""
        try:
            self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
            self.channel = await self.connection.channel()
            self.exchange = await self.channel.declare_exchange(
                self.exchange_name,
                aio_pika.ExchangeType.FANOUT,
                durable=True
            )
            self.queue = await self.channel.declare_queue(
                self.queue_name,
                durable=True
            )
            await self.queue.bind(self.exchange, routing_key=self.routing_key)
            self.logger.info(f"Connected to RabbitMQ exchange: {self.exchange_name}")
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
        """Start consuming messages and processing blocks"""
        self._processing = True
        await self.connect()
        await self.queue.purge()
        self.logger.info(f"Purged messages from queue")
        # Start block processor
        self._processor_task = asyncio.create_task(self._process_blocks())
        
        # Start consuming messages
        async with self.queue.iterator() as queue_iter:
            async for message in queue_iter:
                if not self._processing:
                    break
                await self.process_message(message)
        
        self.logger.info("Block subscriber started")

    async def stop(self):
        """Stop consuming messages and processing blocks"""
        try:
            self.logger.info(f"Stopping block subscriber for queue '{self.queue_name}'...")
            self._processing = False
            
            # Give the start() loop a moment to react to _processing = False
            # and exit its queue.iterator() context manager gracefully.
            # This helps aio_pika send its basic.cancel before the connection is closed.
            # This is a mitigation, ideal solution involves external coordination of shutdown.
            await asyncio.sleep(0.05)  # 50 milliseconds, adjust if necessary
            
            # Cancel processor task first
            if self._processor_task and not self._processor_task.done():
                self.logger.info(f"Cancelling internal processor task for '{self.queue_name}'...")
                self._processor_task.cancel()
                try:
                    await asyncio.wait_for(self._processor_task, timeout=2.0)
                    self.logger.info(f"Internal processor task for '{self.queue_name}' awaited successfully.")
                except asyncio.CancelledError:
                    self.logger.info(f"Internal processor task for '{self.queue_name}' was cancelled.")
                except asyncio.TimeoutError:
                    self.logger.warning(f"Internal processor task for '{self.queue_name}' timed out during stop.")
                except Exception as e:
                    self.logger.error(f"Error awaiting internal processor task for '{self.queue_name}' during stop: {e}", exc_info=True)
            
            if self.connection and not self.connection.is_closed:
                self.logger.info(f"Closing RabbitMQ connection for '{self.queue_name}'...")
                try:
                    await asyncio.wait_for(self.connection.close(), timeout=2.0)
                    self.logger.info(f"RabbitMQ connection for '{self.queue_name}' closed successfully.")
                except asyncio.TimeoutError:
                    self.logger.warning(f"RabbitMQ connection close for '{self.queue_name}' timed out.")
                except Exception as e:
                    # More specific exceptions like aio_pika.exceptions.AMQPConnectionError could be caught here
                    self.logger.warning(f"Error closing RabbitMQ connection for '{self.queue_name}': {type(e).__name__} - {e}", exc_info=True)
            else:
                self.logger.info(f"RabbitMQ connection for '{self.queue_name}' already closed or not established.")
            
            self.logger.info(f"Block subscriber for queue '{self.queue_name}' stopped successfully.")
            
        except Exception as e:
            self.logger.error(f"Error during block subscriber shutdown for queue '{self.queue_name}': {e}", exc_info=True)
            # Do not re-raise here if this is the top-level stop, to avoid unhandled exceptions during shutdown
            # If this stop is called by another component that expects to handle exceptions, then re-raise.

    async def process_message(self, message: aio_pika.Message):
        """Add block to processing queue"""
        try:
            async with message.process():
                block_data = orjson.loads(message.body.decode())
                block_number = block_data[0].get("block_number")
                self.logger.info(f"Received block {block_number} from publisher")
                await self.block_queue.put((block_number, block_data))
        except Exception as e:
            self.logger.error(f"Error processing block in BlockSubscriber: {e}", exc_info=True)

    async def _process_blocks(self):
        """Process blocks in strict sequence using priority queue"""
        while self._processing:
            try:
                # Get lowest block number from queue
                block_number, block_data = await self.block_queue.get()
                
                self.logger.info(f"Processing block {block_number} from queue")
                
                # Process block
                if self.callback:
                    await self.callback(block_data)
                    self.block_token_processor.processed_blocks[block_number] = True 
                    
                self.block_queue.task_done()
                
            except Exception as e:
                self.logger.error(f"Error processing block {block_number} in BlockSubscriber: {e}", exc_info=True)
                if 'block_data' in locals():
                    self.block_queue.task_done()
