"""
ERC20TokenData: Real-time Token Data Management and Analytics
=============================================================

Objective:
----------
To provide a comprehensive data structure and associated methods for managing, processing, and analyzing real-time token data for ERC20 tokens on Ethereum. This module is designed to capture both static token details and dynamic, event-based data from on-chain activities, facilitating real-time analytics, historical analysis, and integration with decentralized exchange (DEX) pool data.

Main Concepts:
--------------
1. **Token Identification and Core Information**  
   - **Contract Address, Name, and Symbol:**  
     Identifies the token and includes basic metadata such as name and symbol.
   - **Decimals and Total Supply:**  
     Stores token precision and the total supply, forming the basis for all value calculations.

2. **Token Creation and Setup**  
   - **Creation Event Handling:**  
     Captures contract creation details like block number, timestamp, transaction hash, and creator details.
   - **Initial Ownership:**  
     Initializes and tracks token ownership from the moment of creation.

3. **Trading Status and Event Tracking**  
   - **Trading Enabled State:**  
     Monitors when trading is enabled via specific on-chain events, recording the corresponding block, 
     transaction hash, and event index.
   - **State Transitions and Updates:**  
     Continuously updates the token's state (e.g., TRADING_ENABLED, PAIR_CREATION) based on observed events.

4. **Event Aggregation**  
   - **ERC20 Transfers & Internal ETH Transfers:**  
     Aggregates ERC20 token transfers and direct ETH transfers (internal transactions) for comprehensive tracking.
   - **Approvals and Allowances:**  
     Captures approval events to monitor token spending rights and related on-chain allowances.
   - **Liquidity Events:**  
     Records liquidity-related events from DEX platforms (Uniswap V2/V3/V4), including swaps, mints, burns, 
     and sync events.
   - **Other Token Transfers:**  
     Processes transfers of associated tokens (like USDC, USDT) that are part of broader token ecosystem interactions.

5. **DEX Pool and Liquidity Management**  
   - **Pool Information and Price Data:**  
     Manages liquidity pool details including pool type (V2, V3, V4), associated denomination currency, and decimals.
   - **Price Ratio Calculations:**  
     Calculates current and historical price ratios from pool sync events for real-time valuation.
   - **Liquidity Token Analytics:**  
     Tracks liquidity token transfers, mints, and burns, enabling liquidity supply analysis.

6. **Ownership and Approval Management**  
   - **Owner Tracking:**  
     Aggregates ownership change events and identifies the current token owner, while maintaining a history of all owners.
   - **Approval Data:**  
     Stores token approval events for subsequent approval synchronization and allowance management.

7. **Bribe and Scam Detection Mechanisms**  
   - **Bribe Tracking:**  
     Records bribe transactions, computes total and average bribe amounts, and maintains a dictionary for 
     per-address bribe statistics.
   - **Scam Detection:**  
     Implements checks (e.g., on reserve ratios and hidden mint indicators) to flag potential scam events 
     based on liquidity anomalies.

8. **Data Access and Utility Properties**  
   - **Pandas DataFrames:**  
     Provides properties that convert aggregated event data (e.g., transfers, approvals) into Pandas DataFrames 
     for further analysis and reporting.
   - **Convenience Properties:**  
     Exposes key details such as pool addresses, denomination currencies, and latest price ratios via properties 
     for easy access.

Usage:
------
1. **Instantiation:**  
   Create a LiveTokenData object with the token's contract address to initialize token state tracking.

2. **Event Updates:**  
   As new blockchain transactions occur, call `update_from_transaction(transaction_dict)` with the transaction 
   data to update the LiveTokenData object's internal state.

3. **Data Retrieval:**  
   Retrieve up-to-date views:
      - `live_token_data.erc20_transfer_df` for transfer history.
      - `live_token_data.latest_pools_price_ratio` for current pool price ratios.
      - Other properties such as `denom_currency`, `pool_addresses`, and `unique_addresses` for additional insights.

Dependency Overview:
--------------------
- **dataclasses:** For structuring token data as a Python data class.
- **datetime:** For handling timestamps and date-related operations.
- **pandas:** For DataFrame-based analysis of token events.
- **enum:** For standardized token status codes (e.g., TRADING_ENABLED, INACTIVE_SCAM).
- **Custom Modules:**  
  - Token approval synchronization data (for processing approval events).  
  - Contract information utilities (to fetch on-chain metadata).

Implementation Notes:
---------------------
- **Event-Driven Updates:**  
  Each method prefixed with `_add_` processes a specific type of event (transfer, sync, swap, etc.), 
  appending the event data to internal lists.
- **Data Integrity:**  
  Methods assume correctly structured transaction data. Exceptions are raised for critical errors 
  during event processing, ensuring high data fidelity.
- **Liquidity and Scam Analysis:**  
  Price ratios and liquidity pool events are continuously monitored to detect anomalies, such as hidden 
  mints or drastic reserve changes, for potential scam identification.
- **Modularity:**  
  The LiveTokenData class is designed to be modular, allowing easy integration with additional data sources 
  or analytics modules if needed.

This documentation serves as a blueprint to re-implement or understand the design and functionality 
of the LiveTokenData class for real-time token state management and analytics.
"""
from datetime import datetime
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional
from enum import Enum
from logging import Logger
import pandas as pd

from eth_token.erc20_token.data.pools import PoolManager, PoolReservePriceTracker
from eth_token.erc20_token.data.token_approval_sync_data import UniV2PairSyncData
from eth_token.utils.common_addresses import DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, DENOM_NAMES_TO_ADDRESS
from eth_block_processor.address.contract_type import get_erc20_contract_info
        
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
class ERC20TokenData:
    logger: Logger
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
    trading_enabled_event_index: Optional[int] = None
    trading_enabled_event: Optional[Dict] = None
    token_status: Optional[TokenStatusEnum] = None

    # Event collections with proper typing
    erc20_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: txn_hash, value: list of erc20 transfers
    eth_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: txn_hash, value: list of eth transfers
    other_denom_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: txn_hash, value: list of other denom transfers
    
    approvals: List[Dict] = field(default_factory=list)
    other_currencies: Dict[str, int] = field(default_factory=dict) # {Currency: Number of transfers}

    # Ownership info
    owner_events: List[Dict] = field(default_factory=list)
    current_owner: Optional[str] = None
    all_owners: List[str] = field(default_factory=list)
    ownership_renounced: bool = False
    ownership_renounced_block: Optional[int] = None
    ownership_renounced_txn: Optional[str] = None
    
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
    
    # Event fields for various token events
    renouncement_block: Optional[int] = None
    renouncement_txn: Optional[str] = None
    renouncement_event: Optional[Dict] = None
    renouncement_event_index: Optional[int] = None
    renouncement_event_log_index: Optional[int] = None
    
    tax_event: Optional[Dict] = None
    tax_event_block: Optional[int] = None
    tax_event_txn: Optional[str] = None
    tax_event_index: Optional[int] = None
    tax_event_log_index: Optional[int] = None
    
    max_buy_limit: Optional[int] = None
    max_buy_limit_block: Optional[int] = None
    max_buy_limit_txn: Optional[str] = None
    max_buy_limit_index: Optional[int] = None
    max_buy_limit_log_index: Optional[int] = None
    
    max_buy_ratio: Optional[float] = None
    max_buy_ratio_block: Optional[int] = None
    max_buy_ratio_txn: Optional[str] = None
    max_buy_ratio_index: Optional[int] = None
    max_buy_ratio_log_index: Optional[int] = None
    
    # Modern, unified pool management system (initialized in __post_init__)
    pool_manager: Optional[PoolManager] = field(default=None, init=False)
    reserve_tracker: Optional[PoolReservePriceTracker] = field(default=None, init=False)
    
    def __post_init__(self):
        """Initialize pool management system."""
        # Pool manager will be initialized when first pool is created
        self.logger = get_logger(f"erc20_token_{self.contract_address[:8]}")
    
    def to_dict(self):
        return self.__dict__
    
    def _sort_df(self, df: pd.DataFrame) -> pd.DataFrame:
        """Sort the dataframe"""
        if not df.empty:
            df = df.sort_values(by=['block_number', 'txn_index', 'log_index']).reset_index(drop=True)
            df = df.set_index('txn_hash')
        return df
   
    @property
    def erc20_transfer_df(self):
        """Get ERC20 transfers as pandas DataFrame"""    
        all_transfers = []
        for txn_hash, transfers in self.erc20_transfers.items():
            all_transfers.extend(transfers)
        
        if not all_transfers:
            return pd.DataFrame()
            
        return self._sort_df(pd.DataFrame(all_transfers))
    
    @property
    def approval_df(self):
        """Get approvals as pandas DataFrame"""
        if not self.approvals:
            return pd.DataFrame()
        return self._sort_df(pd.DataFrame(self.approvals))
    
    @property
    def sync_data(self):
        """Get the sync data"""
        return UniV2PairSyncData(self.univ2_syncs)

    @property
    def price_df(self):
        """Get the price dataframe"""
        return self.sync_data.to_dataframe()
    
    @property
    def eth_transfer_df(self):
        """Get ETH transfers as pandas DataFrame"""
        all_transfers = []
        for txn_hash, transfers in self.eth_transfers.items():
            all_transfers.extend(transfers)
        
        if not all_transfers:
            return pd.DataFrame()
            
        return self._sort_df(pd.DataFrame(all_transfers))
    
    @property
    def owner_events_df(self):
        """Get ownership events as pandas DataFrame"""
        if not self.owner_events:
            return pd.DataFrame()
            
        return self._sort_df(pd.DataFrame(self.owner_events))   
    
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
    
    @property
    def pool_addresses(self):
        """Get a tuple of all managed pool addresses."""
        if self.pool_manager:
            return tuple(self.pool_manager.get_all_pool_addresses())
        return tuple()
    
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
    
    @property
    def has_pool(self):
        """Check if the token has any managed liquidity pools."""
        return self.pool_manager.has_pools() if self.pool_manager else False
    
    @property
    def has_uni_v2_pool(self) -> bool:
        """Check if token has Uniswap V2 pools"""
        if self.pool_manager:
            return self.pool_manager.has_v2_pools()
        return False
    
    @property
    def has_uni_v3_pool(self) -> bool:
        """Check if token has Uniswap V3 pools"""
        if self.pool_manager:
            return self.pool_manager.has_v3_pools()
        return False
    
    @property
    def has_uni_v4_pool(self) -> bool:
        """Check if token has Uniswap V4 pools"""
        if self.pool_manager:
            return self.pool_manager.has_v4_pools()
        return False
    
    @property
    def all_pool_reserves(self):
        """Get current reserves across all pools"""
        if not self.pool_manager:
            return {}
            
        reserves = {}
        for pool in self.pool_manager.get_all_pools():
            reserves[pool.pool_address] = {
                'denom_reserve': pool.get_denom_reserve(),
                'token_reserve': pool.get_token_reserve(),
                'denom_symbol': pool.denom_address,
                'protocol': pool.get_protocol()
            }
        return reserves
    
    @property
    def total_liquidity_by_denom(self):
        """Get total liquidity aggregated by denomination currency"""
        if not self.pool_manager:
            return {}
        return self.pool_manager.get_total_liquidity()
    
    @property
    def current_prices(self):
        """
        Get current prices from all pools, delegating to the PoolReserveTracker.
        """
        if not self.pool_manager or not self.reserve_tracker:
            return {}
            
        prices = {}
        for pool_address in self.pool_manager.get_all_pool_addresses():
            price = self.reserve_tracker.get_latest_price(pool_address)
            if price is not None and price > 0:
                pool = self.pool_manager.get_pool(pool_address)
                prices[pool.pool_address] = {
                    'price': price,
                    'protocol': pool.get_protocol(),
                    'denom': pool.denom_address
                }
        return prices
    
    @property
    def swap_events_df(self):
        """Get all swap events across pools as DataFrame"""
        if not self.pool_manager:
            return pd.DataFrame()
            
        all_swaps = []
        for pool in self.pool_manager.get_all_pools():
            if hasattr(pool, 'swap_events'):
                for swap in pool.swap_events:
                    swap_with_pool = {
                        **swap,
                        'pool_address': pool.pool_address,
                        'protocol': pool.get_protocol()
                    }
                    all_swaps.append(swap_with_pool)
        
        return self._sort_df(pd.DataFrame(all_swaps)) if all_swaps else pd.DataFrame()
    
    @property
    def lp_analytics(self):
        """Get LP token analytics across all V2 pools"""
        if not self.pool_manager:
            return {}
        return self.pool_manager.get_lp_holder_stats()
    
    @property
    def pool_info(self):
        """Get pool information dictionary. Sourced directly from PoolManager."""
        if not self.pool_manager:
            return {}
        return self.pool_manager.get_pool_info()
    
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
        if not self.bribe_amount_dict:
            return 0
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
                    # LP token transfers - route to specific pool
                    if self.pool_manager:
                        pool = self.pool_manager.get_pool(token_address)
                        if pool and pool.get_protocol() == 'V2':
                            # Only V2 pools have LP tokens
                            # Add transaction context to transfer
                            transfer['block_number'] = transaction['block_number']
                            transfer['txn_hash'] = transaction['hash']
                            pool.process_lp_transfer(transfer)
                    
                elif token_address == DENOM_NAMES_TO_ADDRESS['WETH']:
                    # WETH transfers
                    self._add_weth_transfer(transaction, transfer)
                    
                elif token_address in DENOM_NAMES_TO_ADDRESS.values():
                    # Other known token transfers (USDC, USDT, etc.)
                    self._add_other_token_transfer(transaction, transfer)      
            except Exception as e:
                self.logger.error(f"Error processing transfer {transfer.get('token_address')} in txn {transaction.get('hash')}: {e}")

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
            
        # Don't override scam status
        if self.is_scam:
            return
            
        # Update token status if we have pools but not trading enabled
        if self.has_pool and self.token_status != TokenStatusEnum.TRADING_ENABLED:
            self.token_status = TokenStatusEnum.PAIR_CREATION
            
    
    def _initialize_pool_manager(self):
        """Initialize the PoolManager and PoolReserveTracker"""
        
        if not hasattr(self, 'logger'):
            self.logger = get_logger("live_tokens", log_folder="token_manager")
        
        # Initialize PoolManager
        self.pool_manager = PoolManager(
            token_address=self.contract_address,
            logger=self.logger
        )
        
        # Initialize PoolReserveTracker for scam detection
        self.reserve_tracker = PoolReservePriceTracker(
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
        if self.total_supply == 0:
            return
        
        # Initialize PoolManager on first transaction if not already done
        if not self.pool_manager:
            self._initialize_pool_manager()
        
        # Convert transaction to ProcessedTransaction format
        processed_tx = self._convert_to_processed_transaction(transaction)
        
        # Process pool events through PoolManager
        self._process_pool_events(processed_tx, transaction)
        
        # Process other transaction events
        self._process_transaction_events(transaction)
        
        # Update latest block info
        self.latest_block_number = transaction['block_number']
        self.latest_block_timestamp = transaction['block_timestamp']
    
    def _convert_to_processed_transaction(self, transaction: Dict):
        """Convert transaction dict to ProcessedTransaction format"""
        if not isinstance(transaction, dict):
            return transaction
            
        # Create object with all transaction data
        processed_tx = type('ProcessedTransaction', (), transaction)()
        
        # Set all transaction attributes
        for key in dir(transaction):
            if not key.startswith('__') and not callable(getattr(transaction, key)):
                setattr(processed_tx, key, getattr(transaction, key))
                
        return processed_tx
    
    def _process_pool_events(self, processed_tx, transaction: Dict):
        """Process pool-related events"""
        if not self.pool_manager:
            return
            
        # Process pool events through PoolManager
        self.pool_manager.process_transaction(processed_tx)
        
        # Update reserve tracker
        if self.reserve_tracker:
            self._update_reserve_tracker(transaction)
            
        # Check for scam detection
        if self.reserve_tracker.is_scam and not self.is_scam:
            self.is_scam = True
            self.scam_label = self.reserve_tracker.scam_reason
            self.scam_block = self.reserve_tracker.scam_block
            self.scam_txn = transaction['hash']
            self.token_status = TokenStatusEnum.INACTIVE_SCAM
            
        # Update pool status
        self._update_pool_status_from_manager()
    
    def _update_reserve_tracker(self, transaction: dict):
        """Update the reserve tracker with the latest pool states."""
        for pool in self.pool_manager.get_all_pools():
            if not hasattr(pool, 'state') or pool.state.last_update_block is None:
                continue

            denom_reserve = pool.get_denom_reserve()
            token_reserve = pool.get_token_reserve()
            price = pool.get_price()

            if denom_reserve > 0 or token_reserve > 0:
                timestamp = transaction['block_timestamp']
                if hasattr(timestamp, 'timestamp'):
                    timestamp = int(timestamp.timestamp())

                self.reserve_tracker.update_reserves(
                    pool.pool_address,
                    pool.denom_address,
                    denom_reserve,
                    token_reserve,
                    price,
                    transaction['block_number'],
                    timestamp,
                    transaction['hash']
                )
    
    def _process_transaction_events(self, transaction: Dict):
        """Process non-pool transaction events"""
        # Process ERC20 transfers
        self._add_transfers(transaction)
        
        # Process internal transfers
        self._add_eth_transfer(transaction)

        # Process approvals
        self._add_approval(transaction)

        # Process owner events
        self._add_owner_event(transaction)

        # Update address tx counter
        self.updated_address_tx_counter(transaction.get('unique_addresses', set()))

        # Update bribe amount
        self._update_bribe_amount(transaction)
        
        # Process trading enabled events
        if self.has_pool or not self.trading_enabled:
            self._add_trading_enabled_event(transaction)
    
    def get_token_summary(self) -> Dict:
        """Get comprehensive token summary including all key metrics"""
        summary = {
            # Basic info
            'contract_address': self.contract_address,
            'name': self.name,
            'symbol': self.symbol,
            'decimals': self.decimals,
            'total_supply': self.total_supply,
            
            # Status
            'token_status': self.token_status.value if self.token_status else None,
            'trading_enabled': self.trading_enabled,
            'is_scam': self.is_scam,
            'scam_label': self.scam_label,
            
            # Creation info
            'creation_block': self.creation_block,
            'creator_address': self.creator_address,
            'current_owner': self.current_owner,
            
            # Pool info
            'has_pools': self.has_pool,
            'pool_count': len(self.pool_addresses),
            'protocols': list(set(info['protocol'] for info in self.current_prices.values())),
            
            # Activity metrics
            'total_transactions': len(self.txn_hashes),
            'unique_addresses': len(self.unique_addresses),
            'total_transfers': sum(len(transfers) for transfers in self.erc20_transfers.values()),
            
            # Latest activity
            'latest_block': self.latest_block_number,
            'latest_timestamp': self.latest_block_timestamp
        }
        
        # Add liquidity info if pools exist
        if self.has_pool:
            summary.update({
                'total_liquidity_by_denom': self.total_liquidity_by_denom,
                'current_prices': self.current_prices,
                'lp_analytics': self.lp_analytics if self.has_uni_v2_pool else None
            })
        
        return summary
    