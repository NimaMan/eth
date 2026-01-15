"""
BlockTokenProcessor: Central coordinator for live ERC20 token state derived from processed blocks.

Responsibilities
----------------
1. Consume processed block payloads (transactions + header) from upstream block processors and walk
   every transaction sequentially so token mutations match canonical on-chain order.
2. Manage the ERC20 token lifecycle:
   - Detect new ERC-20 deployments, hydrate metadata with `TokenChainDataFetcher`, and instantiate
     `ERC20Token` objects seeded with their creation transaction.
   - Persist new tokens plus their bookkeeping (pools, PnL wiring) inside `LiveTokensCache`.
3. Apply ongoing token updates:
   - For every transaction referencing tracked contracts, propagate changes through token data,
     liquidity network, and health analyzers.
   - Record which tokens changed during the block and refresh cache mappings after processing.
4. Track block-processing context (start/latest block numbers, last two headers) so downstream
   consumers can reason about continuity and previous-block metadata.
5. Serve both historical catch-up (`HistoricalBlockTokenProcessor`) and live streaming
   (`LiveBlockTokenProcessor`) flows through the same stateful engine, keeping shared token state in
   sync while offering a simple synchronous mutation API callable from async workflows.
"""

from web3 import Web3
from dataclasses import asdict, is_dataclass
from typing import Dict, Any, Optional, List
from collections import OrderedDict, defaultdict
from tqdm import tqdm

from eth_token.erc20_token.erc20_token import ERC20Token
from eth_token.token_manager.live_tokens_cache import LiveTokensCache
from eth_token.erc20_token.token_chain_data_fetcher import TokenChainDataFetcher
from eth_token.utils.logger import get_logger
from eth_data.blockchain.block_processor import BlockProcessor


class BlockTokenProcessor:
    def __init__(self, logger=None, add_pnl_to_db: bool = False):
        self.logger = logger
        self.add_pnl_to_db = add_pnl_to_db
        self.live_tokens_cache = LiveTokensCache(logger=self.logger, add_pnl_to_db=add_pnl_to_db)
        self.updated_tokens: Dict[str, ERC20Token] = {}
        self.processed_blocks: Dict[int, bool] = OrderedDict()
        self.latest_processed_block = 0
        self.start_block = None  # Track the first block we process
        self.token_chain_fetcher = TokenChainDataFetcher()
        self._chain_query = self.token_chain_fetcher.chain_query
        self.is_live_mode = False
        # Per-block sender -> tx list index used for intra-block pending simulation
        self._address_tx_index = defaultdict(list)

    def process_block_tokens(self, process_block_result, block_number) -> int:
        """Process a single block's transactions sequentially."""
        block_tx_list = process_block_result.get('transactions')
        self.updated_tokens.clear() # Clear the updated tokens cache
        self._address_tx_index.clear() # Reset per-block address index so pending replay never leaks across blocks
        
        # Ensure transactions mutate token state in canonical block order
        for tx in block_tx_list:
            tx_data = self._ensure_tx_dict(tx)
            self._index_transaction(tx_data) # Index after processing so later txs can replay earlier same-sender txs
            self._process_transaction(tx_data, block_number)
            
        # Set start_block on first block processed
        if self.start_block is None:
            self.start_block = block_number

        self.processed_blocks[block_number] = True # Mark the block as processed
        return block_number

    def _process_transaction(self, transaction: Dict, block_number: int):
        """Process a single transaction and update relevant tokens"""
        try:
            # Handle contract creation
            is_token_creation, token_metadata, contract_address = self._is_token_creation(
                transaction, block_number
            )
            if is_token_creation:
                self._handle_token_creation(transaction, block_number, token_metadata, contract_address)
                return
                
            # Handle regular transactions
            self._handle_token_update_from_transaction(transaction)
                
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Error processing transaction {transaction.get('hash')}: {e}")

    def _is_token_creation(self, transaction: Dict, block_number: int) -> Optional[Dict[str, Any]]:
        """Return token metadata if transaction deploys a new ERC-20 contract."""
        contract_address = transaction.get('contract_address')
        if not contract_address:
            return False, None, None

        try:
            # Replay earlier same-sender txs in this block for metadata hydration.
            pending_transactions = self._collect_address_transactions(transaction.get("from_address"))
            pending_transactions = sorted(pending_transactions, key=lambda x: x["tx_index"])
            simulation_block = block_number - 1
            token_metadata = self.token_chain_fetcher.get_token_metadata(
                contract_address,
                block_number = simulation_block,
                pending_transactions=pending_transactions,
                gas_block_number=block_number,
            )
            if token_metadata is None:
                return False, None, None
            return True, token_metadata, contract_address
        except Exception as exc:
            self.logger.error(f"{self.__class__.__name__} Error getting token metadata for contract {contract_address} in transaction {transaction.get('hash')}: {exc}")
            return False, None, None

    def _handle_token_creation(
        self,
        transaction: Dict,
        block_number: int,
        token_metadata: Optional[Dict[str, Any]] = None,
        contract_address: Optional[str] = None,
    ):
        """Handle creation of a new token"""
        if contract_address not in self.live_tokens_cache:
            try:
                token = ERC20Token(
                    contract_address,
                    token_metadata,
                )
                token.update_from_transaction(transaction)
                self.live_tokens_cache[contract_address] = token
                self.updated_tokens[contract_address] = token
                if self.logger:
                    self.logger.info(f"New token created: {contract_address} in block {block_number}")

            except Exception as e:
                self.logger.error(f"{self.__class__.__name__} Failed to create token {contract_address} at tx {transaction.get('hash')}: {e}")

    def _update_token(self, token: ERC20Token, transaction: Dict, token_address: str):
        """Safely update a token with transaction data"""
        try:
            token.update_from_transaction(transaction)
        except Exception as e:
            self.logger.error(f"{self.__class__.__name__} Failed to update token {token_address} at tx {transaction.get('hash')}: {e}") 

    def _handle_token_update_from_transaction(self, transaction: Dict):
        """Handle transaction involving existing tokens"""
        erc20_contracts = transaction.get('erc20_contracts', set())
        if not erc20_contracts:
            return
            
        for token_address in erc20_contracts:
            token = self.live_tokens_cache[token_address]
            if token:
                self._update_token(
                    token=token,
                    transaction=transaction,
                    token_address=token_address
                )
                self.updated_tokens[token_address] = token
        
        # Update the pool and token mapping so that we know which pools belong to which tokens
        if self.updated_tokens:
            for token in self.updated_tokens.values():
                self.live_tokens_cache.update_pool_mapping(token)

    @staticmethod
    def _ensure_tx_dict(tx: Any) -> Dict:
        if isinstance(tx, dict):
            return tx
        if hasattr(tx, "to_dict"):
            return tx.to_dict()
        if is_dataclass(tx):
            return asdict(tx)
        if hasattr(tx, "__dict__"):
            return dict(vars(tx))
        raise TypeError(f"Unsupported transaction type: {type(tx)!r}")

    def _index_transaction(self, transaction: Dict[str, Any]) -> None:
        addresses = set(transaction.get("unique_addresses") or [])
        for addr in addresses:
            self._address_tx_index[addr].append(transaction)

    def _collect_address_transactions(self, address: Optional[str]) -> Optional[List[Dict[str, Any]]]:
        # Collect the tx of address and all other addresses present in its txs 
        txs = list(self._address_tx_index.get(address))
        if not txs:
            return None
        from_addresses = set([tx["from_address"] for tx in txs])
        tx_hahaes = set([tx["hash"] for tx in txs])
        for from_address in from_addresses:
            if from_address == address:
                continue
            other_address_txs = self._collect_address_transactions(from_address)
            for other_tx in other_address_txs:
                if other_tx["hash"] not in tx_hahaes:
                    txs.append(other_tx)
                    tx_hahaes.add(other_tx["hash"])
        
        return txs



class HistoricalBlockTokenProcessor:
    """
    HistoricalBlockTokenProcessor: Process historical block ranges for token analysis
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
                 logger=None,
                 index_address_txs: bool = False):
        self.logger = logger or get_logger(name="token_manager")
        self.w3 = w3 or Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))    
        self.block_processor = BlockProcessor(logger=self.logger, index_address_txs=index_address_txs)
        self.block_token_processor = block_token_processor or BlockTokenProcessor(
            logger=self.logger
        )
        self.processed_blocks = self.block_token_processor.processed_blocks
    
    async def process_block_range(self, start_block: int, end_block: int):
        """Process block data in sequential order using shared base processor"""
        for block_number in tqdm(range(start_block, end_block + 1), desc="Processing blocks"):
            if block_number not in self.processed_blocks:
                try:
                    # Use shared base processor for token processing
                    processed_block_result = await self.block_processor.process_block(block_number)
                    # Process block data for token updates
                    self.block_token_processor.process_block_tokens(
                        processed_block_result,
                        block_number=block_number
                    )
                    self.block_token_processor.latest_processed_block = block_number
                except Exception as e:
                    self.logger.error(f"{self.__class__.__name__} Error processing block {block_number}: {e}", exc_info=True)
                    raise

    async def process_range_until_live(self, block_range: int):
        """Process blocks in ranges until we're close enough to live"""
        start_block = self.w3.eth.get_block_number() - block_range
        self._has_caught_up_to_live = False
        current_block = start_block
        while not self._has_caught_up_to_live:
            try:
                processed_block_result = await self.block_processor.process_block(block_number=current_block)
                # Process block data for token updates 
                self.block_token_processor.process_block_tokens(
                    processed_block_result,
                    block_number=current_block
                )
                self.block_token_processor.latest_processed_block = current_block
                # Check if we're caught up after processing this range
                latest_block = self.w3.eth.get_block_number()
                if latest_block - current_block == 0:
                    self._has_caught_up_to_live = True
                    self.logger.info(f"Caught up to live (gap: {latest_block - current_block} blocks)")
                    break
                current_block += 1
            except Exception as e:
                self.logger.error(f"Error catching up to live at block {current_block}: {e}")
                raise

        return self.block_token_processor.latest_processed_block
        
