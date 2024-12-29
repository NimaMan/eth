"""
BaseTokenProcessor: Core token processing functionality

Objective:
---------
1. Provide core token processing functionality for both live and historical modes
2. Process blocks and their transactions efficiently

Architecture & Flow:
------------------
1. Core Processing Pipeline:
   - Process blocks and their transactions
   - Create and update token instances
   - Cache token states
"""

import asyncio
from dataclasses import asdict
from typing import Dict, List, Optional
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.services.live_tokens_object_cache import LiveTokenObjectsCache
from eth_tokens_live.services.cache.token_cache_service import TokenCacheService
from eth_tokens_live.utils.logger import get_logger


class BlockTokenProcessor:
    def __init__(self, redis_url: str = "redis://localhost:6379/0", logger=None):
        self.logger = logger or get_logger(name="tokens_live", log_folder="tokens_live")
        
        # Token tracking
        self.live_tokens_cache = LiveTokenObjectsCache(logger=self.logger)
        self.token_first_seen: Dict[str, int] = {}
        self.updated_tokens: Dict[str, LiveERC20Token] = {}
        self.latest_processed_block = 0
        
        # Cache service
        self.cache_service = TokenCacheService(
            redis_url=redis_url,
            batch_size=100,
            flush_interval=1.0,
            logger=self.logger
        )

    async def _process_block(self, block_data: List[Dict]):
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
                
                await self.cache_service.store_token(token)
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
            await self.cache_service.store_token(token)
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Failed to update token {token_address} for tx {transaction.get('hash')}: {e}") 