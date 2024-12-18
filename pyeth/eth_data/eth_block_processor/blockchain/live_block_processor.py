import asyncio
import dataclasses
import time
from decimal import Decimal
import orjson
from eth_typing import ChecksumAddress
from hexbytes import HexBytes
from web3 import AsyncWeb3
from web3.providers import WebSocketProvider
import aio_pika

from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.utils.logger import get_logger

logger = get_logger(name="live_block_processor", log_folder="eth_block_processor")


def transaction_serializer(obj):
    """Serializer that handles dataclasses and Decimal values"""
    if dataclasses.is_dataclass(obj):
        return transaction_serializer(dataclasses.asdict(obj))
    if isinstance(obj, (HexBytes, bytes)):
        return obj.hex()
    if isinstance(obj, set):
        return list(obj)
    if isinstance(obj, ChecksumAddress):
        return str(obj)
    if isinstance(obj, dict):
        return {k: transaction_serializer(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [transaction_serializer(item) for item in obj]
    if isinstance(obj, Decimal):    
        return float(obj)
    return obj


class LiveBlockProcessor:
    """
    LiveBlockProcessor handles real-time block monitoring and processing.
    
    Objective:
    - Monitor new Ethereum blocks via WebSocket
    - Process blocks and publish to RabbitMQ for alert processing
    - Ensure non-blocking operations for high performance
    - Handle connection failures and reconnections gracefully
    
    Flow:
    1. Connect to Ethereum node via WebSocket
    2. Subscribe to new block headers
    3. Process each block with transaction details
    4. Publish processed blocks to RabbitMQ
    5. Handle errors and reconnections
    """
    
    def __init__(
        self,
        websocket_url: str = "ws://127.0.0.1:8546",
        http_url: str = "http://127.0.0.1:8545",
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        save_erc20_txns: bool = False
    ):
        # Initialize WebSocket provider and web3 instance
        self.provider = WebSocketProvider(websocket_url)
        self.w3 = AsyncWeb3(self.provider)
        self.rabbitmq_url = rabbitmq_url
        
        # Initialize BlockProcessor with HTTP connection for detailed data fetching
        self.block_processor = BlockProcessor(
            node_url=http_url,
            save_erc20_txn_to_db=save_erc20_txns,
        )
        
        # RabbitMQ connection and channel
        self.connection = None
        self.channel = None
        self.exchange = None

        self.reconnect_delay = 1  

    async def setup_rabbitmq(self):
        """Initialize RabbitMQ connection and channel."""
        try:
            if not self.connection or self.connection.is_closed:
                self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
                self.channel = await self.connection.channel()
                self.exchange = await self.channel.declare_exchange(
                    "blocks_exchange",
                    aio_pika.ExchangeType.FANOUT,
                    durable=True
                )
                logger.info("Successfully connected to RabbitMQ")
        except Exception as e:
            logger.error(f"Failed to setup RabbitMQ connection: {e}")
            raise

    async def publish_block(self, block_number: int, processed_block):
        """Publish the processed block to RabbitMQ."""
        try:
            if not self.exchange:
                await self.setup_rabbitmq()
            
            # Convert block data to JSON-serializable format
            block_data = orjson.dumps(
                processed_block,
                default=transaction_serializer,
                option=orjson.OPT_SERIALIZE_NUMPY
            )
        
            message = aio_pika.Message(
                body=block_data,
                delivery_mode=aio_pika.DeliveryMode.PERSISTENT,
                content_type='application/json',
                headers={'block_number': str(block_number)} 
            )
            
            await self.exchange.publish(
                message, 
                routing_key='processed_blocks'
            )
            logger.info(f"RabbitMQ: Published block {block_number}")
            
        except Exception as e:
            logger.error(f" {__name__} Error publishing processed block to RabbitMQ: {e}: {processed_block[0]}")
            # Attempt to reconnect on next publish
            self.exchange = None
            raise

    async def process_latest_block(self, block_number: int) -> dict:
        """
        Process a single block and prepare it for publishing.
        
        Args:
            block_hash: The hash of the block to process
            
        Returns:
            dict: The processed block data or None if processing fails
        """
        try:
            start_time = time.time()
            # Pass the full block directly to process_block
            processed_block = await self.block_processor.process_block(block_number=block_number)
            end_time = time.time()
            logger.info(f"Processed block {block_number} with {len(processed_block)} processed transactions in {end_time - start_time:.2f} seconds")                
            return processed_block
            
        except Exception as e:
            logger.error(f" {__name__} Error processing block {block_number} transactions: {e}")
            return

    async def monitor_new_blocks(self):
        """Monitor new blocks in real-time using WebSocket subscription."""
        try:
            async with self.w3:  # Properly manage WebSocket lifecycle
                if not await self.w3.is_connected():
                    raise ConnectionError("Failed to connect to WebSocket")
                
                subscription_id = await self.w3.eth.subscribe("newHeads")
                logger.info(f"Subscribed to newHeads with ID: {subscription_id}")
                
                async for message in self.w3.socket.process_subscriptions():
                    try:
                        block_data = message.get("result", {})
                        if not block_data or "hash" not in block_data:
                            continue
                        block_number = block_data["number"] if isinstance(block_data["number"], int) else int(block_data["number"], 16)
                        processed_block = await self.process_latest_block(block_number=block_number)   
                        if processed_block:
                            await self.publish_block(block_number=block_number, processed_block=processed_block)
                    except Exception as e:
                        continue
        
        except Exception as e:
            logger.error(f"WebSocket subscription error: {e}", exc_info=True)
            await asyncio.sleep(self.reconnect_delay)
            self.reconnect_delay = min(self.reconnect_delay * 2, 60)  # Max 60s delay
            # Recursive call to restart monitoring
            await self.monitor_new_blocks()
            
        finally:
            # Cleanup WebSocket connection
            if hasattr(self, 'w3'):
                await self.w3.provider.disconnect()

    async def cleanup(self):
        """Cleanup WebSocket and RabbitMQ connections."""
        try:
            if hasattr(self, 'w3'):
                await self.w3.provider.disconnect()
            
            if self.connection and not self.connection.is_closed:
                await self.connection.close()
                
            self.exchange = None
            self.channel = None
            self.connection = None
            
        except Exception as e:
            logger.error(f"Error during cleanup: {e}")

    async def run(self):
        """Main entry point to run the LiveBlockProcessor."""
        try:
            await self.monitor_new_blocks()
        except KeyboardInterrupt:
            logger.info("Received shutdown signal")
        finally:
            await self.cleanup()
