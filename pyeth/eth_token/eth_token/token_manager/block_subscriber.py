"""
LiveBlockSnapshotSubscriber: Handles block data subscription and snapshot resolution

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
from typing import Optional, Callable

import aio_pika
from aiormq.exceptions import ChannelInvalidStateError
from eth_token.token_manager.block_token_processor import BlockTokenProcessor
from eth_data.live_data_registry import RedisSnapshotReader


class LiveBlockSnapshotSubscriber:
    def __init__(
        self,
        rabbitmq_url: str = "amqp://guest:guest@localhost/",
        callback: Optional[Callable] = None,
        logger=None,
        block_token_processor: Optional[BlockTokenProcessor] = None,
        block_snapshot_reader: Optional[RedisSnapshotReader] = None,
        ):
        self.logger = logger
        self.rabbitmq_url = rabbitmq_url
        self.connection = None
        self.channel = None
        self.queue = None
        self.exchange = None
        self._processing = False
    
        # Create unique queue name for this consumer instance
        self.queue_name = f"token_tracking_blocks_{uuid.uuid4().hex[:8]}"
        self.exchange_name = "block_published_notifier"
        self.routing_key = ""

        self.callback = callback
        # Remove priority queue since we only keep latest block
        self.block_token_processor = block_token_processor
        self.block_snapshot_reader = block_snapshot_reader or RedisSnapshotReader()
        
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
            self.logger.info(
                f"Subscribed to RabbitMQ block notifier '{self.exchange_name}' via exclusive queue '{self.queue_name}'"
            )
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
        """Start consuming messages and processing blocks immediately.

        Exits only on explicit stop() or on connection fatal error. If the
        consumption loop exits unexpectedly while still marked as processing,
        raise to allow callers to react (e.g., trigger shutdown/logging).
        """
        self._processing = True
        await self.connect()
        await self.queue.purge()
        self.logger.info(f"Purged messages from queue and starting immediate block processing")
        
        # Start consuming messages - process immediately, no internal queue
        try:
            async with self.queue.iterator() as queue_iter:
                async for message in queue_iter:
                    if not self._processing:
                        break
                    await self.process_message(message)
        except asyncio.CancelledError:
            # Propagate cancellation so callers can react appropriately
            self.logger.info(f"Block subscriber for queue '{self.queue_name}' cancelled.")
            raise
        except ChannelInvalidStateError as e:
            if self._processing:
                self.logger.error(
                    f"Block subscriber consumption error: {type(e).__name__}: {e}",
                    exc_info=True,
                )
                raise
            # Channel closed while we were shutting down; treat as expected noise
            self.logger.debug(
                "ChannelInvalidStateError encountered during shutdown for queue '%s'",
                self.queue_name,
            )
        except Exception as e:
            # Surface unexpected consumption errors to caller
            self.logger.error(f"Block subscriber consumption error: {type(e).__name__}: {e}", exc_info=True)
            raise
        finally:
            # If we exit the loop while still marked as processing, this was unexpected
            if self._processing:
                msg = f"Block subscriber loop exited unexpectedly for queue '{self.queue_name}'"
                self.logger.warning(msg)
                raise RuntimeError(msg)
            else:
                self.logger.info(f"Block subscriber for queue '{self.queue_name}' stopped")

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
            
            self.logger.info(f"Live block snapshot subscriber for queue '{self.queue_name}' stopped successfully.")
            
        except Exception as e:
            self.logger.error(f"Error during snapshot subscriber shutdown for queue '{self.queue_name}': {e}", exc_info=True)

    async def process_message(self, message: aio_pika.Message):
        """Process block immediately without queuing"""
        try:
            async with message.process():
                payload = orjson.loads(message.body.decode())
                block_payload = self._load_block_payload(payload)
                if block_payload is None:
                    self.logger.warning("Unable to resolve block payload from message: %s", payload)
                    return

                if self.callback:
                    await self.callback(block_payload)
                elif self.block_token_processor:
                    block_number = block_payload.get("block_number")
                    if block_number is None:
                        self.logger.warning("Block payload missing block_number; skipping")
                        return
                    self.block_token_processor.process_block_tokens(
                        block_payload,
                        block_number,
                    )
                    
        except Exception as e:
            self.logger.error(f"Error processing block in LiveBlockSnapshotSubscriber: {e}", exc_info=True)

    def _load_block_payload(self, payload):
        if isinstance(payload, dict) and "transactions" in payload:
            return payload

        block_number = None
        if isinstance(payload, dict):
            block_number = payload.get("block_number")
        elif isinstance(payload, int):
            block_number = payload

        if block_number is None:
            return None

        snapshot = self.block_snapshot_reader.get_block(int(block_number))
        if snapshot is None:
            return None

        header = snapshot.get("header")
        header_json = None
        if header is not None:
            if isinstance(header, str):
                header_json = header
            else:
                # Backcompat for older snapshots stored as dicts
                header_json = orjson.dumps(header).decode()

        return {
            "block_number": snapshot.get("block_number", block_number),
            "block_header": header_json,
            "transactions": snapshot.get("transactions", []),
        }
