"""
LiveTokenData: Data structure for real-time token state

Objective:
---------
1. Store and manage token state data
2. Track token events and metrics
3. Maintain historical data
4. Provide efficient data access patterns
"""
from datetime import datetime
import pandas as pd
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional

from eth_tokens_live.live_erc20_token.data.token_approval_sync_data import UniV2PairSyncData, ERC20TokenApprovalData
from eth_tokens_live.utils.common_addresses import names_by_address, addresses_by_name, known_denom_decimals
from eth_block_processor.contracts.contract_type import get_erc20_contract_info
from eth_tokens_live.utils.logger import get_logger


logger = get_logger(name="live_token_data", log_folder="tokens_live")


@dataclass
class LiveTokenData:
    contract_address: str  
    
    # Core token info
    name: Optional[str] = None
    symbol: Optional[str] = None
    decimals: Optional[int] = None
    total_supply: Optional[int] = 0
    total_supply_from_transfers: Optional[int] = 0
    
    # Creation info
    creation_block: Optional[int] = None
    creation_txn: Optional[str] = None
    creator_address: Optional[str] = None
    creator_nonce: Optional[int] = None
    creation_time: Optional[datetime] = None
    # Contract state
    trading_enabled: bool = False
    trading_enabled_block: Optional[int] = None
    trading_enabled_txn: Optional[str] = None
    
    # Event collections with proper typing
    erc20_transfers: List[Dict] = field(default_factory=list)
    eth_transfers: List[Dict] = field(default_factory=list)
    liquidity_token_transfers: List[Dict] = field(default_factory=list)
    other_denom_transfers: List[Dict] = field(default_factory=list)
    approvals: List[Dict] = field(default_factory=list)
    syncs: List[Dict] = field(default_factory=list)
    mints: List[Dict] = field(default_factory=list)
    burns: List[Dict] = field(default_factory=list)
    
    # Pair info
    pair_events: List[Dict] = field(default_factory=list)
    pair_addresses: Set[str] = field(default_factory=set)
    lp_token_info: Optional[Dict] = None
    lp_token_name: Optional[str] = None
    lp_token_symbol: Optional[str] = None
    lp_token_decimals: Optional[int] = None
    
    liquidity_token_supply: Optional[float] = 0
    liquidity_token_supply_from_transfers: Optional[float] = 0

    # Ownership info
    owner_events: List[Dict] = field(default_factory=list)
    current_owner: Optional[str] = None
    all_owners: List[str] = field(default_factory=list)
    
    # Transaction fees
    transaction_fees: List[Dict] = field(default_factory=list)
    
    # Tracked addresses
    all_denom_addresses: Set[str] = field(default_factory=set)
    approved_addresses: Set[str] = field(default_factory=set)
    unique_addresses: Set[str] = field(default_factory=set)
    total_bribe_amount: float = 0
    bribe_amount_dict: Dict[str, float] = field(default_factory=dict)
    
    def _handle_creation(self, transaction: Dict):
        """Process contract creation event"""
        self.contract_address = transaction['contract_address']
        self.creation_block = transaction['block_number']
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

    def _add_pair_event(self, transaction: Dict):
        """Process a pair event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
       
        for pair_event in transaction.get('pair_events', []):
            self.pair_addresses.add(pair_event['pair_address'])
            self.has_uni_v2_pair = True
            if pair_event['token0'] == self.contract_address:
                self.denom_address = pair_event['token1']
                self.token1_is_denom = True
            else:
                self.denom_address = pair_event['token0']
                self.token1_is_denom = False
            self.all_denom_addresses.add(names_by_address[self.denom_address])
            self.lp_token_info = get_erc20_contract_info(pair_event['pair_address'])
            if self.lp_token_info is not None:
                self.lp_token_name = self.lp_token_info['name']
                self.lp_token_symbol = self.lp_token_info['symbol']
                self.lp_token_decimals = self.lp_token_info['decimals']
                self.liquidity_token_supply = self.lp_token_info['total_supply']

            self.pair_events.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': pair_event['log_index'],
                'pair_address': pair_event['pair_address'],
                'token0': pair_event['token0'],
                'token1': pair_event['token1']
            })
        
    def _add_trading_enabled_event(self, transaction: Dict):
        """Process a trading enabled event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for trading_enabled_event in transaction.get('trading_enabled_events', []):
            self.trading_enabled = True
            self.trading_enabled_block = block_number
            self.trading_enabled_txn = txn_hash
            self.trading_enabled_event = trading_enabled_event
            self.trading_enabled_event_index = txn_index

    def _add_erc20_transfer(self, transaction: Dict, transfer: Dict):
        """Process a transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        amount = int(transfer['amount'])/10**self.decimals
        self.erc20_transfers.append({
            'txn_hash': txn_hash,
            'block_number': block_number,
            'txn_index': txn_index,
            'log_index': transfer['log_index'],
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': amount,
            'token_address': transfer['token_address']
        })
        if transfer['from_address'] == '0x0000000000000000000000000000000000000000':
            self.total_supply_from_transfers += amount
        
    def _add_weth_transfer(self, transaction: Dict, transfer: Dict):
        """Process an ETH transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        self.eth_transfers.append({
            'txn_hash': txn_hash,
            'block_number': block_number,
            'txn_index': txn_index,
            'log_index': transfer['log_index'],
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': float(transfer['amount'])/10**18,
            'token_address': transfer['token_address']
        })

    def _add_liquidity_token_transfer(self, transaction: Dict, transfer: Dict):
        """Process a liquidity token transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        decimals = None
        if self.lp_token_info is not None:
            decimals = 10**self.lp_token_info['decimals']
        
        self.liquidity_token_transfers.append({
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

    def _add_eth_transfer(self, transaction: Dict):
        """Process an internal transfer event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for transfer in transaction.get('internal_transfers', []):
            self.eth_transfers.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': transfer['log_index'],
                'from_address': transfer['from_address'],
                'to_address': transfer['to_address'],
                'amount': float(transfer['value']),
                'token_address': transfer['token_address'] if 'token_address' in transfer else "ETH"
            })

    def _add_other_token_transfer(self, transaction: Dict, transfer: Dict):
        """Process transfers of other known tokens (USDC, USDT, etc.)"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        if transfer['token_address'] in names_by_address.keys():
            denom_name = names_by_address[transfer['token_address']]
            denom_decimals = known_denom_decimals[denom_name]
            amount = float(transfer['amount'])/10**denom_decimals
            self.all_denom_addresses.add(denom_name)
        
            self.other_denom_transfers.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': transfer['log_index'],
                'from_address': transfer['from_address'],
                'to_address': transfer['to_address'],
                'amount': amount,
                'token_address': transfer['token_address'],
            })
        else:
            logger.warning(f"Different tokens being transfered {transfer['token_address']}, txn {transaction['hash']}")

    def _add_transfers(self, transaction: Dict):
        """Process transfer events for all relevant tokens"""
        # Track all transfers in the transaction
        for transfer in transaction.get('erc20_transfers', []):
            try:
                token_address = transfer['token_address']
                
                if token_address == self.contract_address:
                    # Our token transfers
                    self._add_erc20_transfer(transaction, transfer)
                    
                elif token_address in self.pair_addresses:
                    # LP token transfers
                    self._add_liquidity_token_transfer(transaction, transfer)
                    
                elif token_address == addresses_by_name['WETH']:
                    # WETH transfers
                    self._add_weth_transfer(transaction, transfer)
                    
                elif token_address in addresses_by_name.values():
                    # Other known token transfers (USDC, USDT, etc.)
                    self._add_other_token_transfer(transaction, transfer)      
            except Exception as e:
                logger.error(f" {__name__} Error processing transfer {token_address}, txn {transaction['hash']}: {e}")

    def _add_syncs(self, transaction: Dict):
        """Process a sync event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        timestamp = transaction['block_timestamp']
        for sync in transaction.get('uniswap_v2_syncs', []):
            if self.token1_is_denom:
                denom_name = names_by_address[self.denom_address]
                denom_reserve = float(sync['reserve1'])/10**known_denom_decimals[denom_name]
                token_reserve = float(sync['reserve0'])/10**self.decimals
            else:
                denom_name = names_by_address[self.denom_address]
                denom_reserve = float(sync['reserve0'])/10**known_denom_decimals[denom_name]
                token_reserve = float(sync['reserve1'])/10**self.decimals
            self.syncs.append({
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

    def _add_mint(self, transaction: Dict):
        """Process a mint event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        
        for mint in transaction.get('uniswap_v2_mints', []):
            self.mints.append({
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
                'spender': approval['spender'],
                'amount': float(approval['amount'])/10**self.decimals,
                'token_address': approval['token_address']
            })
            self.approved_addresses.add(approval['spender'])

    def _add_burn(self, transaction: Dict):
        """Process a burn event"""
        txn_hash = transaction['hash']
        block_number = transaction['block_number']
        txn_index = transaction['txn_index']
        for burn in transaction.get('uniswap_v2_burns', []):
            self.burns.append({
                'txn_hash': txn_hash,
                'block_number': block_number,
                'txn_index': txn_index,
                'log_index': burn['log_index'],
                'from_address': burn['from_address'],
                'amount': burn['amount'],
                'token_address': burn['token_address']
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
            if event['token_address'].lower() == self.contract_address.lower():
                self.trading_enabled = True
                self.trading_enabled_block = transaction['block_number']
                self.trading_enabled_txn = transaction['hash']

    def _update_bribe_amount(self, transaction: Dict):
        """Update the bribe amount"""
        bribe_amount = transaction['bribe_amount']
        briber_address = transaction['from_address']
        self.bribe_amount_dict[briber_address] = bribe_amount
        self.total_bribe_amount += bribe_amount

    def _sort_df(self, df: pd.DataFrame) -> pd.DataFrame:
        """Sort the dataframe"""
        if not df.empty:
            df = df.sort_values(by=['block_number', 'txn_index', 'log_index']).reset_index(drop=True)
            df = df.set_index('txn_hash')
        return df
    
    @property
    def erc20_transfer_df(self):
        """Get the transfer dataframe"""
        return self._sort_df(pd.DataFrame(self.erc20_transfers))

    @property
    def eth_transfer_df(self):
        """Get the ETH transfer dataframe"""
        return self._sort_df(pd.DataFrame(self.eth_transfers))
    
    @property
    def liquidity_token_transfer_df(self):
        """Get the liquidity token transfer dataframe"""
        return self._sort_df(pd.DataFrame(self.liquidity_token_transfers))

    @property
    def liquidity_token_mint_df(self):
        """Get the liquidity token mint dataframe"""
        return self._sort_df(pd.DataFrame(self.mints))

    @property
    def liquidity_token_burn_df(self):
        """Get the liquidity token burn dataframe"""
        return self._sort_df(pd.DataFrame(self.burns))

    @property
    def liquidity_token_lock_df(self):
        """Get the liquidity token lock dataframe"""
        lock_df = pd.DataFrame(self.locks)
        lock_df['duration'] = lock_df['unlock_at'] - lock_df['timestamp']
     
        return self._sort_df(lock_df)

    @property
    def sync_data(self):
        """Get the sync data"""
        return UniV2PairSyncData(self.syncs)

    @property
    def price_df(self):
        """Get the price dataframe"""
        return UniV2PairSyncData(self.syncs).to_dataframe()
    
    @property
    def approval_df(self):
        """Get the approval dataframe"""
        return ERC20TokenApprovalData(self.approvals).to_dataframe()
    
    def to_dict(self):
        return self.__dict__
    
    def update_from_transaction(self, transaction: Dict):
        """Update token data from a new transaction"""
        # Handle contract creation
        if transaction['txn_type'] == 'Contract Creation':
            self._handle_creation(transaction)
        
        # Process pair events
        self._add_pair_event(transaction)
        
        # Process trading enabled events
        self._add_trading_enabled_event(transaction)

        # Process ERC20 transfers
        self._add_transfers(transaction)
        
        # Process internal transfers
        self._add_eth_transfer(transaction)

        # Process approvals
        self._add_approval(transaction)

        # Process owner events
        self._add_owner_event(transaction)

        # Process sync events
        self._add_syncs(transaction)

        # Update unique addresses
        self.unique_addresses.update(transaction.get('unique_addresses', set()))

        # update bribe amount
        self._update_bribe_amount(transaction)
