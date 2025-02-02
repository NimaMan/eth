"""
LiveBlockProcessor: Real-time Ethereum Block Processing and Alert System

Objective:
---------
Create a high-performance, resilient system that:
1. Monitors the Ethereum blockchain in real-time
2. Processes blocks and their transactions
3. Generates and publishes alerts based on transaction patterns in less than 1 second
4. Maintains system stability through proper error handling and reconnection logic

Architecture & Workflow:
----------------------
1. Blockchain Connectivity:
    - Uses WebSocket for real-time block notifications (faster than polling)
    - Maintains separate HTTP connection for detailed data fetching
    - Implements automatic reconnection with exponential backoff

2. Block Processing Pipeline:
    - Receives new block headers via WebSocket subscription
    - Fetches full block data including transactions
    - Processes transactions to extract:
        * ERC20/721/1155 transfers
        * Internal transactions
        * Contract interactions
        * Other relevant on-chain events

3. Message Queue Integration:
    - Uses RabbitMQ for reliable message delivery
    - Maintains two separate exchanges:
        * blocks_exchange: For processed block data
        * alerts_exchange: For detected alerts
    - Implements persistent messaging to prevent data loss

4. Alert Processing:
    - Analyzes transactions for specific patterns
    - Generates alerts based on configurable criteria
    - Processes alerts concurrently for better performance

Key Design Decisions:
-------------------
1. Separation of Concerns:
    - WebSocket for notifications, HTTP for data fetching
    - Separate exchanges for blocks and alerts
    - Modular processing pipeline for maintainability

2. Error Handling:
    - Graceful handling of connection failures
    - Automatic reconnection with backoff
    - Continued processing despite individual failures
    - Exchange reinitialization on connection issues

3. Performance Optimization:
    - Asynchronous processing throughout
    - Efficient serialization with orjson
    - Minimal blocking operations
    - Concurrent alert processing

4. Data Integrity:
    - Persistent message delivery
    - Transaction validation
    - Proper cleanup on shutdown
    - Error logging for debugging

Configuration Options:
--------------------
- websocket_url: WebSocket endpoint for real-time updates
- http_url: HTTP endpoint for detailed data fetching
- rabbitmq_url: RabbitMQ connection string
- save_erc20_txns: Toggle for ERC20 transaction storage

Error Handling Strategy:
----------------------
1. Connection Failures:
    - Automatic reconnection with exponential backoff
    - Separate handling for WebSocket and RabbitMQ
    - Resource cleanup before reconnection attempts

2. Processing Errors:
    - Continue processing on non-critical errors
    - Log errors for debugging
    - Reset connections when necessary
    - Maintain system stability

3. Data Validation:
    - Verify block and transaction data
    - Handle missing or malformed data
    - Proper type checking and conversion

Dependencies:
------------
- web3: Ethereum interaction
- aio_pika: RabbitMQ integration
- orjson: High-performance JSON handling
- asyncio: Asynchronous operations

Usage:
------
1. Initialize:
    processor = LiveBlockProcessor(websocket_url, http_url, rabbitmq_url)

2. Run:
    await processor.run()

3. Cleanup:
    await processor.cleanup()

Note: This system is designed for production use with emphasis on:
- Reliability: Handles network issues and data anomalies
- Performance: Optimized for high-throughput processing
- Maintainability: Clear separation of concerns and error handling
- Scalability: Modular design for easy extension
"""

import asyncio
import dataclasses
from decimal import Decimal
import orjson
from eth_typing import ChecksumAddress
from hexbytes import HexBytes
from web3 import AsyncWeb3
from web3.providers import WebSocketProvider
import aio_pika

from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_block_processor.alert.block_alert_processor import BlockAlertProcessor
from eth_block_processor.utils.logger import get_logger


logger = get_logger(name="block_processor", log_folder="eth_block_processor")
alert_logger = get_logger("alert_processor", log_folder="alert")


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
    elif isinstance(obj, int) and (obj > 2**63 - 1 or obj < -(2**63)):
        return str(obj)
    return obj


def alert_serializer(alert_data):
    """Serialize alert data, converting bytes and other special types to JSON-compatible format"""
    if dataclasses.is_dataclass(alert_data):
        return transaction_serializer(dataclasses.asdict(alert_data))
    if isinstance(alert_data, (str, int, float, bool, type(None))):
        return alert_data
    elif isinstance(alert_data, bytes):
        return alert_data.hex()  # Convert bytes to hex string
    elif isinstance(alert_data, (list, tuple)):
        return [alert_serializer(item) for item in alert_data]
    elif isinstance(alert_data, dict):
        return {k: alert_serializer(v) for k, v in alert_data.items()}
    elif hasattr(alert_data, '__dict__'):
        # Handle dataclass/custom objects
        return {k: alert_serializer(v) for k, v in alert_data.__dict__.items()}
    else:
        return str(alert_data)


class LiveBlockProcessor:
    
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
        self.block_alert_processor = BlockAlertProcessor()
        # RabbitMQ connection and channel
        self.connection = None
        self.channel = None
        self.blocks_exchange = None
        self.alerts_exchange = None

        self.reconnect_delay = 1  

    async def setup_rabbitmq(self, max_retries=3):
        """Initialize RabbitMQ connection and channel with retries."""
        retry_count = 0
        while retry_count < max_retries:
            try:
                # First cleanup any existing connections
                if self.connection:
                    if not self.connection.is_closed:
                        await self.connection.close()
                    self.connection = None
                    self.channel = None
                    self.blocks_exchange = None
                    self.alerts_exchange = None

                # Create new connection
                self.connection = await aio_pika.connect_robust(self.rabbitmq_url)
                self.channel = await self.connection.channel()
                self.blocks_exchange = await self.channel.declare_exchange(
                    "blocks_exchange",
                    aio_pika.ExchangeType.FANOUT,
                    durable=True
                )
                self.alerts_exchange = await self.channel.declare_exchange(
                    "alerts_exchange",
                    aio_pika.ExchangeType.FANOUT,
                    durable=True
                )
                logger.info("Successfully connected to RabbitMQ")
                return True
                
            except Exception as e:
                retry_count += 1
                logger.error(f"Failed to setup RabbitMQ connection (attempt {retry_count}/{max_retries}): {e}")
                await asyncio.sleep(min(2 ** retry_count, 30))  # Exponential backoff
                
        raise RuntimeError(f"Failed to setup RabbitMQ after {max_retries} attempts")

    async def publish_block(self, block_number: int, processed_block):
        """Publish the processed block to RabbitMQ. Continue on failure."""
        try:
            # Only try to setup if we don't have an exchange
            if not self.blocks_exchange:
                success = await self.setup_rabbitmq()
                if not success:
                    logger.error(f"Failed to initialize RabbitMQ for block {block_number}")
                    return False  # Return False but don't reset exchange
            
            try:
                # Try to serialize first to catch any serialization errors
                block_data = orjson.dumps(
                    processed_block,
                    default=transaction_serializer,
                    option=orjson.OPT_SERIALIZE_NUMPY
                )
            except Exception as e:
                logger.error(f"Serialization error for block {block_number}: {e}")
                return False  # Continue with next block without resetting exchange
        
            message = aio_pika.Message(
                body=block_data,
                delivery_mode=aio_pika.DeliveryMode.PERSISTENT,
                content_type='application/json',
                headers={'block_number': str(block_number)} 
            )
            
            await self.blocks_exchange.publish(
                message, 
                routing_key='processed_blocks'
            )
            return True
            
        except aio_pika.exceptions.ConnectionClosed:
            logger.error(f"RabbitMQ connection lost while publishing block {block_number}")
            self.blocks_exchange = None  # Only reset on actual connection issues
            return False
            
        except Exception as e:
            logger.error(f"{__name__} Error publishing block {block_number}: {e}")
            return False  # Don't reset exchange for other errors

    async def publish_alert(self, alert_data):
        """Publish alert to RabbitMQ alerts exchange"""
        try:
            # First check if exchange exists, if not set it up
            if not self.alerts_exchange:
                success = await self.setup_rabbitmq()
                if not success:
                    alert_logger.error("Failed to initialize RabbitMQ for alert")
                    return False
            
            try:
                # Try to serialize first to catch any serialization errors
                serialized_data = orjson.dumps(
                    alert_data,
                    default=alert_serializer,
                    option=orjson.OPT_SERIALIZE_NUMPY
                )
            except Exception as e:
                alert_logger.error(f"Alert serialization error: {e}")
                return False  # Continue without resetting exchange
            
            message = aio_pika.Message(
                body=serialized_data,
                delivery_mode=aio_pika.DeliveryMode.PERSISTENT,
                content_type='application/json'
            )
            
            await self.alerts_exchange.publish(
                message, 
                routing_key="alerts"
            )
            alert_logger.info(f"Successfully published {len(alert_data)} alerts")
            return True
            
        except aio_pika.exceptions.ConnectionClosed:
            alert_logger.error(f"{__name__}: RabbitMQ connection lost while publishing alert")
            self.alerts_exchange = None  # Only reset on connection issues
            return False
            
        except Exception as e:
            alert_logger.error(f"{__name__}: Error publishing alert: {e}", exc_info=True)
            return False  # Don't reset exchange for other errors

    async def process_latest_block(self, block_number: int):
        """
        Process a single block and prepare it for publishing.
        """
        try:
            processed_block = await self.block_processor.process_block(block_number=block_number)
            return processed_block
        except Exception as e:
            logger.error(f" {__name__} Error processing block {block_number} transactions: {e}")
            return
        
    async def process_latest_block_alerts(self, processed_block):
        """Process alerts for the latest block"""
        try:
            alerts = await self.block_alert_processor.process_block_transactions(processed_block)
            return alerts
        except Exception as e:
            logger.error(f" {__name__} Error processing alerts: {e}")
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
                            # Continue with alerts even if block publish fails
                            publish_success = await self.publish_block(block_number, processed_block)
                            if not publish_success:
                                logger.warning(f"Failed to publish block {block_number}, continuing with next block")
                                
                            alerts = await self.process_latest_block_alerts(processed_block)
                            if alerts and len(alerts) > 0:
                                await self.publish_alert(alerts)
                            
                    except Exception as e:
                        logger.error(f"{__name__} Error processing block: {e}", exc_info=True)
                        continue  # Continue with next block regardless of error
        
        except Exception as e:
            logger.error(f"WebSocket subscription error: {e}", exc_info=True)
            await asyncio.sleep(self.reconnect_delay)
            self.reconnect_delay = min(self.reconnect_delay * 2, 60)
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
                
            self.blocks_exchange = None
            self.alerts_exchange = None
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
