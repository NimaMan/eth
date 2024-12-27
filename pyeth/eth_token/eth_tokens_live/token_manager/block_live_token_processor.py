"""
BlockLiveTokenProcessor: Processes blocks to maintain live token states

Objective:
---------
1. Subscribe to processed blocks from RabbitMQ
2. Process transactions and update token states
3. Track live tokens and their data
4. Provide access to current token states

Architecture & Flow:
------------------
1. Block Subscription (via RabbitMQ):
   - Connects to 'blocks_exchange' with routing_key='blocks'
   - Receives processed block data containing detailed transactions
   - Adds blocks to internal processing queue
   - Handles reconnection and message persistence

2. Block Processing Pipeline:
   - Async queue consumer processes blocks in order
   - Each block contains list of detailed transactions
   - Transactions are filtered for token-related events:
     * Contract creations
     * ERC20 transfers
     * Pair creations
     * Owner events
     * Trading events

3. Token State Management:
   - Creates new LiveERC20Token instances for detected tokens
   - Updates existing token states with new transaction data
   - Maintains token metadata and metrics
   - Tracks token relationships (pairs, holders)

4. Data Access:
   - Provides methods to query token states
   - Tracks processing statistics
   - Monitors queue health

Message Flow:
-----------
BlockProcessor -> RabbitMQ -> BlockSubscriber -> Queue -> BlockLiveTokenProcessor -> Token Events
                                                         |                     |
                                AlertSubscriber <- RabbitMQ <- AlertProcessor  |
                                                                               v
                                                                          RabbitMQ -> Token Event Consumers

   
Token Events:
-----------
1. Token Creation Events:
   - Create a new LiveERC20Token instance
   - Initialize token data
   - Add to live tokens cache

2. Token Update Events:
   - Update the Live token with the transaction data
"""

import asyncio
from typing import Dict, Set, List
import time
from eth_tokens_live.subscribers.block_subscriber import BlockSubscriber
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.token_manager.live_tokens_object_cache import LiveTokenObjectsCache
from eth_tokens_live.utils.logger import get_logger


class BlockLiveTokenProcessor:
    def __init__(self, rabbitmq_url: str = None, logger=None):
        # Initialize subscribers and publishers
        self.logger = logger
        if logger is None:
            self.logger = get_logger(name="tokens_live", log_folder="tokens_live")
        self.block_subscriber = BlockSubscriber(
            callback=self.process_block,
            rabbitmq_url=rabbitmq_url,
            logger=logger
        )
        
        # Live token tracking
        self.live_tokens_cache = LiveTokenObjectsCache(logger=self.logger)
        self.token_first_seen: Dict[str, int] = {}
        self.updated_tokens: Dict[str, LiveERC20Token] = {}
        self.latest_processed_block = 0
        self.block_processed_event = asyncio.Event()

    async def process_block(self, block_data: List[Dict]):
        """
        Process block data with concurrent transaction processing        
        1. Create tasks for all transactions in block
        2. Process transactions concurrently
        3. Wait for all transactions to complete
        4. Update latest processed block
        5. Signal that block processing is complete
        """
        start_time = time.time()
        block_number = block_data[0]['block_number']
        self.updated_tokens.clear()
        # Create tasks for all transactions
        tasks = [
            self._process_transaction(txn) 
            for txn in block_data
        ]
        
        # Wait for all transactions to be processed
        await asyncio.gather(*tasks)
        
        # Update latest processed block
        self.latest_processed_block = max(self.latest_processed_block, block_number)
        # Signal that block processing is complete
        self.block_processed_event.set()
        self.logger.info(f"Block {block_number} processed. Number of tokens: {len(self.live_tokens_cache)}. Time taken: {time.time() - start_time:.2f} seconds")
        
    async def _process_transaction(self, transaction: Dict):
        """
        Process a single transaction and update relevant tokens
        1. For Contract Creation:
           - Create new LiveERC20Token instance
           - Initialize token data
           - Add to live tokens cache
        
        2. For Other Transactions:
           - Extract relevant token addresses
           - If token is in cache, update it. if not, skip
        """
        try:
            block_number = transaction.get('block_number')  
            # Handle contract creation
            if transaction.get('txn_type') == 'Contract Creation' and transaction.get('contract_address'):
                new_token_address = transaction['contract_address']
                if len(transaction['contract_creation_events']) > 0:
                    contract_creation_event = transaction['contract_creation_events'][0]
                    if contract_creation_event["contract_type"] == "ERC-20":
                        if new_token_address not in self.live_tokens_cache:
                            try:
                                token = LiveERC20Token(new_token_address)
                                token.update_from_transaction(transaction)
                                self.live_tokens_cache[new_token_address] = token
                                self.token_first_seen[new_token_address] = block_number 
                                self.logger.info(f"New token created: {new_token_address} in block {block_number}")
                                self.updated_tokens[new_token_address] = token
                            except Exception as e:
                                self.logger.error(f"Failed to create token {new_token_address}: {e}")
                return
            # Handle regular transactions
            erc20_contracts = transaction.get('erc20_contracts', set())
            if not erc20_contracts:
                return
            # Prepare update tasks for existing tokens
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
            # Execute all updates concurrently
            if update_tasks:
                await asyncio.gather(*update_tasks)

        except Exception as e:
            self.logger.error(f"{__name__}: Error processing transaction {transaction.get('hash')}: {e}")

    async def _update_token_safe(self, token: LiveERC20Token, transaction: Dict, token_address: str) -> None:
        """
        Safely update a token with transaction data
        
        Wraps token updates in error handling to prevent individual failures
        from affecting other token updates
        """
        try:
            await token.update_from_transaction_async(transaction)
        except Exception as e:
            self.logger.error(f" {__name__} Failed to update token {token_address} for tx {transaction.get('hash')}: {e}")