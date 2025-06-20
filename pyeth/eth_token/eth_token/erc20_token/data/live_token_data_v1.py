"""
LiveTokenData: Real-time Token Data Management and Analytics
=============================================================

Objective:
----------
To provide a comprehensive, streamlined data structure for managing real-time data
for ERC20 tokens. This class serves as the central orchestrator, integrating static
token metadata with dynamic on-chain events processed through a modern, reliable
pool management system.

Main Concepts:
--------------
1. **Core Token Information:** Manages contract address, name, symbol, decimals, and total supply.
2. **Creation Event Handling:** Captures initial contract creation details.
3. **Trading Status:** Monitors when trading is enabled via on-chain events.
4. **Event Aggregation:** Collects ERC20 transfers, ETH transfers, and approvals.
5. **Modern Pool Management:** Delegates all liquidity pool logic (V2, V3, V4) to the `PoolManager` class, which acts as the single source of truth for pool state, reserves, and prices.
6. **Scam Detection:** Utilizes the `PoolReserveTracker` to label tokens based on low-liquidity metrics derived from the `PoolManager`'s data.

Usage:
------
1. **Instantiation:** Create a `LiveTokenData` object with the token's contract address.
2. **Event Updates:** Call `update_from_transaction()` with a `ProcessedTransaction` object to update the token's state.
3. **Data Retrieval:** Access properties like `pool_manager`, `reserve_tracker`, and event lists for analysis.

"""
from datetime import datetime
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional
from enum import Enum

from eth_token.utils.common_addresses import DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, DENOM_NAMES_TO_ADDRESS
from eth_block_processor.address.contract_type import get_erc20_contract_info
from .pools import PoolManager, PoolReserveTracker
from eth_token.utils.logger import get_logger


WETH_DENOM_RESERVE_THRESHOLD = 1e-2
USD_DENOM_RESERVE_THRESHOLD = 100
HIDDEN_MINTS_THRESHOLD = 1+1e-2


class TokenStatusEnum(Enum):
    CREATION = "CONTRACT_CREATION"
    PAIR_CREATION = "PAIR_CREATION"
    TRADING_ENABLED = "TRADING_ENABLED"
    INACTIVE_SCAM = "INACTIVE_SCAM"
    INACTIVE_OTHER = "INACTIVE_OTHER"


@dataclass
class LiveTokenData:
    contract_address: str  
    txn_hashes_to_makers: Dict[str, str] = field(default_factory=dict) # key: txn_hash, value: fee source

    # Core token info
    name: Optional[str] = None
    symbol: Optional[str] = None
    decimals: Optional[int] = None
    total_supply: Optional[int] = 0
    total_supply_from_transfers: Optional[int] = 0
    
    # Creation info
    creation_block: Optional[int] = None
    creation_timestamp: Optional[datetime] = None
    creation_txn: Optional[str] = None
    creator_address: Optional[str] = None
    creator_nonce: Optional[int] = None
    
    # Contract state
    trading_enabled: bool = False
    trading_enabled_block: Optional[int] = None
    trading_enabled_timestamp: Optional[datetime] = None
    trading_enabled_txn: Optional[str] = None
    token_status: Optional[TokenStatusEnum] = None

    # Event collections with proper typing
    erc20_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: txn_hash, value: list of erc20 transfers
    eth_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: txn_hash, value: list of eth transfers
    liquidity_token_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: txn_hash, value: list of liquidity token transfers
    other_denom_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: txn_hash, value: list of other denom transfers
    
    approvals: List[Dict] = field(default_factory=list)
    other_currencies: Dict[str, int] = field(default_factory=dict) # {Currency: Number of transfers}
    # Liquidity token info # TODO: needs a similar structure as pool_info
    liquidity_token_supply: Optional[float] = 0
    liquidity_token_supply_from_transfers: Optional[float] = 0

    # Ownership info
    owner_events: List[Dict] = field(default_factory=list)
    current_owner: Optional[str] = None
    all_owners: List[str] = field(default_factory=list)
    
    # Transaction fees
    transaction_fees: List[Dict] = field(default_factory=list)
    
    approved_addresses: Set[str] = field(default_factory=set)
    address_tx_counter: Dict[str, int] = field(default_factory=dict)
    total_bribe_amount: float = 0
    bribe_amount_dict: Dict[str, float] = field(default_factory=dict)

    latest_block_number: Optional[int] = None
    latest_block_timestamp: Optional[datetime] = None

    is_scam: bool = False
    scam_label: Optional[str] = None
    scam_block: Optional[int] = None
    scam_txn: Optional[str] = None
    
    # Modern, unified pool management system (initialized in __post_init__)
    pool_manager: Optional[PoolManager] = field(default=None, init=False)
    reserve_tracker: Optional[PoolReserveTracker] = field(default=None, init=False)
    
    def __post_init__(self):
        """Initialize pool management system upon first use."""
        self.logger = get_logger("live_tokens", log_folder="token_manager")
    
    def to_dict(self):
        return self.__dict__
    
    @property
    def fee_sources(self):
        """Get the fee sources"""
        return list(set(self.txn_hashes_to_makers.values()))
    
    @property
    def txn_hashes(self):
        """Get the txn hashes"""
        return list(set(self.txn_hashes_to_makers.keys()))
    
    @property
    def unique_addresses(self):
        """Get the unique addresses"""
        return list(self.address_tx_counter.keys())
    
    def get_pool_denom_currency(self, pool_address: str):
        """Get the denom currency for a given pool address"""
        # PoolManager is the single source of truth.
        if self.pool_manager:
            pool = self.pool_manager.get_pool(pool_address)
            return pool.get_denom_currency_name() if pool else "Unknown"
        return "Unknown"

    @property
    def denom_currency(self):
        """Get the denomination currencies of all pools, comma-separated if multiple."""
        if self.pool_manager:
            denom_currencies = self.pool_manager.get_denom_currencies()
            if denom_currencies:
                return ", ".join(sorted(list(set(denom_currencies.values()))))
        return None
    
    @property
    def pool_addresses(self):
        """Get a tuple of all managed pool addresses."""
        if self.pool_manager:
            return tuple(self.pool_manager.get_all_pool_addresses())
        return tuple()
    
    @property
    def has_pool(self):
        """Check if the token has any managed liquidity pools."""
        return self.pool_manager.has_pools() if self.pool_manager else False
    
    @property
    def has_uni_v2_pool(self) -> bool:
        """Check if token has any Uniswap V2 pools."""
        return self.pool_manager.has_v2_pools() if self.pool_manager else False
    
    @property
    def has_uni_v3_pool(self) -> bool:
        """Check if token has any Uniswap V3 pools."""
        return self.pool_manager.has_v3_pools() if self.pool_manager else False
    
    @property
    def has_uni_v4_pool(self) -> bool:
        """Check if token has any Uniswap V4 pools."""
        return self.pool_manager.has_v4_pools() if self.pool_manager else False
    
    @property
    def latest_pools_price_ratio(self):
        """
        Get the current price ratio relative to the initial price for each pool.
        Delegates calculation to the PoolReserveTracker.
        """
        price_ratios = {}
        if self.is_scam or not self.pool_manager or not self.reserve_tracker:
            return price_ratios
            
        for pool_address in self.pool_manager.get_all_pool_addresses():
            ratio = self.reserve_tracker.get_price_ratio_to_initial(pool_address)
            price_ratios[pool_address] = ratio if ratio is not None else 0.0
            
        return price_ratios
    
    @property
    def average_bribe_amount(self):
        """Get the average bribe amount"""
        return sum(self.bribe_amount_dict.values())/len(self.bribe_amount_dict)
    
    @property
    def num_bribes(self):
        """Get the bribes frequency"""
        from collections import Counter
        counter = Counter(self.bribe_amount_dict.values())
        return f"({', '.join([f'{c}:{v:.3f}' for v, c in counter.items()])})"

    def _handle_creation(self, transaction: Dict):
        """Process contract creation event"""
        self.contract_address = transaction['contract_address']
        self.creation_block = transaction['block_number']
        self.creation_timestamp = transaction['block_timestamp']
        self.creation_txn = transaction['hash']
        self.creator_address = transaction['from_address']
        self.creator_nonce = transaction['nonce']
        # Set initial owner
        self.current_owner = transaction['from_address']
        contract_creation_event = transaction['contract_creation_events'][0]
        self.name = contract_creation_event['name']
        self.symbol = contract_creation_event['symbol']
        self.decimals = contract_creation_event['decimals']
        self.total_supply = contract_creation_event['total_supply']
        self.token_status = TokenStatusEnum.CREATION
    
    def get_pool_info_dict(self) -> Dict[str, Dict]:
        """
        Get pool information dictionary. Sourced directly from PoolManager.
        
        Returns pool_info in the format:
        {pool_address: {pool_type, denom_currency, decimals, token1_is_denom}}
        """
        return self.pool_manager.get_pool_info() if self.pool_manager else {}
    
    def get_pool_token_reserve(self, pool_address: str):
        """Get the token reserve for a specific pool directly from the PoolManager."""
        if self.pool_manager:
            pool = self.pool_manager.get_pool(pool_address)
            if pool:
                return pool.get_token_reserve()
        return None
    
    def get_pool_reserve(self, pool_address: str):
        """Get the ETH/denomination reserve for a specific pool."""
        if self.pool_manager:
            pool = self.pool_manager.get_pool(pool_address)
            if pool:
                return pool.get_denom_reserve()
        return None
    
    def get_all_pools(self):
        """Get all pools including V4 pools."""
        if self.pool_manager:
            return self.pool_manager.get_all_pools()
        return []

    def _update_trading_enabled(self, transaction: Dict):
        """Update the trading enabled status"""
        self.trading_enabled = True
        self.trading_enabled_block = transaction['block_number']
        self.trading_enabled_timestamp = transaction['block_timestamp']
        self.trading_enabled_txn = transaction['hash']
        self.trading_enabled_event_index = transaction['txn_index']
        self.token_status = TokenStatusEnum.TRADING_ENABLED
            
    def _add_trading_enabled_event(self, transaction: Dict):
        """Process a trading enabled event"""
        for trading_enabled_event in transaction.get('trading_enabled_events', []):
            self.trading_enabled_event = trading_enabled_event
            if not self.trading_enabled:
                self._update_trading_enabled(transaction)
    
    def _add_erc20_transfer(self, transaction: Dict, transfer: Dict):
        """Process a transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        amount = int(transfer['amount'])/10**self.decimals
        if txn_hash not in self.erc20_transfers.keys():
            self.erc20_transfers[txn_hash] = []
        transfer_dict = {
            'txn_hash': txn_hash,
            'block_number': block_number,
            'txn_index': txn_index,
            'log_index': transfer['log_index'],
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': amount,
            'token_address': transfer['token_address']
        }
        self.erc20_transfers[txn_hash].append(transfer_dict)
        if transfer['from_address'] == '0x0000000000000000000000000000000000000000':
            self.total_supply_from_transfers += amount
        
        # Update pool reserves if pool is the sender
        if transfer['from_address'] in self.pool_addresses:
            self.update_pool_reserves(transfer['from_address'], -amount, "token")
        # Update pool reserves if pool is the recipient
        if transfer['to_address'] in self.pool_addresses:
            self.update_pool_reserves(transfer['to_address'], amount, "token")

    def _update_eth_pool_reserves(self, transfer_dict: Dict):
        """(DEPRECATED) This logic is now fully handled by the PoolManager."""
        pass

    def _add_weth_transfer(self, transaction: Dict, transfer: Dict):
        """Process an ETH transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        if txn_hash not in self.eth_transfers.keys():
            self.eth_transfers[txn_hash] = []
        transfer_dict = {
            'txn_hash': txn_hash,
            'block_number': block_number,
            'txn_index': txn_index,
            'log_index': transfer['log_index'],
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': float(transfer['amount'])/10**18,
            'token_address': "WETH"
        }
        self.eth_transfers[txn_hash].append(transfer_dict)
        if transfer_dict['token_address'] in self.pool_addresses:
            self.update_pool_reserves(transfer_dict['token_address'], transfer_dict['amount'], "denom")

        self._update_eth_pool_reserves(transfer_dict)
        
    def _add_eth_transfer(self, transaction: Dict):
        """Process an internal transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for transfer in transaction.get('internal_transactions', []):
            if txn_hash not in self.eth_transfers.keys():
                self.eth_transfers[txn_hash] = []
            transfer_dict = {
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'depth': transfer['depth'],
                'from_address': transfer['from_address'],
                'to_address': transfer['to_address'],
                'amount': float(transfer['value']),
                'token_address': "ETH"
            }
            self.eth_transfers[txn_hash].append(transfer_dict)
            if transfer_dict['token_address'] in self.pool_addresses:
                self.update_pool_reserves(transfer_dict['token_address'], transfer_dict['amount'], "denom")

            self._update_eth_pool_reserves(transfer_dict)

    def _add_other_token_transfer(self, transaction: Dict, transfer: Dict):
        """Process transfers of other known tokens (USDC, USDT, etc.)"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        if transfer['token_address'] in DENOM_ADDRESSES.keys():
            denom_name = DENOM_ADDRESSES[transfer['token_address']]
            denom_decimals = ERC20_TOKEN_DECIMALS[denom_name]
            amount = float(transfer['amount'])/10**denom_decimals
            if denom_name not in self.other_currencies:
                self.other_currencies[denom_name] = 0
            self.other_currencies[denom_name] += 1
        
            if txn_hash not in self.other_denom_transfers.keys():
                self.other_denom_transfers[txn_hash] = []
            transfer_dict = {
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': transfer['log_index'],
                'from_address': transfer['from_address'],
                'to_address': transfer['to_address'],
                'amount': amount,
                'token_address': transfer['token_address'],
            }
            self.other_denom_transfers[txn_hash].append(transfer_dict)

            if transfer_dict['from_address'] in self.pool_addresses:
                pool_address = transfer_dict['from_address']
                if transfer_dict['token_address'] == self.pool_info[pool_address]['denom_address']:
                    self.update_pool_reserves(pool_address, -amount, "denom")
            if transfer_dict['to_address'] in self.pool_addresses:
                pool_address = transfer_dict['to_address']
                if transfer_dict['token_address'] == self.pool_info[pool_address]['denom_address']:
                    self.update_pool_reserves(pool_address, amount, "denom")
     
    def _add_liquidity_token_transfer(self, transaction: Dict, transfer: Dict):
        """Process a liquidity token transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        if self.pool_info[transfer['token_address']]['lp_token_info'] is not None:
            decimals = 10**self.pool_info[transfer['token_address']]['lp_token_info']['decimals']
        else:
            decimals = None
        
        if txn_hash not in self.liquidity_token_transfers.keys():
            self.liquidity_token_transfers[txn_hash] = []
        self.liquidity_token_transfers[txn_hash].append({
            'txn_hash': txn_hash,
            'block_number': block_number,
            'txn_index': txn_index,
            'log_index': transfer['log_index'],
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': float(transfer['amount'])/decimals if decimals is not None else transfer['amount'], 
            'token_address': transfer['token_address']
            })
        self.liquidity_token_supply_from_transfers += float(transfer['amount'])/decimals if decimals is not None else transfer['amount']

    def _add_transfers(self, transaction: Dict):
        """Process transfer events for all relevant tokens"""
        # Track all transfers in the transaction
        for transfer in transaction.get('erc20_transfers', []):
            try:
                token_address = transfer['token_address']
                
                if token_address == self.contract_address:
                    # Our token transfers
                    self._add_erc20_transfer(transaction, transfer)
                    
                elif token_address in self.pool_addresses:
                    # LP token transfers
                    self._add_liquidity_token_transfer(transaction, transfer)
                    
                elif token_address == DENOM_NAMES_TO_ADDRESS['WETH']:
                    # WETH transfers
                    self._add_weth_transfer(transaction, transfer)
                    
                elif token_address in DENOM_NAMES_TO_ADDRESS.values():
                    # Other known token transfers (USDC, USDT, etc.)
                    self._add_other_token_transfer(transaction, transfer)      
            except Exception as e:
                raise Exception(f" {__name__} Error processing transfer {token_address}, txn {transaction['hash']}: {e}")

    def _add_approval(self, transaction: Dict):
        """Process an approval event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for approval in transaction.get('approvals', []):
            self.approvals.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': approval['log_index'],
                'owner': approval['owner'],
                'spender': approval.get('spender') or approval.get('approved_address'),
                #'amount': float(approval['amount'])/10**self.decimals,
                'token_address': approval['token_address']
            })
            self.approved_addresses.add(approval.get('spender') or approval.get('approved_address'))

    def _add_owner_event(self, transaction: Dict):
        """Process an owner event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for owner_event in transaction.get('owner_events', []):
            self.all_owners.append(owner_event['new_owner'])
            self.current_owner = owner_event['new_owner']
            if owner_event['previous_owner'] == '0x0000000000000000000000000000000000000000':
                self.ownership_renounced = True
                self.ownership_renounced_block = block_number
                self.ownership_renounced_txn = txn_hash

            self.owner_events.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': owner_event['log_index'],
                'previous_owner': owner_event['previous_owner'],
                'new_owner': owner_event['new_owner'],
                'token_address': owner_event['contract_address']
            })

    def _add_renouncement_event(self, transaction: Dict):
        """Process a renouncement event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        
        for renouncement_event in transaction.get('renouncement_events', []):
            self.renouncement_block = block_number
            self.renouncement_txn = txn_hash
            self.renouncement_event = renouncement_event
            self.renouncement_event_index = txn_index
            self.renouncement_event_log_index = renouncement_event['log_index']
        
    def _add_tax_event(self, transaction: Dict):
        """Process a tax event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for tax_event in transaction.get('tax_events', []):
            self.tax_event = tax_event
            self.tax_event_block = block_number
            self.tax_event_txn = txn_hash
            self.tax_event_index = txn_index
            self.tax_event_log_index = tax_event['log_index']

    def _add_max_buy_limit_event(self, transaction: Dict):
        """Process a max buy limit event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        
        for max_buy_limit_event in transaction.get('max_buy_limit_events', []):
            self.max_buy_limit = max_buy_limit_event['max_buy_limit']
            self.max_buy_limit_block = block_number
            self.max_buy_limit_txn = txn_hash
            self.max_buy_limit_index = txn_index
            self.max_buy_limit_log_index = max_buy_limit_event['log_index']

    def _add_max_buy_ratio_event(self, transaction: Dict):
        """Process a max buy ratio event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        
        for max_buy_ratio_event in transaction.get('max_buy_ratio_events', []):
            self.max_buy_ratio = max_buy_ratio_event['max_buy_ratio']
            self.max_buy_ratio_block = block_number
            self.max_buy_ratio_txn = txn_hash
            self.max_buy_ratio_index = txn_index
            self.max_buy_ratio_log_index = max_buy_ratio_event['log_index']

    def _process_other_events(self, transaction: Dict):
        """Process other relevant events"""
        # Process trading enabled events
        for event in transaction.get('trading_enabled_events', []):
            if event['token_address'] == self.contract_address:
                self.trading_enabled = True
                self.trading_enabled_block = transaction['block_number']
                self.trading_enabled_txn = transaction['hash']
    
    def _update_bribe_amount(self, transaction: Dict):
        """Update the bribe amount"""
        bribe_amount = transaction['bribe_amount']
        if bribe_amount > 0:
            briber_address = transaction['from_address']
            self.bribe_amount_dict[briber_address] = bribe_amount
            self.total_bribe_amount += bribe_amount
    
    def _update_pool_status_from_manager(self):
        """Update token status based on pool manager state."""
        if not self.pool_manager:
            return
            
        # Update token status if we have pools but not trading enabled
        if self.has_pool and self.token_status != TokenStatusEnum.TRADING_ENABLED:
            self.token_status = TokenStatusEnum.PAIR_CREATION
            
    def _initialize_pool_manager(self):
        """Initialize the PoolManager and PoolReserveTracker"""
        from .pools.pool_manager import PoolManager
        from .pools.pool_reserve_tracker import PoolReserveTracker
        
        if not hasattr(self, 'logger'):
            self.logger = get_logger("live_tokens", log_folder="token_manager")
        
        # Initialize PoolManager
        self.pool_manager = PoolManager(
            token_address=self.contract_address,
            logger=self.logger
        )
        
        # Initialize PoolReserveTracker for scam detection
        self.reserve_tracker = PoolReserveTracker(
            token_address=self.contract_address,
            logger=self.logger
        )
   
    def updated_address_tx_counter(self, unique_addresses: set):
        """Update the transaction counter for a unique address"""
        for unique_address in unique_addresses:
            if unique_address not in self.address_tx_counter:
                self.address_tx_counter[unique_address] = 0
            self.address_tx_counter[unique_address] += 1

    def update_from_transaction(self, transaction: Dict):
        """Update token data from a new transaction"""
        # Add the txn hash to the list of processed transactions
        self.txn_hashes_to_makers[transaction['hash']] = transaction['from_address']

        # Handle contract creation
        if transaction['txn_type'] == 'Contract Creation':
            if len(transaction['contract_creation_events']) > 0:
                self._handle_creation(transaction)
        if self.total_supply==0:
            return
        
        # Initialize PoolManager on first transaction if not already done
        if not self.pool_manager:
            self._initialize_pool_manager()
        
        # Convert transaction dict to ProcessedTransaction if needed
        if isinstance(transaction, dict):
            # For backward compatibility, we'll create a simple object
            # that mimics ProcessedTransaction structure
            processed_tx = type('ProcessedTransaction', (), transaction)()
            # Add missing attributes if they don't exist
            # Map event names from LiveTokenData format to ProcessedTransaction format
            event_mappings = {
                'pair_events': 'pair_events',
                'uniswap_v3_pools': 'uniswap_v3_pools', 
                'uniswap_v4_initializes': 'uniswap_v4_initializes',
                'uniswap_v2_syncs': 'univ2_syncs',  # Map from live token data name
                'uniswap_v2_swaps': 'univ2_swaps',
                'uniswap_v3_swaps': 'univ3_swaps',
                'uniswap_v3_mints': 'univ3_mints', 
                'uniswap_v3_burns': 'univ3_burns',
                'uniswap_v3_collects': 'univ3_collects',
                'uniswap_v4_swaps': 'univ4_swaps',
                'uniswap_v4_modifies': 'univ4_modifies',
                'uniswap_v4_donates': 'univ4_donates',
                'uniswap_v4_hook_events': 'univ4_hook_events'
            }
            
            # Special handling for generic mints/burns from ProcessedTransaction
            # ProcessedTransaction has generic 'mints' and 'burns' that need to be distributed
            if isinstance(transaction, dict):
                # For dict transactions, copy all attributes
                for key, value in transaction.items():
                    setattr(processed_tx, key, value)
                # Handle generic mints/burns if present
                if 'mints' in transaction:
                    processed_tx.univ2_mints = transaction['mints']
                if 'burns' in transaction:
                    processed_tx.univ2_burns = transaction['burns']
            else:
                # For object transactions
                if hasattr(transaction, 'mints') and transaction.mints:
                    processed_tx.univ2_mints = transaction.mints
                if hasattr(transaction, 'burns') and transaction.burns:
                    processed_tx.univ2_burns = transaction.burns
            
            for processed_name, live_name in event_mappings.items():
                if not hasattr(processed_tx, processed_name):
                    setattr(processed_tx, processed_name, transaction.get(live_name, transaction.get(processed_name, [])))
        else:
            processed_tx = transaction
        
        # Initialize PoolManager on first transaction if not already done
        if not self.pool_manager:
            self._initialize_pool_manager()
        
        # Modern: Use PoolManager to process all pool-related events
        self.pool_manager.process_transaction(processed_tx)
            
        # Update reserve tracker with latest pool states
        if self.reserve_tracker:
            for pool in self.pool_manager.get_all_pools():
                # Skip pools that don't have a valid state yet
                if not hasattr(pool, 'state') or pool.state.last_update_block is None:
                    continue

                denom_reserve = pool.get_denom_reserve()
                token_reserve = pool.get_token_reserve()
                price = pool.get_price()
                
                if denom_reserve > 0 or token_reserve > 0:
                    self.reserve_tracker.update_reserves(
                        pool.pool_address,
                        pool.denom_address,
                        denom_reserve,
                        token_reserve,
                        price,
                        processed_tx.block_number,
                        int(processed_tx.block_timestamp.timestamp()) if hasattr(processed_tx.block_timestamp, 'timestamp') else processed_tx.block_timestamp,
                        processed_tx.hash
                    )
                    
                # Check for scam detection from reserve tracker
                if self.reserve_tracker.is_scam and not self.is_scam:
                    self.is_scam = True
                    self.scam_label = self.reserve_tracker.scam_reason
                    self.scam_block = self.reserve_tracker.scam_block
                    self.scam_txn = self.reserve_tracker.scam_tx_hash
                    self.token_status = TokenStatusEnum.INACTIVE_SCAM
        
        # Process ERC20 transfers
        self._add_transfers(transaction)
        
        # Process internal transfers
        self._add_eth_transfer(transaction)

        # Check the pool reserves (legacy - now handled by PoolManager)
        # self._check_pool_reserves(transaction)  # Commented out - handled by PoolManager

        # Process approvals
        self._add_approval(transaction)

        # Process owner events
        self._add_owner_event(transaction)

        # Update address tx counter
        self.updated_address_tx_counter(transaction.get('unique_addresses', set()))

        # Update bribe amount
        self._update_bribe_amount(transaction)

        # Update pool status based on pool manager's state
        self._update_pool_status_from_manager()
        
        # Process trading enabled events
        if self.has_pool or not self.trading_enabled:
            self._add_trading_enabled_event(transaction)

        # Update latest block number and timestamp
        self.latest_block_number = transaction['block_number']
        self.latest_block_timestamp = transaction['block_timestamp']

