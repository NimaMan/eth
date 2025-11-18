"""
LiveBlockProcessor: Real-time Ethereum Block Processing and Notification System

Objective:
---------
Create a high-performance, resilient system that:
1. Monitors the Ethereum blockchain in real-time
2. Processes blocks and their transactions
3. Publishes live block metadata to downstream consumers in less than 1 second
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

3. Message Distribution:
    - Publishes processed block numbers to Redis Pub/Sub (`live_blocks` by default)
    - Writes block headers and processed transactions to Redis for downstream replay
    - Ensures consumers receive both the notification and the cached state

Key Design Decisions:
-------------------
1. Separation of Concerns:
    - WebSocket for notifications, HTTP for data fetching
    - Redis for both live notifications and short-term state storage
    - Modular processing pipeline for maintainability

2. Error Handling:
    - Graceful handling of connection failures
    - Automatic reconnection with backoff
    - Continued processing despite individual failures

3. Performance Optimization:
    - Asynchronous processing throughout
    - Minimal blocking operations
    - Concurrent tasks for Redis publishing and address indexing

4. Data Integrity:
    - Redis snapshot retention protects consumers from transient node lag
    - Transaction validation before publishing
    - Proper cleanup on shutdown
    - Error logging for debugging

Configuration Options:
--------------------
- websocket_url: WebSocket endpoint for real-time updates
- http_url: HTTP endpoint for detailed data fetching
- redis_url: Redis connection string (falls back to LIVE_BLOCKCHAIN_DATA_REDIS_URL)
- live_block_cache_size: Number of recent block snapshots to retain

Error Handling Strategy:
----------------------
1. Connection Failures:
    - Automatic reconnection with exponential backoff
    - Resource cleanup before reconnection attempts

2. Processing Errors:
    - Continue processing on non-critical errors
    - Log errors for debugging
    - Maintain system stability

3. Data Validation:
    - Verify block and transaction data
    - Handle missing or malformed data
    - Proper type checking and conversion

Dependencies:
------------
- web3: Ethereum interaction
- redis: Redis Pub/Sub
- asyncio: Asynchronous operations

Usage:
------
1. Initialize:
    processor = LiveBlockProcessor(websocket_url, http_url)

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
import os
from collections import deque
from typing import Optional

from web3 import AsyncWeb3
from web3.providers import WebSocketProvider

from eth_data.blockchain.block_data_models import ProcessedBlockResult
from eth_data.blockchain.block_processor import BlockProcessor
from eth_data.database.writers.transaction_writer import TransactionAddresstoTxIndexer
from eth_data.live_data_registry import LiveDataPublisher, build_block_snapshot
from eth_data.utils.logger import get_logger
from eth_data.notifications import RedisSignalPublisher


class LiveBlockProcessor:
    
    def __init__(
        self,
        websocket_url: str = "ws://127.0.0.1:8546",
        http_url: str = "http://127.0.0.1:8545",
        index_address_txs: bool = False, 
        logger=None,
        redis_url: Optional[str] = None,
        block_notification_channel: str = "live_blocks",
        live_block_cache_size: int = 3,
    ):
        # Initialize WebSocket provider and web3 instance
        self.provider = WebSocketProvider(websocket_url)
        self.w3 = AsyncWeb3(self.provider)
        self.index_address_txs = index_address_txs
        self.logger = logger or get_logger(name="live_block_processor")
        self.redis_url = redis_url or os.getenv(
            "LIVE_BLOCKCHAIN_DATA_REDIS_URL", "redis://localhost:6379/0"
        )
        self._block_notification_channel = block_notification_channel
        
        # Initialize BlockProcessor with HTTP connection for detailed data fetching
        self.block_processor = BlockProcessor(
            node_url=http_url,
            logger=self.logger,
            index_address_txs=self.index_address_txs,
        )
        self.index_writer: Optional[TransactionAddresstoTxIndexer] = None
        if self.index_address_txs:
            # Reuse the BlockProcessor writer but keep live ingestion asynchronous.
            self.index_writer = (
                self.block_processor.transaction_writer
                if self.block_processor.transaction_writer is not None
                else TransactionAddresstoTxIndexer()
            )
            self.block_processor.transaction_writer = None
            self.block_processor.index_address_txs = False
        self.reconnect_delay = 1  
        self._live_block_retention = max(0, int(live_block_cache_size))
        self._recent_blocks = deque()
        self._live_data_publisher = (
            LiveDataPublisher() if self._live_block_retention > 0 else None
        )
        self._block_signal_publisher = RedisSignalPublisher(redis_url=self.redis_url)

        # Downstream pipeline state
        self._queue_maxsize = 128
        self.publish_queue: Optional[asyncio.Queue] = None
        self.index_queue: Optional[asyncio.Queue] = None
        self.publish_task: Optional[asyncio.Task] = None
        self.index_task: Optional[asyncio.Task] = None
    
    async def start_workers(self):
        """Initialize background tasks that handle publishing and optional indexing."""
        if self.publish_queue is None:
            self.publish_queue = asyncio.Queue(maxsize=self._queue_maxsize)
        if self.publish_task is None or self.publish_task.done():
            self.publish_task = asyncio.create_task(
                self._publish_worker(), name="block_publish_worker"
            )

        if self.index_address_txs and self.index_writer is not None:
            if self.index_queue is None:
                self.index_queue = asyncio.Queue(maxsize=self._queue_maxsize)
            if self.index_task is None or self.index_task.done():
                self.index_task = asyncio.create_task(
                    self._index_worker(), name="block_index_worker"
                )

    async def stop_workers(self):
        """Gracefully stop background workers."""
        await self._stop_worker(self.publish_queue, self.publish_task)
        await self._stop_worker(self.index_queue, self.index_task)
        self.publish_queue = None
        self.index_queue = None
        self.publish_task = None
        self.index_task = None

    async def _stop_worker(
        self,
        queue: Optional[asyncio.Queue],
        task: Optional[asyncio.Task],
    ):
        if queue is None or task is None:
            return
        await queue.put(None)
        await task

    async def _publish_worker(self):
        """Publish processed blocks without blocking head ingestion."""
        assert self.publish_queue is not None
        while True:
            item = await self.publish_queue.get()
            if item is None:
                self.publish_queue.task_done()
                break
            try:
                block_number, processed_block = item
                await self._publish_live_block_snapshot(block_number, processed_block)
                notified = await self.publish_block_notification(block_number)
                if not notified:
                    self.logger.warning(
                        "Failed to publish block %s notification, continuing",
                        block_number,
                    )
                elif self.index_queue is not None:
                    await self.index_queue.put((block_number, processed_block))
            except Exception as exc:
                self.logger.error("Publish worker error for block %s: %s", block_number, exc)
            finally:
                self.publish_queue.task_done()

    async def _index_worker(self):
        """Write address-index data asynchronously when enabled."""
        assert self.index_queue is not None
        writer = self.index_writer
        if writer is None:
            # Drain queue to avoid blocking even if writer is unexpectedly missing.
            while True:
                item = await self.index_queue.get()
                if item is None:
                    self.index_queue.task_done()
                    break
                self.index_queue.task_done()
            return

        while True:
            item = await self.index_queue.get()
            if item is None:
                self.index_queue.task_done()
                break
            try:
                block_number, processed_block = item
                txs = processed_block.transactions
                await asyncio.to_thread(
                    writer.write_transactions_address_tx,
                    txs,
                )
            except Exception as exc:
                self.logger.error(
                    "Index worker error for block %s: %s", block_number, exc
                )
            finally:
                self.index_queue.task_done()

    async def _dispatch_work_item(self, work_item):
        if self.publish_queue is None:
            raise RuntimeError("Publish queue not initialized. Did you call start_workers()?")
        await self.publish_queue.put(work_item)

    async def publish_block_notification(self, block_number: int) -> bool:
        """Publish the processed block number via Redis Pub/Sub. Continue on failure."""
        if not self._block_signal_publisher or not self._block_notification_channel:
            self.logger.warning(
                "Redis signal publisher not configured; skipping block %s notification",
                block_number,
            )
            return False
        try:
            await self._block_signal_publisher.publish(
                self._block_notification_channel,
                {"block_number": block_number},
            )
            return True
        except Exception as exc:
            self.logger.error(
                "Error publishing block %s notification to Redis: %s",
                block_number,
                exc,
                exc_info=True,
            )
            return False

    async def monitor_new_blocks(self):
        """Monitor new blocks in real-time using WebSocket subscription."""
        try:
            async with self.w3:  # Properly manage WebSocket lifecycle
                if not await self.w3.is_connected():
                    raise ConnectionError("Failed to connect to WebSocket")
                
                subscription_id = await self.w3.eth.subscribe("newHeads")
                self.logger.info(f"Subscribed to newHeads with ID: {subscription_id}")
                
                async for message in self.w3.socket.process_subscriptions():
                    try:
                        block_data = message.get("result", {})
                        if not block_data or "hash" not in block_data:
                            continue
                        
                        block_number = block_data["number"] if isinstance(block_data["number"], int) else int(block_data["number"], 16)
                        processed_block_result = await self.block_processor.process_block(block_number=block_number)
                        
                        if processed_block_result:
                            work_item = (block_number, processed_block_result)
                            await self._dispatch_work_item(work_item)
                            
                    except Exception as e:
                        self.logger.error(f"{__name__} Error processing live block {block_number}: {e}", exc_info=True)
                        continue  # Continue with next block regardless of error
        
        except Exception as e:
            self.logger.error(f"WebSocket subscription error: {e}", exc_info=True)
            await asyncio.sleep(self.reconnect_delay)
            self.reconnect_delay = min(self.reconnect_delay * 2, 60)
            await self.monitor_new_blocks()
            
        finally:
            # Cleanup WebSocket connection
            await self.w3.provider.disconnect()

    async def cleanup(self):
        """Cleanup WebSocket connections and background workers."""
        try:
            await self.stop_workers()
            await self.w3.provider.disconnect()
        except Exception as e:
            self.logger.error(f"Error during cleanup: {e}")

    async def _publish_live_block_snapshot(
        self,
        block_number: int,
        processed_block: ProcessedBlockResult,
    ) -> None:
        if not self._live_data_publisher:
            return
        try:
            snapshot = build_block_snapshot(
                processed_block,
                block_number=block_number,
                include_transactions=True,
            )
            await self._live_data_publisher.publish_block(block_number, snapshot)
            self._record_retained_block(block_number)
            await self._evict_old_blocks()
        except Exception as exc:
            self.logger.warning(
                "Failed to publish live snapshot for block %s: %s",
                block_number,
                exc,
            )

    def _record_retained_block(self, block_number: int) -> None:
        if block_number in self._recent_blocks:
            self._recent_blocks.remove(block_number)
        self._recent_blocks.append(block_number)

    async def _evict_old_blocks(self) -> None:
        if not self._live_data_publisher:
            return
        while len(self._recent_blocks) > self._live_block_retention:
            evicted = self._recent_blocks.popleft()
            try:
                await self._live_data_publisher.delete_block(evicted)
            except Exception as exc:
                self.logger.warning(
                    "Failed to evict live snapshot for block %s: %s",
                    evicted,
                    exc,
                )

    async def run(self):
        """Main entry point to run the LiveBlockProcessor."""
        await self.start_workers()
        try:
            await self.monitor_new_blocks()
        except KeyboardInterrupt:
            self.logger.info("Received shutdown signal")
        finally:
            await self.cleanup()
