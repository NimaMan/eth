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
from typing import Any, Dict, List, Set, Optional, Iterator, Union
from enum import Enum
from web3 import Web3
from .token_chain_data_fetcher import TokenChainDataFetcher
from eth_token.erc20_token.data.pools import PoolManager
from eth_token.erc20_token.data.pools.pool_manager import UNISWAP_V2_PROTOCOL
from eth_token.erc20_token.data.pools.base_pool import BasePool
from .pool_liquidity_matrix import PoolLiquidityMatrix
from eth_data.chain_utils.common_addresses import (
    DENOM_ADDRESSES,
    ERC20_TOKEN_DECIMALS,
    DENOM_NAMES_TO_ADDRESS,
)


# Hidden mint detection threshold - flags if circulating supply exceeds total supply by this factor
HIDDEN_MINTS_THRESHOLD = 1+1e-2


class TokenStatusEnum(Enum):
    CREATION = "CONTRACT_CREATION"
    PAIR_CREATION = "PAIR_CREATION"
    TRADING_ENABLED = "TRADING_ENABLED"
    INACTIVE_SCAM = "INACTIVE_SCAM"
    INACTIVE_OTHER = "INACTIVE_OTHER"


class PoolCollection:
    """Lightweight view over pools managed by PoolManager."""

    def __init__(self, manager: PoolManager):
        self._manager = manager

    def _all_pools(self) -> List['BasePool']:
        return self._manager.get_all_pools()

    def _normalize_key(self, key: str) -> str:
        if '#' in key:
            # V4 display address (PoolManager#poolId) should be passed through
            return key
        try:
            return Web3.to_checksum_address(key)
        except ValueError:
            return key

    def __len__(self) -> int:
        return len(self._all_pools())

    def __iter__(self) -> Iterator['BasePool']:
        return iter(self._all_pools())

    def __getitem__(self, key: Union[int, slice, str]) -> Union['BasePool', List['BasePool']]:
        if isinstance(key, slice):
            return self._all_pools()[key]

        if isinstance(key, int):
            pools = self._all_pools()
            try:
                return pools[key]
            except IndexError as exc:
                raise IndexError(f"Pool index {key} out of range") from exc

        if isinstance(key, str):
            pool = self._manager.get_pool(self._normalize_key(key))
            if pool is None:
                raise KeyError(f"No pool found for key '{key}'")
            return pool

        raise TypeError(f"Pools can be indexed by int, slice, or address string, not {type(key)!r}")

    def __contains__(self, item: Union[str, 'BasePool']) -> bool:
        if isinstance(item, str):
            return self._manager.get_pool(self._normalize_key(item)) is not None
        try:
            return item in self._all_pools()
        except Exception:
            return False

    def addresses(self) -> List[str]:
        """Return currently known pool identifiers (addresses + V4 ids)."""
        addresses = list(self._manager.pools.keys())
        v4_identifiers = [
            getattr(pool, "display_address", getattr(pool, "pool_address", None))
            for pool in self._manager.v4_pools.values()
        ]
        addresses.extend(addr for addr in v4_identifiers if addr is not None)
        return addresses

@dataclass
class ERC20TokenData:
    contract_address: str  
    tx_hashes_to_makers: Dict[str, str] = field(default_factory=dict) # key: tx_hash, value: fee source
    history_limit: int = 1000

    # Core token info
    name: Optional[str] = None
    symbol: Optional[str] = None
    decimals: Optional[int] = None
    total_supply: Optional[int] = 0
    total_supply_from_transfers: Optional[int] = 0
    
    # Creation info
    creation_block: Optional[int] = None
    creation_timestamp: Optional[datetime] = None
    creation_tx: Optional[str] = None
    creator_address: Optional[str] = None
    creator_nonce: Optional[int] = None
    
    # Contract state
    token_status: Optional[TokenStatusEnum] = None

    # Event collections with proper typing
    erc20_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: tx_hash, value: list of erc20 transfers
    eth_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: tx_hash, value: list of eth transfers
    other_denom_transfers: Dict[str, List[Dict]] = field(default_factory=dict) # key: tx_hash, value: list of other denom transfers
    
    approvals: List[Dict] = field(default_factory=list)
    other_currencies: Dict[str, int] = field(default_factory=dict) # {Currency: Number of transfers}

    # Ownership info
    owner_events: List[Dict] = field(default_factory=list)
    current_owner: Optional[str] = None
    all_owners: List[str] = field(default_factory=list)
    ownership_renounced: bool = False
    ownership_renounced_block: Optional[int] = None
    ownership_renounced_tx: Optional[str] = None
    
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
    scam_tx: Optional[str] = None
    
    # Event fields for various token events
    renouncement_block: Optional[int] = None
    renouncement_tx: Optional[str] = None
    renouncement_event: Optional[Dict] = None
    renouncement_event_index: Optional[int] = None
    renouncement_event_log_index: Optional[int] = None    
    pool_manager: PoolManager = field(init=False)
    token_chain_fetcher: TokenChainDataFetcher = field(init=False, repr=False)
    _pools_view: PoolCollection = field(init=False, repr=False, default=None)
    _liquidity_matrix: PoolLiquidityMatrix = field(init=False, repr=False, default=None)

    def __post_init__(self) -> None:
        """Initialise helpers that depend on the contract address."""
        self.token_chain_fetcher = TokenChainDataFetcher()
        self.pool_manager = PoolManager(
            token_address=self.contract_address,
            history_limit=self.history_limit,
        )
        if self.decimals is not None:
            self.pool_manager._token_decimals = int(self.decimals)
        self._pools_view = PoolCollection(self.pool_manager)
        self._liquidity_matrix = PoolLiquidityMatrix(self.pool_manager)
        self._liquidity_matrix = PoolLiquidityMatrix(self.pool_manager)

    def _append_with_limit(self, items: List[Any], entry: Any) -> None:
        items.append(entry)
        if len(items) > self.history_limit:
            del items[: len(items) - self.history_limit]

    def _append_dict_with_limit(self, mapping: Dict[str, List[Any]], key: str, entry: Any) -> None:
        bucket = mapping.setdefault(key, [])
        bucket.append(entry)
        if len(bucket) > self.history_limit:
            del bucket[: len(bucket) - self.history_limit]
        while len(mapping) > self.history_limit:
            oldest_key = next(iter(mapping))
            if oldest_key == key and len(mapping) == 1:
                break
            mapping.pop(oldest_key)

    @property
    def pools(self) -> PoolCollection:
        """Convenient view of tracked pools, indexable by position or address."""
        return self._pools_view

    @property
    def liquidity_matrix(self) -> PoolLiquidityMatrix:
        """Matrix-based liquidity and price analysis across all pools."""
        return self._liquidity_matrix

    @property
    def liquidity_analyzer(self) -> PoolLiquidityMatrix:
        """
        Backwards compatible alias for components expecting a
        ``liquidity_analyzer`` attribute.
        """
        return self._liquidity_matrix

    def to_dict(self):
        data = self.__dict__.copy()
        data.pop('_pools_view', None)
        data.pop('_liquidity_matrix', None)
        return data

    @property
    def fee_sources(self):
        """Get the fee sources"""
        return list(set(self.tx_hashes_to_makers.values()))
    
    @property
    def tx_hashes(self):
        """Get the tx hashes"""
        return list(set(self.tx_hashes_to_makers.keys()))
    
    def get_address_tx_hashes(self, address: str) -> List[str]:
        return [tx_hash for tx_hash, maker in self.tx_hashes_to_makers.items() if maker == address]

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
        if not self.pool_manager:
            return {}
            
        prices = {}
        for pool in self.pool_manager.get_all_pools():
            price = pool.get_price()
            if price is not None and price > 0:
                prices[pool.pool_address] = {
                    'price': price,
                    'protocol': pool.get_protocol(),
                    'denom': pool.denom_address
                }
        return prices
    
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
        if self.is_scam or not self.pool_manager:
            return price_ratios
            
        for pool in self.pool_manager.get_all_pools():
            ratio = pool.reserve_tracker.get_price_ratio_to_initial()
            price_ratios[pool.pool_address] = ratio if ratio is not None else 0.0            
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
        self.creation_tx = transaction['hash']
        self.creator_address = transaction['from_address']
        self.creator_nonce = transaction['nonce']
        self.current_owner = transaction['from_address'] # Set initial owner
        if self.total_supply is None:
            self.set_erc20_contract_info(transaction.get('block_header_json'))

    def set_erc20_contract_info(
        self,
        creation_block_header: Optional[Union[Dict[str, Any], str]] = None,
    ):
        """Fetch and update ERC20 contract info from on-chain data"""
        info = self.token_chain_fetcher.get_token_metadata(
            self.contract_address,
            self.creation_block,
            creation_block_header,
        )
        if info is None:
            raise RuntimeError(
                f"Missing token metadata for {self.contract_address} from PyReth"
            )
        self.name = str(info.name)
        self.symbol = str(info.symbol)
        self.decimals = int(info.decimals)
        self.pool_manager._token_decimals = self.decimals
        self.total_supply = int(info.total_supply)
        self.token_status = TokenStatusEnum.CREATION

    @property
    def _update_trading_enabled(self, transaction: Dict):
        """Update the trading enabled status"""
        self.trading_enabled = True
        self.trading_enabled_block = transaction['block_number']
        self.trading_enabled_timestamp = transaction['block_timestamp']
        self.trading_enabled_tx = transaction['hash']
        self.trading_enabled_event_index = transaction['tx_index']
        self.token_status = TokenStatusEnum.TRADING_ENABLED
            
    def _add_trading_enabled_event(self, transaction: Dict):
        """Process a trading enabled event"""
        for trading_enabled_event in transaction.get('trading_enabled_events', []):
            self.trading_enabled_event = trading_enabled_event
            if not self.trading_enabled:
                self._update_trading_enabled(transaction)
    
    def _add_erc20_transfer(self, transaction: Dict, transfer: Dict):
        """Process a transfer event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        amount = int(transfer['amount'])/10**self.decimals
        transfer_dict = {
            'tx_hash': tx_hash,
            'block_number': block_number,
            'tx_index': tx_index,
            'log_index': transfer['log_index'],
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': amount,
            'token_address': transfer['token_address']
        }
        self._append_dict_with_limit(self.erc20_transfers, tx_hash, transfer_dict)
        if transfer['from_address'] == '0x0000000000000000000000000000000000000000':
            self.total_supply_from_transfers += amount
        
    def _add_weth_transfer(self, transaction: Dict, transfer: Dict):
        """Process an ETH transfer event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        transfer_dict = {
            'tx_hash': tx_hash,
            'block_number': block_number,
            'tx_index': tx_index,
            'log_index': transfer['log_index'],
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': float(transfer['amount'])/10**18,
            'token_address': "WETH"
        }
        self._append_dict_with_limit(self.eth_transfers, tx_hash, transfer_dict)
        
    def _add_eth_transfer(self, transaction: Dict):
        """Process an internal transfer event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        for transfer in transaction.get('internal_transactions', []):
            transfer_dict = {
                'tx_hash': tx_hash,
                'block_number': block_number,
                'tx_index': tx_index,
                'depth': transfer['depth'],
                'from_address': transfer['from_address'],
                'to_address': transfer['to_address'],
                'amount': float(transfer['value']),
                'token_address': "ETH"
            }
            self._append_dict_with_limit(self.eth_transfers, tx_hash, transfer_dict)

    def _add_other_token_transfer(self, transaction: Dict, transfer: Dict):
        """Process transfers of other known tokens (USDC, USDT, etc.)"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        if transfer['token_address'] in DENOM_ADDRESSES.keys():
            denom_name = DENOM_ADDRESSES[transfer['token_address']]
            denom_decimals = ERC20_TOKEN_DECIMALS[denom_name]
            amount = float(transfer['amount'])/10**denom_decimals
            if denom_name not in self.other_currencies:
                self.other_currencies[denom_name] = 0
            self.other_currencies[denom_name] += 1
        
            transfer_dict = {
                'tx_hash': tx_hash,
                'block_number': block_number,
                'tx_index': tx_index,
                'log_index': transfer['log_index'],
                'from_address': transfer['from_address'],
                'to_address': transfer['to_address'],
                'amount': amount,
                'token_address': transfer['token_address'],
            }
            self._append_dict_with_limit(self.other_denom_transfers, tx_hash, transfer_dict)     

    def _add_transfers(self, transaction: Dict):
        """Process transfer events for all relevant tokens"""
        # Track all transfers in the transaction
        for transfer in transaction.get('erc20_transfers', []):
            try:
                token_address = transfer['token_address']
                
                if token_address == self.contract_address:
                    # Our token transfers
                    self._add_erc20_transfer(transaction, transfer)
                    
                else:
                    pool = self.pool_manager.get_pool(token_address) if self.pool_manager else None
                    if pool and pool.get_protocol() == UNISWAP_V2_PROTOCOL:
                        # V2 LP token transfer
                        transfer['block_number'] = transaction['block_number']
                        transfer['tx_hash'] = transaction['hash']
                        pool.process_lp_transfer(transfer)
                        continue
                    
                if token_address == DENOM_NAMES_TO_ADDRESS['WETH']:
                    # WETH transfers
                    self._add_weth_transfer(transaction, transfer)
                    
                elif token_address in DENOM_NAMES_TO_ADDRESS.values():
                    # Other known token transfers (USDC, USDT, etc.)
                    self._add_other_token_transfer(transaction, transfer)      
            except Exception as exc:  # noqa: BLE001
                raise RuntimeError(
                    "ERC20TokenData._add_transfers failed: "
                    f"token={transfer.get('token_address')} tx={transaction.get('hash')} error={exc}"
                ) from exc

    def _add_approval(self, transaction: Dict):
        """Process an approval event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        for approval in transaction.get('approvals', []):
            token_address = approval['token_address']
            
            # Check if this is an LP token approval (pool address)
            pool = self.pool_manager.get_pool(token_address) if self.pool_manager else None
            if pool and pool.get_protocol() == UNISWAP_V2_PROTOCOL:
                # Add transaction context
                approval['block_number'] = block_number
                approval['tx_hash'] = tx_hash
                approval['block_timestamp'] = transaction.get('block_timestamp')
                pool.process_lp_approval(approval)
            
            # Also store in general approvals list
            self._append_with_limit(self.approvals, {
                'tx_hash': tx_hash,
                'block_number': block_number,
                'tx_index': tx_index,
                'log_index': approval['log_index'],
                'owner': approval['owner'],
                'spender': approval.get('spender') or approval.get('approved_address'),
                #'amount': float(approval['amount'])/10**self.decimals,
                'token_address': approval['token_address']
            })
            self.approved_addresses.add(approval.get('spender') or approval.get('approved_address'))

    def _add_owner_event(self, transaction: Dict):
        """Process an owner event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        for owner_event in transaction.get('owner_events', []):
            self._append_with_limit(self.all_owners, owner_event['new_owner'])
            self.current_owner = owner_event['new_owner']
            if owner_event['previous_owner'] == '0x0000000000000000000000000000000000000000':
                self.ownership_renounced = True
                self.ownership_renounced_block = block_number
                self.ownership_renounced_tx = tx_hash

            self._append_with_limit(self.owner_events, {
                'tx_hash': tx_hash,
                'block_number': block_number,
                'tx_index': tx_index,
                'log_index': owner_event['log_index'],
                'previous_owner': owner_event['previous_owner'],
                'new_owner': owner_event['new_owner'],
                'token_address': owner_event['contract_address']
            })

    def _add_renouncement_event(self, transaction: Dict):
        """Process a renouncement event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        
        for renouncement_event in transaction.get('renouncement_events', []):
            self.renouncement_block = block_number
            self.renouncement_tx = tx_hash
            self.renouncement_event = renouncement_event
            self.renouncement_event_index = tx_index
            self.renouncement_event_log_index = renouncement_event['log_index']
        
    def _add_tax_event(self, transaction: Dict):
        """Process a tax event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        for tax_event in transaction.get('tax_events', []):
            self.tax_event = tax_event
            self.tax_event_block = block_number
            self.tax_event_tx = tx_hash
            self.tax_event_index = tx_index
            self.tax_event_log_index = tax_event['log_index']

    def _add_max_buy_limit_event(self, transaction: Dict):
        """Process a max buy limit event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        
        for max_buy_limit_event in transaction.get('max_buy_limit_events', []):
            self.max_buy_limit = max_buy_limit_event['max_buy_limit']
            self.max_buy_limit_block = block_number
            self.max_buy_limit_tx = tx_hash
            self.max_buy_limit_index = tx_index
            self.max_buy_limit_log_index = max_buy_limit_event['log_index']

    def _add_max_buy_ratio_event(self, transaction: Dict):
        """Process a max buy ratio event"""
        tx_hash = transaction['hash']
        block_number = transaction['block_number']
        tx_index = transaction['tx_index']
        
        for max_buy_ratio_event in transaction.get('max_buy_ratio_events', []):
            self.max_buy_ratio = max_buy_ratio_event['max_buy_ratio']
            self.max_buy_ratio_block = block_number
            self.max_buy_ratio_tx = tx_hash
            self.max_buy_ratio_index = tx_index
            self.max_buy_ratio_log_index = max_buy_ratio_event['log_index']

    def _process_other_events(self, transaction: Dict):
        """Process other relevant events"""
        # Trading enabled is now tracked at pool level, not token level
        pass

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
        # Initialize PoolManager
        self.pool_manager = PoolManager(
            token_address=self.contract_address,
            history_limit=self.history_limit,
        )
        if self.decimals is not None:
            self.pool_manager._token_decimals = int(self.decimals)
        self._pools_view = PoolCollection(self.pool_manager)

    def updated_address_tx_counter(self, unique_addresses: set):
        """Update the transaction counter for a unique address"""
        for unique_address in unique_addresses:
            if unique_address not in self.address_tx_counter:
                self.address_tx_counter[unique_address] = 0
            self.address_tx_counter[unique_address] += 1

    def update_from_transaction(self, transaction: Dict):
        """Update token data from a new transaction"""
        # Add the tx hash to the list of processed transactions
        self.tx_hashes_to_makers[transaction['hash']] = transaction['from_address']

        # Handle contract creation
        tx_type = transaction.get('tx_type', transaction.get('tx_type'))
        if tx_type == 'Contract Creation' or transaction.get('contract_creation_events'):
            self._handle_creation(transaction)

        self._process_pool_events(transaction)
        self._process_transaction_events(transaction) # Process other transaction events
        self.latest_block_number = transaction['block_number'] # Update latest block info
        self.latest_block_timestamp = transaction['block_timestamp']

    def _process_pool_events(self, transaction: Dict):
        """Process pool-related events"""
        if not self.pool_manager:
            return
        # Process pool events through PoolManager
        self.pool_manager.process_transaction(transaction)
        #Todo: This is handled within each pool. but this one might be token level 
        # Check for hidden mint scam first
        if self.total_supply and self.total_supply_from_transfers:
            if self.total_supply_from_transfers > self.total_supply * HIDDEN_MINTS_THRESHOLD:
                if not self.is_scam:
                    self.is_scam = True
                    self.scam_label = "hidden_mint"
                    self.scam_block = transaction['block_number']
                    self.scam_tx = transaction['hash']
                    self.token_status = TokenStatusEnum.INACTIVE_SCAM      
        # Update pool status
        self._update_pool_status_from_manager()
    
    def _process_transaction_events(self, transaction: Dict):
        """Process non-pool transaction events"""
        self._add_transfers(transaction) # Process ERC20 transfers
        self._add_eth_transfer(transaction) # Process internal transfers
        self._add_approval(transaction)         # Process approvals
        self._add_owner_event(transaction)         # Process owner events
        self.updated_address_tx_counter(transaction.get('unique_addresses', set()))         # Update address tx counter
        self._update_bribe_amount(transaction)         # Update bribe amount
    
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
            # Trading enabled is now tracked at pool level
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
            'total_transactions': len(self.tx_hashes),
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
    
