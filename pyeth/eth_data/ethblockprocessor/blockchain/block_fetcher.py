# src/blockchain/block_fetcher.py

"""
BlockFetcher Module

This module is responsible for fetching Ethereum blocks from an Erigon node. It serves as the 
primary interface between our application and the Ethereum blockchain.

The main objective of this module is to provide a reliable and efficient way to retrieve the 
latest blocks from the Ethereum network, enabling real-time analysis of blockchain data.

Key responsibilities:
1. Establish and maintain a connection to an Erigon node
2. Fetch the latest block from the Ethereum network
3. Handle network errors and implement retry mechanisms
4. Provide methods to retrieve specific blocks by number or hash
5. Implement rate limiting to avoid overwhelming the Erigon node

This module is essential for achieving the project's objective of real-time Ethereum block 
analysis, as it provides the raw block data that will be processed by other components of the system.
"""

import asyncio
from web3 import Web3
from web3.exceptions import BlockNotFound

class BlockFetcher:
    def __init__(self, node_url: str, max_retries: int = 3, retry_delay: float = 1.0):
        """
        Initialize the BlockFetcher.

        :param node_url: URL of the Erigon node
        :param max_retries: Maximum number of retries for failed requests
        :param retry_delay: Delay between retries in seconds
        """
        self.w3 = Web3(Web3.HTTPProvider(node_url))
        self.max_retries = max_retries
        self.retry_delay = retry_delay

    async def fetch_latest_block_number(self):
        """
        Fetch the latest block number from the Ethereum network.

        :return: The latest block
        """
        for attempt in range(self.max_retries):
            try:
                return self.w3.eth.get_block('latest', full_transactions=True)
            except Exception as e:
                if attempt == self.max_retries - 1:
                    raise
                await asyncio.sleep(self.retry_delay)

    async def fetch_block_by_number(self, block_number: int):
        """
        Fetch a specific block by its number.

        :param block_number: The number of the block to fetch
        :return: The requested block
        """
        for attempt in range(self.max_retries):
            try:
                return self.w3.eth.get_block(block_number, full_transactions=True)
            except BlockNotFound:
                return None
            except Exception as e:
                if attempt == self.max_retries - 1:
                    raise
                await asyncio.sleep(self.retry_delay)

    async def fetch_block_by_hash(self, block_hash: str):
        """
        Fetch a specific block by its hash.

        :param block_hash: The hash of the block to fetch
        :return: The requested block
        """
        for attempt in range(self.max_retries):
            try:
                return self.w3.eth.get_block(block_hash, full_transactions=True)
            except BlockNotFound:
                return None
            except Exception as e:
                if attempt == self.max_retries - 1:
                    raise
                await asyncio.sleep(self.retry_delay)

    async def close(self):
        """
        Close the connection to the Erigon node.
        """
        # If there's any cleanup needed, do it here
        pass