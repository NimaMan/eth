"""
LiveTokenData: Real-time Token Data Management and Analytics
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
from collections import OrderedDict

from eth_token.utils.common_addresses import DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, DENOM_NAMES_TO_ADDRESS
from eth_block_processor.address.contract_type import get_erc20_contract_info


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
    
    # Uniswap V2
    univ2_syncs: List[Dict] = field(default_factory=list)
    univ2_swaps: Dict[str, List[Dict]] = field(default_factory=dict)
    univ2_mints: List[Dict] = field(default_factory=list)
    univ2_burns: List[Dict] = field(default_factory=list)
    univ2_pair: List[Dict] = field(default_factory=list)

    # Uniswap V3
    univ3_swaps: Dict[str, List[Dict]] = field(default_factory=dict)
    univ3_mints: List[Dict] = field(default_factory=list)
    univ3_burns: List[Dict] = field(default_factory=list)
    uniswap_v3_pool: List[Dict] = field(default_factory=list)
    
    # Uniswap V4
    univ4_swaps: Dict[str, List[Dict]] = field(default_factory=dict)
    univ4_mints: List[Dict] = field(default_factory=list)
    univ4_burns: List[Dict] = field(default_factory=list)
    uniswap_v4_pool: List[Dict] = field(default_factory=list)
    
    # Pool info
    has_uni_v2_pool: bool = False
    has_uni_v3_pool: bool = False
    has_uni_v4_pool: bool = False
    pool_prices: OrderedDict[str, List[float]] = field(default_factory=dict)
    pool_reserves: OrderedDict[str, List[float]] = field(default_factory=dict)
    # Token Type Series
    pool_info: Dict[str, Dict] = field(default_factory=dict) # {pool_address: {pool_type: str, denom_currency: str, decimals: int, is_token1_denom: bool}}
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
        if pool_address in self.pool_info.keys():
            return self.pool_info[pool_address]['denom_currency']
        else:
           address_info = get_erc20_contract_info(pool_address)
           return address_info['name']

    @property
    def denom_currency(self):
        """Get the denom currencies"""
        denom_currencies = {}
        for pool_address, pool_info in self.pool_info.items():
            denom_currencies[pool_address] = pool_info['denom_currency']
        if len(denom_currencies) > 1:
            return f"{', '.join(denom_currencies.values())}"
        elif len(denom_currencies) == 1:
            return tuple(denom_currencies.values())[0]
        else:
            return None
    
    @property
    def pool_addresses(self):
        """Get the pool addresses"""
        return tuple(self.pool_info.keys())
    
    @property
    def has_pool(self):
        """Get the pool types"""
        return len(self.pool_info) > 0
    
    @property
    def latest_pools_price_ratio(self):
        """Get the current price ratio"""
        latest_price_ratios = {}
        if self.is_scam:
            return {pool_address: 0 for pool_address in self.pool_addresses}
        
        for pool_address, prices in self.pool_prices.items():
            if self.pool_info[pool_address]['pool_type'] == "V2":
                if len(prices) > 0:
                    latest_price_ratios[pool_address] = prices[-1]/prices[0]
            elif self.pool_info[pool_address]['pool_type'] == "V3":
                if len(prices) > 0:
                    latest_price_ratios[pool_address] = (prices[-1]/prices[0])**2
            elif self.pool_info[pool_address]['pool_type'] == "V4":
                if len(prices) > 0:
                    latest_price_ratios[pool_address] = (prices[-1]/prices[0])**2
        return latest_price_ratios
    
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
    
    def set_pool_info(self, pool_address: str, pool_type: str, denom_address: str,  token1_is_denom: bool = None) -> bool:
        """Set the pool info and return True if the pool is valid, False otherwise"""
        if denom_address in DENOM_ADDRESSES.keys():
            denom_currency = DENOM_ADDRESSES[denom_address]
            decimals = ERC20_TOKEN_DECIMALS[denom_currency]
        else:
            token_info = get_erc20_contract_info(denom_address)
            if token_info is not None:
                denom_currency = token_info['name']
                decimals = token_info['decimals']
            else:
                return False # Pool is not a valid denom
        self.pool_info[pool_address] = {
            'pool_type': pool_type,
            'denom_address': denom_address,
            'denom_currency': denom_currency,
            'decimals': decimals
        }
        if token1_is_denom is not None:
            self.pool_info[pool_address]['token1_is_denom'] = token1_is_denom
        return True # Pool is valid
    
    def update_pool_reserves(self, pool_address: str, reserve_change: float, reserve_type: str):
        """Update the pool reserves"""
        if pool_address not in self.pool_reserves.keys():
            self.pool_reserves[pool_address] = {
                "denom_reserve_changes": [], 
                "token_reserve_changes": [],
                "denom_reserve": 0,
                "token_reserve": 0
                }
        if reserve_type == "denom":
            self.pool_reserves[pool_address]["denom_reserve_changes"].append(reserve_change)
            self.pool_reserves[pool_address]["denom_reserve"] += reserve_change
        else:
            self.pool_reserves[pool_address]["token_reserve_changes"].append(reserve_change)
            self.pool_reserves[pool_address]["token_reserve"] += reserve_change

    def _check_pool_reserves(self, transaction: Dict):
        """Check the pool reserves and set the token status to scam if all the pools have less than denom_threshold"""
        # check if we have any reserves
        if len(self.pool_reserves) == 0:
            return
        denom_reserves = {}
        token_reserves = []
        for pool_address, pool_info in self.pool_info.items():
            denom_address = pool_info['denom_address']
            if pool_address not in self.pool_reserves.keys():
                continue
            denom_reserves[denom_address] = self.pool_reserves[pool_address]["denom_reserve"]
            token_reserves.append(self.pool_reserves[pool_address]["token_reserve"])
        if len(denom_reserves) > 0:
            for denom_address, denom_reserve in denom_reserves.items():
                # Check if it is a known denom
                if denom_address in DENOM_ADDRESSES.keys():
                    denom_currency = DENOM_ADDRESSES[denom_address]
        
                    if denom_currency == 'WETH':
                        if denom_reserve < WETH_DENOM_RESERVE_THRESHOLD:
                            self.is_scam = True
                            self.scam_label = f'Denom Reserve({denom_currency}) < {WETH_DENOM_RESERVE_THRESHOLD}'
                            self.scam_block = transaction['block_number']
                            self.scam_txn = transaction['hash']
                            self.token_status = TokenStatusEnum.INACTIVE_SCAM
                    else:
                        if denom_reserve < USD_DENOM_RESERVE_THRESHOLD:
                            self.is_scam = True
                            self.scam_label = f'Denom Reserve({denom_currency}) < {USD_DENOM_RESERVE_THRESHOLD}'
                            self.scam_block = transaction['block_number']
                            self.scam_txn = transaction['hash']
                            self.token_status = TokenStatusEnum.INACTIVE_SCAM
            
        if len(token_reserves) > 0:
            if sum(token_reserves)/self.total_supply > HIDDEN_MINTS_THRESHOLD:
                self.is_scam = True
                self.scam_label = f'Token in Circulation > 1'
                self.scam_block = transaction['block_number']
                self.scam_txn = transaction['hash']
                self.token_status = TokenStatusEnum.INACTIVE_SCAM

    def get_pool_reserve(self, pool_address: str):
        """Get the initial reserve"""
        if pool_address not in self.pool_reserves.keys():
            return None
        return self.pool_reserves[pool_address]['denom_reserve']

    def set_lp_token_info(self, pool_address: str):
        """Set the lp token info"""
        lp_token_info = {}
        lp_token_address_info = get_erc20_contract_info(pool_address)
        if lp_token_address_info is not None:
            lp_token_info['name'] = lp_token_address_info['name']
            lp_token_info['symbol'] = lp_token_address_info['symbol']
            lp_token_info['decimals'] = lp_token_address_info['decimals']
            lp_token_info['total_supply'] = lp_token_address_info['total_supply']
        self.pool_info[pool_address]['lp_token_info'] = lp_token_info

    def _update_trading_enabled(self, transaction: Dict):
        """Update the trading enabled status"""
        self.trading_enabled = True
        self.trading_enabled_block = transaction['block_number']
        self.trading_enabled_timestamp = transaction['block_timestamp']
        self.trading_enabled_txn = transaction['hash']
        self.trading_enabled_event_index = transaction['txn_index']
        self.token_status = TokenStatusEnum.TRADING_ENABLED

    def _add_univ2_pair_event(self, transaction: Dict):
        """Process a pair event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for pair_event in transaction.get('pair_events', []):
            if self.token_status != TokenStatusEnum.TRADING_ENABLED:
                self.token_status = TokenStatusEnum.PAIR_CREATION
            self.has_uni_v2_pool = True
            if pair_event['token0'] == self.contract_address:
                denom_address = pair_event['token1']
                token1_is_denom = True
            else:
                denom_address = pair_event['token0']
                token1_is_denom = False
            pool_is_valid = self.set_pool_info(pair_event['pair_address'], "V2", denom_address, token1_is_denom)
            if not pool_is_valid:
                return
            self.univ2_pair.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': pair_event['log_index'],
                'pair_address': pair_event['pair_address'],
                'token0': pair_event['token0'],
                'token1': pair_event['token1']
            })
            
            self.set_lp_token_info(pair_event['pair_address'])
            
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
        """Update the ETH pool reserves"""
        # Update pool reserves if pool is the sender. its reserves are reduced
        if transfer_dict['from_address'] in self.pool_addresses:
            self.update_pool_reserves(transfer_dict['from_address'], -transfer_dict['amount'], "denom")
        # Update pool reserves if pool is the recipient
        if transfer_dict['to_address'] in self.pool_addresses:
            self.update_pool_reserves(transfer_dict['to_address'], transfer_dict['amount'], "denom")

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

    def _check_univ2_scam(self, denom_reserve: float, token_reserve: float, block_number: int, txn_hash: str):
        if denom_reserve < WETH_DENOM_RESERVE_THRESHOLD:
            self.is_scam = True
            self.scam_label = f'Denom removal (<{WETH_DENOM_RESERVE_THRESHOLD})'
            self.scam_block = block_number
            self.scam_txn = txn_hash

        token_supply_ratio = token_reserve/self.total_supply
        if token_supply_ratio > HIDDEN_MINTS_THRESHOLD:
            self.is_scam = True
            self.scam_label = f'Hidden mint ({token_supply_ratio:.2f})'
            self.scam_block = block_number
            self.scam_txn = txn_hash
        
        if self.is_scam:
            self.token_status = TokenStatusEnum.INACTIVE_SCAM

    def add_univ2_price_info(self, token_reserve: float, denom_reserve: float, pool_address: str):
        """Add a price info"""
        price = denom_reserve/token_reserve
        if pool_address not in self.pool_prices:
            self.pool_prices[pool_address] = []
        self.pool_prices[pool_address].append(price)

    def _add_univ2_syncs(self, transaction: Dict):
        """Process a sync event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        timestamp = transaction['block_timestamp']
        for sync in transaction.get('uniswap_v2_syncs', []):
            if sync["pair_address"] in self.pool_addresses:
                if self.pool_info[sync['pair_address']]['token1_is_denom']:
                    denom_reserve = float(sync['reserve1'])/10**self.pool_info[sync['pair_address']]['decimals']
                    token_reserve = float(sync['reserve0'])/10**self.decimals
                else:
                    denom_reserve = float(sync['reserve0'])/10**self.pool_info[sync['pair_address']]['decimals']
                    token_reserve = float(sync['reserve1'])/10**self.decimals
                self.univ2_syncs.append({
                    'txn_hash': txn_hash,            
                    'block_number': block_number,
                    'txn_index': txn_index,
                    'log_index': sync['log_index'],
                    'timestamp': timestamp,
                    'from_address': transaction['from_address'],
                    'to_address': transaction['to_address'],
                    'pair_address': sync['pair_address'],
                    'token_reserve': token_reserve,
                    'denom_reserve': denom_reserve,
                    })  
                
                self.add_univ2_price_info(token_reserve, denom_reserve, sync['pair_address'])
                self._check_univ2_scam(denom_reserve, token_reserve, block_number, txn_hash)
                
    def _add_univ2_swaps(self, transaction: Dict):
        """Process a swap event"""
        txn_hash = transaction['hash']
        univ2_swaps = transaction.get('uniswap_v2_swaps', [])
        if len(univ2_swaps) > 0:
            self.univ2_swaps[txn_hash] = []
            # Check if the swap is for a pool that we have already added
            if not univ2_swaps[0]["pair_address"] in self.pool_addresses:
                return # We should have already added the pool address
            if not self.trading_enabled: # Enable trading if it is not already enabled
                self._update_trading_enabled(transaction)

        for swap in univ2_swaps:
            self.univ2_swaps[txn_hash].append({
                'log_index': swap['log_index'],
                'from_address': swap['sender'],
                'to_address': swap['to'],
            })

    def _add_univ2_mint(self, transaction: Dict):
        """Process a mint event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        
        for mint in transaction.get('uniswap_v2_mints', []):
            self.univ2_mints.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': mint['log_index'],
                'to_address': mint['to_address'],
                'amount': mint['amount'],
            'token_address': mint['token_address']
            })

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

    def _add_univ2_burn(self, transaction: Dict):
        """Process a burn event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for burn in transaction.get('uniswap_v2_burns', []):
            self.univ2_burns.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': burn['log_index'],
                'from_address': burn['from_address'],
                'amount': burn['amount'],
                'token_address': burn['token_address']
            })
    
    def _add_univ3_pool(self, transaction: Dict):
        """Process Uniswap V3 pool creation events"""
        for pool_event in transaction.get('uniswap_v3_pools', []):
            if pool_event['token0'] == self.contract_address:
                denom_address = pool_event['token1']
                token1_is_denom = True
            else:
                denom_address = pool_event['token0']
                token1_is_denom = False      
            pool_is_valid = self.set_pool_info(pool_event['pool'], "V3", denom_address, token1_is_denom)
            if not pool_is_valid:
                return
            self.has_uni_v3_pool = True    
            self.uniswap_v3_pool.append({
                'txn_hash': transaction['hash'],
                'block_number': transaction['block_number'],
                'txn_index': transaction['txn_index'],
                'log_index': pool_event['log_index'],
                'pool': pool_event['pool'],
                'token0': pool_event['token0'],
                'token1': pool_event['token1'],
                'fee': pool_event['fee']
            })
        
            self.set_lp_token_info(pool_event['pool'])

    def add_univ3_price_info(self, sqrt_price_x96: int, pool_address: str):
        """Add a price info"""
        if pool_address not in self.pool_prices:
            self.pool_prices[pool_address] = []
        self.pool_prices[pool_address].append(sqrt_price_x96)

    def _add_univ3_swaps(self, transaction: Dict):
        """Process Uniswap V3 swap events"""
        txn_hash = transaction['hash']
        univ3_swaps = transaction.get('uniswap_v3_swaps', [])
        if len(univ3_swaps) > 0:
            if univ3_swaps[0]["pool_address"] not in self.pool_addresses:
                return # We should have already added the pool address
            self.univ3_swaps[txn_hash] = []
            # Ensure trading is enabled if v3 events exist
            if not self.trading_enabled:
                self._update_trading_enabled(transaction)
        
        for swap in univ3_swaps:
            sqrt_price_x96 = int(swap.get('sqrt_price_x96'))
            self.univ3_swaps[txn_hash].append({
                'log_index': swap.get('log_index'),
                'from_address': swap.get('sender'),
                'to_address': swap.get('recipient'),
                'amount0': swap.get('amount0'),
                'amount1': swap.get('amount1'),
                'sqrtPriceX96': sqrt_price_x96,
                'liquidity': swap.get('liquidity'),
                'tick': swap.get('tick'),
                'pool_address': swap.get('pool_address')
            })
            if swap.get('pool_address') in self.pool_addresses:
                self.add_univ3_price_info(sqrt_price_x96, swap.get('pool_address'))

    def _add_univ3_mints(self, transaction: Dict):
        """Process Uniswap V3 mint events"""
        txn_hash = transaction['hash']
        v3_mints = transaction.get('uniswap_v3_mints', [])
        if v3_mints:
            for mint in v3_mints:
                self.univ3_mints.append({
                    'txn_hash': txn_hash,
                    'block_number': transaction['block_number'],
                    'txn_index': transaction['txn_index'],
                    'log_index': mint.get('log_index'),
                    'to_address': mint.get('to_address'),
                    'amount': mint.get('amount'),
                    'token_address': mint.get('token_address')
                })

    def _add_univ3_burns(self, transaction: Dict):
        """Process Uniswap V3 burn events"""
        txn_hash = transaction['hash']
        v3_burns = transaction.get('uniswap_v3_burns', [])
        if v3_burns:
            for burn in v3_burns:
                self.univ3_burns.append({
                    'txn_hash': txn_hash,
                    'block_number': transaction['block_number'],
                    'txn_index': transaction['txn_index'],
                    'log_index': burn.get('log_index'),
                    'from_address': burn.get('from_address'),
                    'amount': burn.get('amount'),
                    'token_address': burn.get('token_address')
                })

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
    
    # --- Update _add_univ4_pool for Uniswap V4 Events ---
    def _add_univ4_pool(self, transaction: Dict):
        """Process Uniswap V4 pool initialization events and update pool info."""
        for pool_event in transaction.get('uniswap_v4_initializes', []):
            pool_address = pool_event.get('pool_manager_address')
            if pool_event.get('currency0') == self.contract_address:
                denom_address = pool_event.get('currency1')
                token1_is_denom = True
            else:
                denom_address = pool_event.get('currency0')
                token1_is_denom = False
            pool_is_valid = self.set_pool_info(pool_address, "V4", denom_address, token1_is_denom)
            if not pool_is_valid:
                return
            self.has_uni_v4_pool = True
            self.uniswap_v4_pool.append({
                'txn_hash': transaction['hash'],
                'block_number': transaction['block_number'],
                'txn_index': transaction['txn_index'],
                'log_index': pool_event.get('log_index'),
                'pool_address': pool_address,
                'currency0': pool_event.get('currency0'),
                'currency1': pool_event.get('currency1'),
                'fee': pool_event.get('fee')
            })
            self.set_lp_token_info(pool_address)

    def _add_univ4_swaps(self, transaction: Dict):
        """Process Uniswap V4 swap events."""
        txn_hash = transaction['hash']
        v4_swaps = transaction.get('uniswap_v4_swaps', [])
        if v4_swaps:
            self.univ4_swaps[txn_hash] = []
            #check if the pool address is in the pool_addresses list
            if not v4_swaps[0]["pool_manager_address"] in self.pool_addresses:
                return # We should have already added the pool address

            if not self.trading_enabled:
                self._update_trading_enabled(transaction)
            for swap in v4_swaps:
                self.univ4_swaps[txn_hash].append({
                    'log_index': swap.get('log_index'),
                    'sender': swap.get('sender'),
                    'amount0': swap.get('amount0'),
                    'amount1': swap.get('amount1'),
                    'sqrtPriceX96': swap.get('sqrt_price_x96'),
                    'liquidity': swap.get('liquidity'),
                    'tick': swap.get('tick'),
                    'fee': swap.get('fee'),
                    'pool_address': swap.get('pool_manager_address')
                })
                if swap.get('pool_manager_address') in self.pool_addresses:
                    price = float(swap.get('sqrt_price_x96'))
                    if swap.get('pool_manager_address') not in self.pool_prices:
                        self.pool_prices[swap.get('pool_manager_address')] = []
                    self.pool_prices[swap.get('pool_manager_address')].append(price)

    def _add_univ4_mints(self, transaction: Dict):
        """Process Uniswap V4 mint events."""
        txn_hash = transaction['hash']
        v4_mints = transaction.get('uniswap_v4_mints', [])
        if v4_mints:
            for mint in v4_mints:
                self.univ4_mints.append({
                    'txn_hash': txn_hash,
                    'block_number': transaction['block_number'],
                    'txn_index': transaction['txn_index'],
                    'log_index': mint.get('log_index'),
                    'to_address': mint.get('to_address'),
                    'amount': mint.get('amount'),
                    'token_address': mint.get('token_address')
                })

    def _add_univ4_burns(self, transaction: Dict):
        """Process Uniswap V4 burn events."""
        txn_hash = transaction['hash']
        v4_burns = transaction.get('uniswap_v4_burns', [])
        if v4_burns:
            for burn in v4_burns:
                self.univ4_burns.append({
                    'txn_hash': txn_hash,
                    'block_number': transaction['block_number'],
                    'txn_index': transaction['txn_index'],
                    'log_index': burn.get('log_index'),
                    'from_address': burn.get('from_address'),
                    'amount': burn.get('amount'),
                    'token_address': burn.get('token_address')
                })

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
        # Process pair events (Uniswap V2 pools)
        self._add_univ2_pair_event(transaction)
        # Process Uniswap V3 pool creation events
        self._add_univ3_pool(transaction)
        # Process Uniswap V4 pool initialization events
        self._add_univ4_pool(transaction)
        
        # Process ERC20 transfers
        self._add_transfers(transaction)
        
        # Process internal transfers
        self._add_eth_transfer(transaction)

        # Check the pool reserves
        self._check_pool_reserves(transaction)

        # Process approvals
        self._add_approval(transaction)

        # Process owner events
        self._add_owner_event(transaction)

        # Update address tx counter
        self.updated_address_tx_counter(transaction.get('unique_addresses', set()))

        # Update bribe amount
        self._update_bribe_amount(transaction)

        # Process Uniswap V2 events
        self._add_univ2_syncs(transaction)
        self._add_univ2_swaps(transaction)

        # Process Uniswap V3 events
        self._add_univ3_swaps(transaction)
        self._add_univ3_mints(transaction)
        self._add_univ3_burns(transaction)
        
        # Process Uniswap V4 events
        self._add_univ4_swaps(transaction)
        self._add_univ4_mints(transaction)
        self._add_univ4_burns(transaction)
        
        # Process trading enabled events
        if self.has_pool or not self.trading_enabled:
            self._add_trading_enabled_event(transaction)

        # Update latest block number and timestamp
        self.latest_block_number = transaction['block_number']
        self.latest_block_timestamp = transaction['block_timestamp']

