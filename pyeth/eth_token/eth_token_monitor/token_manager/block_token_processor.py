"""
BlockTokenProcessor: Core token processing functionality

Objective:
---------
1. Provide core token processing functionality for both live and historical modes
    - Process blocks and their transactions
    - Create and update token instances of LiveERC20Token class
    - Cache token states
2. Process blocks and their transactions efficiently  with Concurrency Limit
"""

import asyncio
from web3 import Web3
from dataclasses import asdict
from typing import Dict, List, Set

from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_token_monitor.services.live_tokens_object_cache import LiveTokenObjectsCache
from eth_token_monitor.utils.logger import get_logger


class BlockTokenProcessor:
    def __init__(self, redis_url: str = "redis://localhost:6379/0", logger=None, max_concurrency=20):
        self.logger = logger or get_logger(name="token_manager", log_folder="tokens_live")
        # Token tracking
        self.live_tokens_cache = LiveTokenObjectsCache(logger=self.logger, redis_url=redis_url)
        self.token_first_seen: Dict[str, int] = {}
        self.updated_tokens: Dict[str, LiveERC20Token] = {}
        self.processed_blocks: Dict[int, bool] = {}
        self.latest_processed_block = 0

        # Introduce concurrency semaphore
        self.semaphore = asyncio.Semaphore(value=max_concurrency)

    async def process_block(self, block_data: List[Dict]):
        """Process a single block's transactions with concurrency limit."""
        if not block_data:
            return
        
        if isinstance(block_data[0], dict):
           block_number = block_data[0].get('block_number')
        else:
            block_number = block_data[0].block_number
        
        self.updated_tokens.clear()
        tasks = []
        for txn in block_data:
            async def sem_task(txn_data=txn):
                async with self.semaphore:
                    return await self._process_transaction(txn_data, block_number)

            tasks.append(asyncio.create_task(sem_task()))
        await asyncio.gather(*tasks)

        # Mark the block as processed (assumes all txns in the same block)
        self.processed_blocks[block_number] = True

    async def _process_transaction(self, transaction: Dict, block_number: int):
        """Process a single transaction and update relevant tokens"""
        if not isinstance(transaction, dict):
            transaction = asdict(transaction)
        try:
            # Handle contract creation
            if self._is_token_creation(transaction):
                await self._handle_token_creation(transaction, block_number)
                return
                
            # Handle regular transactions
            await self._handle_token_transaction(transaction)
                
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing transaction {transaction.get('hash')}: {e}")

    def _is_token_creation(self, transaction: Dict) -> bool:
        """Check if transaction creates a new token"""
        return (
            transaction.get('txn_type') == 'Contract Creation' 
            and transaction.get('contract_address')
            and transaction.get('contract_creation_events', [])
            and transaction['contract_creation_events'][0]["contract_type"] == "ERC-20"
        )

    async def _handle_token_creation(self, transaction: Dict, block_number: int):
        """Handle creation of a new token"""
        new_token_address = transaction['contract_address']
        if new_token_address not in self.live_tokens_cache:
            try:
                token = LiveERC20Token(new_token_address)
                token.update_from_transaction(transaction)
                
                self.live_tokens_cache[new_token_address] = token
                self.token_first_seen[new_token_address] = block_number
                self.updated_tokens[new_token_address] = token
                
                self.logger.info(f"New token created: {new_token_address} in block {block_number}")
                
            except Exception as e:
                self.logger.error(f"{self.__class__.__name__} Failed to create token {new_token_address} at txn {transaction.get('hash')}: {e}")

    async def _handle_token_transaction(self, transaction: Dict):
        """Handle transaction involving existing tokens"""
        erc20_contracts = transaction.get('erc20_contracts', set())
        if not erc20_contracts:
            return
            
        update_tasks = []
        for token_address in erc20_contracts:
            token = self.live_tokens_cache[token_address]
            if token:
                update_tasks.append(
                    self._update_token_safe(
                        token=token,
                        transaction=transaction,
                        token_address=token_address
                    )
                )
                self.updated_tokens[token_address] = token
                
        if update_tasks:
            await asyncio.gather(*update_tasks)

    async def _update_token_safe(self, token: LiveERC20Token, transaction: Dict, token_address: str):
        """Safely update a token with transaction data"""
        try:
            await token.update_from_transaction_async(transaction)
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Failed to update token {token_address} for txn {transaction.get('hash')}: {e}") 


class BlockRangeTokenProcessor:
    """
    BlockRangeTokenProcessor: Process historical block ranges for token analysis
    Objective:
        ---------
        1. Process specific ranges of historical blocks for token analysis
        2. Share token state management with live processor
        3. Support warm-up phase for live token tracking
        4. Enable independent historical analysis

        Key Features:
        -----------
        1. Historical Processing:
        - Process specific block ranges
        - Share token cache with live processor
        - Support warm-up for live tracking
        - Enable independent historical analysis

        2. State Sharing:
        - Uses shared BaseTokenProcessor instance
        - Consistent token state across processors
        - Unified cache management
        - Synchronized token updates
   """
    def __init__(self, 
                 block_token_processor: BlockTokenProcessor = None,
                 w3: Web3 = None,
                 logger=None):
        self.logger = logger or get_logger(name="token_manager", log_folder="tokens_live")
        self.w3 = w3 or Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))    
        self.block_processor = BlockProcessor(logger=self.logger)
        self.block_token_processor = block_token_processor or BlockTokenProcessor(
            logger=self.logger
        )
        self.processed_blocks = self.block_token_processor.processed_blocks
    
    async def process_block_range(self, start_block: int, end_block: int):
        """Process block data in sequential order using shared base processor"""
        try:
            for block_number in range(start_block, end_block + 1):
                if block_number not in self.processed_blocks:
                    # Use shared base processor for token processing            
                    block_data = await self.block_processor.process_block(block_number)
                    # Process block data for token updates
                    await self.block_token_processor.process_block(block_data)                    
                    self.block_token_processor.latest_processed_block = block_number

        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing block: {e}")
            raise

    async def process_range_until_live(self, block_range: int):
        """Process blocks in ranges until we're close enough to live"""
        start_block = self.w3.eth.get_block_number() - block_range
        self._has_caught_up_to_live = False
        try:
            current_block = start_block
            while not self._has_caught_up_to_live:
                
                current_block_data = await self.block_processor.process_block(block_number=current_block)
                # Process block data for token updates 
                await self.block_token_processor.process_block(current_block_data)
                self.block_token_processor.latest_processed_block = current_block
                # Check if we're caught up after processing this range
                latest_block = self.w3.eth.get_block_number()
                if latest_block - current_block == 0:
                    self._has_caught_up_to_live = True
                    self.logger.info(f"Caught up to live (gap: {latest_block - current_block} blocks)")
                    break
                current_block += 1
            return self.block_token_processor.latest_processed_block
        except Exception as e:
            self.logger.error(f"Error catching up to live: {e}")
            raise
