"""
BlockTokenProcessor: Core token processing functionality

Objective:
---------
1. Provide core token processing functionality for both live and historical modes
    - Process blocks and their transactions
    - Create and update token instances
    - Cache token states
2. Process blocks and their transactions efficiently
"""

import asyncio
from dataclasses import asdict
from typing import Dict, List, Optional

from eth_block_processor.blockchain.block_processor import BlockProcessor
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.services.live_tokens_object_cache import LiveTokenObjectsCache
from eth_tokens_live.utils.logger import get_logger


class BlockTokenProcessor:
    def __init__(self, redis_url: str = "redis://localhost:6379/0", logger=None):
        self.logger = logger or get_logger(name="tokens_live", log_folder="tokens_live")
        
        # Token tracking
        self.live_tokens_cache = LiveTokenObjectsCache(logger=self.logger, redis_url=redis_url)
        self.token_first_seen: Dict[str, int] = {}
        self.updated_tokens: Dict[str, LiveERC20Token] = {}
        self.latest_processed_block = 0

    async def process_block(self, block_data: List[Dict]):
        """Process a single block's transactions"""
        if not block_data:
            return    
        
        self.updated_tokens.clear()
        tasks = [self._process_transaction(asdict(txn)) for txn in block_data]
        await asyncio.gather(*tasks)

    async def _process_transaction(self, transaction: Dict):
        """Process a single transaction and update relevant tokens"""
        try:
            block_number = transaction.get('block_number')
            
            # Handle contract creation
            if self._is_token_creation(transaction):
                await self._handle_token_creation(transaction, block_number)
                return
                
            # Handle regular transactions
            await self._handle_token_transaction(transaction)
                
        except Exception as e:
            self.logger.error(f"Error processing transaction {transaction.get('hash')}: {e}")

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
                self.logger.error(f"Failed to create token {new_token_address}: {e}")

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
            self.logger.error(f"{self.__class__.__name__} Failed to update token {token_address} for tx {transaction.get('hash')}: {e}") 


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
                 logger=None):
        self.logger = logger or get_logger(name="tokens_live", log_folder="tokens_live")
        self.block_processor = BlockProcessor(logger=self.logger)
        self.block_token_processor = block_token_processor or BlockTokenProcessor(
            logger=self.logger
        )
        self.processed_blocks: Dict[int, List[Dict]] = {}
    
    async def fetch_block_range_data(self, start_block: int, end_block: int):
        """Fetch and return block data for a specific range"""
        try:
            block_range_detailed_transaction_data = await self.block_processor.process_block_range(
                start_block=start_block,
                end_block=end_block
            )   
            return block_range_detailed_transaction_data                 
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing blocks {start_block}-{end_block}: {e}")
            raise

    async def process_blocks(self, block_range_detailed_transaction_data: Dict[int, List[Dict]]):
        """Process block data in sequential order using shared base processor"""
        try:
            for block_number, block_data in sorted(block_range_detailed_transaction_data.items()):
                if block_number not in self.processed_blocks:
                    # Use shared base processor for token processing
                    await self.block_token_processor.process_block(block_data)
                    self.processed_blocks[block_number] = True
                    self.block_token_processor.latest_processed_block = max(
                        self.block_token_processor.latest_processed_block, 
                        block_number
                    )
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing block: {e}")
            raise

