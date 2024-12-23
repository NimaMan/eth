"""
BlockFetcher Module

Objective:
---------
Provide a high-performance, reliable interface for fetching Ethereum blocks from a node,
serving as the primary data acquisition layer for blockchain analysis.

Key Components and Flow:
----------------------
1. Block Retrieval:
   - Asynchronous fetching of latest blocks
   - Targeted retrieval by block number or hash
   - Full transaction data inclusion

2. Error Handling:
   - Automatic retry mechanism for failed requests
   - Graceful handling of missing blocks
   - Connection management

3. Performance Characteristics:
   - I/O Bound operations suitable for async/await
   - Independent block fetching allowing parallel operations
   - No sequential dependencies between operations

Design Choices:
-------------
- Uses AsyncWeb3 because:
  1. Operations are purely I/O bound
  2. No dependencies between block fetches
  3. Used in async contexts throughout the application
  4. Enables better resource utilization during network waits

Usage:
-----
The fetcher is typically used in two contexts:
1. Real-time block monitoring
2. Historical block analysis
"""

import asyncio
from web3 import AsyncWeb3, AsyncHTTPProvider
from web3.exceptions import BlockNotFound


class BlockFetcher:
    def __init__(self, node_url: str, max_retries: int = 3, retry_delay: float = 1.0):
        """Initialize the BlockFetcher with async Web3 instance."""
        self.w3 = AsyncWeb3(AsyncHTTPProvider(node_url))
        self.max_retries = max_retries
        self.retry_delay = retry_delay

    async def fetch_latest_block_number(self):
        """Fetch the latest block number from the Ethereum network."""
        for attempt in range(self.max_retries):
            try:
                return await self.w3.eth.block_number
            except Exception as e:
                if attempt == self.max_retries - 1:
                    raise
                await asyncio.sleep(self.retry_delay)

    async def fetch_latest_block(self):
        """Fetch the latest block with full transaction data."""
        latest_number = await self.fetch_latest_block_number()
        return await self.fetch_block_by_number(latest_number)

    async def fetch_block_by_number(self, block_number: int):
        """Fetch a specific block by its number."""
        for attempt in range(self.max_retries):
            try:
                return await self.w3.eth.get_block(block_number, full_transactions=True)
            except BlockNotFound:
                return None
            except Exception as e:
                if attempt == self.max_retries - 1:
                    raise
                await asyncio.sleep(self.retry_delay)

    async def fetch_block_by_hash(self, block_hash: str):
        """Fetch a specific block by its hash."""
        for attempt in range(self.max_retries):
            try:
                return await self.w3.eth.get_block(block_hash, full_transactions=True)
            except BlockNotFound:
                return None
            except Exception as e:
                if attempt == self.max_retries - 1:
                    raise
                await asyncio.sleep(self.retry_delay)

    async def close(self):
        """Close the connection to the Ethereum node."""
        if hasattr(self.w3.provider, 'close'):
            await self.w3.provider.close()