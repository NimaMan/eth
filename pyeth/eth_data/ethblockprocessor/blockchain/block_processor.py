"""
BlockProcessor Module

This module provides the BlockProcessor class, which is responsible for processing
Ethereum blocks and their transactions. It combines the functionality of BlockFetcher
and TransactionAnalyzer to provide comprehensive block and transaction analysis.

Objectives:
1. Process individual Ethereum blocks, including all transactions within them.
2. Analyze the latest block in real-time.
3. Process a range of blocks for historical analysis or catching up after downtime.
4. Retrieve and analyze contract storage at specific blocks.
5. Analyze contract creation transactions.

The BlockProcessor serves as a core component in the EthBlockProcessor system,
enabling detailed blockchain data extraction and analysis for various use cases
such as monitoring, auditing, and data analytics.
"""

from typing import List, Dict, Any
from web3 import Web3
from ..txn.txn_analyzer import TransactionAnalyzer
from .block_fetcher import BlockFetcher
import asyncio
from web3.providers.websocket import WebsocketProvider
from multiprocessing import Pool


class BlockProcessor:
    """
    A class to process Ethereum blocks and their transactions.

    This class combines BlockFetcher and TransactionAnalyzer to provide
    comprehensive block and transaction analysis.

    Attributes:
        w3 (Web3): A Web3 instance for interacting with the Ethereum network.
        block_fetcher (BlockFetcher): An instance of BlockFetcher for retrieving blocks.
        transaction_analyzer (TransactionAnalyzer): An instance of TransactionAnalyzer for analyzing transactions.
    """

    def __init__(self, node_url: str):
        """
        Initialize the BlockProcessor with necessary components.

        Args:
            node_url (str): A URL for interacting with the Ethereum network.
        """
        self.w3 = Web3(Web3.HTTPProvider(node_url))
        self.block_fetcher = BlockFetcher(node_url)
        self.transaction_analyzer = TransactionAnalyzer(node_url)

    async def process_block(self, block_number: int) -> ProcessedBlock:
        """
        Process a single block and its transactions.

        Args:
            block_number (int): The number of the block to process.

        Returns:
            ProcessedBlock: A data structure containing the processed block data.

        Raises:
            ValueError: If the specified block is not found.
        """
        block = await self.block_fetcher.fetch_block_by_number(block_number)
        if not block:
            raise ValueError(f"Block {block_number} not found")

        block_metadata = BlockMetadata(
            number=block['number'],
            hash=block['hash'].hex(),
            parent_hash=block['parentHash'].hex(),
            timestamp=block['timestamp'],
            state_root=block['stateRoot'].hex(),
            receipts_root=block['receiptsRoot'].hex(),
            gas_used=block['gasUsed'],
            gas_limit=block['gasLimit'],
            extra_data=block['extraData'].hex(),
        )

        transactions = []
        detailed_transactions = {}

        for tx in block['transactions']:
            tx_metadata, detailed_tx = await self.transaction_analyzer.analyze_transaction(tx)
            transactions.append(tx_metadata)
            if detailed_tx:
                detailed_transactions[tx_metadata.hash] = detailed_tx

        return ProcessedBlock(
            metadata=block_metadata,
            transactions=transactions,
            detailed_transactions=detailed_transactions
        )

    async def process_latest_block(self) -> Dict[str, Any]:
        """
        Process the most recent block on the Ethereum network.

        Returns:
            Dict[str, Any]: A dictionary containing the processed latest block data.
        """
        latest_block_number = self.w3.eth.get_block_number()
        return await self.process_block(latest_block_number)

    async def process_block_range(self, start_block: int, end_block: int) -> List[Dict[str, Any]]:
        """
        Process a range of blocks.

        Args:
            start_block (int): The starting block number.
            end_block (int): The ending block number (inclusive).

        Returns:
            List[Dict[str, Any]]: A list of dictionaries, each containing processed block data.
        """
        processed_blocks = []
        for block_number in range(start_block, end_block + 1):
            block_data = await self.process_block(block_number)
            processed_blocks.append(block_data)
        return processed_blocks

    def get_storage_at(self, contract_address: str, storage_keys: List[str], block_number: int) -> Dict[str, str]:
        """
        Retrieve storage values for given keys of a contract at a specific block.

        Args:
            contract_address (str): The address of the contract.
            storage_keys (List[str]): A list of storage keys to retrieve.
            block_number (int): The block number at which to retrieve the storage.

        Returns:
            Dict[str, str]: A dictionary mapping storage keys to their values.
        """
        storage_values = {}
        for key in storage_keys:
            storage_values[key] = self.w3.eth.get_storage_at(contract_address, key, block_number)
        return storage_values

    def analyze_contract_creation(self, tx_hash: str) -> Dict[str, Any]:
        """
        Analyze a contract creation transaction.

        Args:
            tx_hash (str): The transaction hash of the contract creation transaction.

        Returns:
            Dict[str, Any]: A dictionary containing details about the newly created contract.
        """
        tx_receipt = self.w3.eth.get_transaction_receipt(tx_hash)
        if tx_receipt['contractAddress'] is None:
            return {"error": "Transaction did not create a contract"}

        contract_address = tx_receipt['contractAddress']
        contract_code = self.w3.eth.get_code(contract_address)

        return {
            "contract_address": contract_address,
            "creator": tx_receipt['from'],
            "creation_tx_hash": tx_hash,
            "block_number": tx_receipt['blockNumber'],
            "code_size": len(contract_code),
            "code": contract_code.hex()
        }