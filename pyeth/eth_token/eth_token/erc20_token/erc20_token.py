from typing import Any, Dict, Iterable, List, Optional, Set
import enum

from eth_token.erc20_token.token_state.pool_state_bridge import PoolStateBridge
from eth_token.erc20_token.token_state.token_transfer_tracker import TokenTransferTracker
from eth_token.erc20_token.token_state.control_address_tracker import ControlAddressTracker
from eth_token.erc20_token.token_state.token_state_monitor import TokenStateMonitor
from eth_token.erc20_token.token_health.scam_thresholds import HIDDEN_MINTS_THRESHOLD
from eth_token.erc20_token.token_chain_data_fetcher import TokenChainDataFetcher
from eth_token.erc20_token.network.token_network import LiveTokenNetwork
from eth_token.erc20_token.token_health.token_health_predictor import TokenHealthPredictor


HISTORY_LIMIT = 1000


class TokenLifecycleState(enum.Enum):
    CREATION = "CONTRACT_CREATION"
    PAIR_CREATION = "PAIR_CREATION"
    TRADING_ENABLED = "TRADING_ENABLED"
    INACTIVE_SCAM = "INACTIVE_SCAM"
    INACTIVE_OTHER = "INACTIVE_OTHER"


class ERC20Token:
    """This class contains all the data and information of a token."""

    def __init__(self, contract_address: str, token_metadata=None):

        self.contract_address = contract_address
        self.token_chain_data_fetcher = TokenChainDataFetcher()
        if token_metadata is None:
            token_metadata = self.token_chain_data_fetcher.get_token_metadata(self.contract_address)            
        self.name = token_metadata.name
        self.symbol = token_metadata.symbol
        self.decimals = int(token_metadata.decimals)
        self.total_supply = int(token_metadata.total_supply)

        self.history_limit = HISTORY_LIMIT

        # Creation info
        self.creation_block: Optional[int] = None
        self.creation_timestamp: Optional[int] = None
        self.creation_tx: Optional[str] = None
        self.creator_address: Optional[str] = None
        self.creator_nonce: Optional[int] = None

        # Status flags
        self.token_life_cycle_status: Optional[TokenLifecycleState] = None
        self.tx_hashes_to_makers: Dict[str, str] = {}
        self.transaction_fees: List[Dict[str, Any]] = []

        # Bribe tracking
        self.total_bribe_amount: float = 0.0
        self.bribe_amount_dict: Dict[str, float] = {}

        # Latest block context
        self.latest_block_number: Optional[int] = None
        self.latest_block_timestamp: Optional[int] = None

        self.pool_state = PoolStateBridge(
            token_address=contract_address,
            history_limit=self.history_limit,
            token_decimals=self.decimals,
        )

        self.transfer_tracker = TokenTransferTracker(
            contract_address=contract_address,
            decimals=self.decimals,
            history_limit=self.history_limit,
            pool_manager=self.pool_state.pool_manager,
        )
        self.control_tracker = ControlAddressTracker(history_limit=self.history_limit)
        self.state_monitor = TokenStateMonitor(hidden_mint_threshold=HIDDEN_MINTS_THRESHOLD)
        self.token_network = LiveTokenNetwork(live_token=self)
        self.token_health_predictor = TokenHealthPredictor()

        self.pool_state.register_token_control_addresses(self.control_tracker.addresses)
        self._sync_token_decimals()
        self.latest_token_assessment: Optional[Dict[str, Any]] = None

    def update_from_transaction(self, transaction: Dict):
        if not transaction["status"]:
            return

        self.record_transaction_metadata(transaction)
        if self.is_contract_creation(transaction):
            self.handle_contract_creation(transaction)
            self._register_control_addresses(
                [
                    self.creator_address,
                    transaction.get('from_address'),
                ]
            )
            self._sync_token_decimals()

        self.pool_state.update_from_transaction(transaction)
        self.transfer_tracker.update_from_transaction(transaction)
        self.control_tracker.update_from_transaction(transaction)
        self.pool_state.register_token_control_addresses(self.control_tracker.addresses)
        self.state_monitor.update_from_transaction(transaction)
        self.update_bribe_amount(transaction)
        self.token_network.update_from_transaction(transaction)
        
        self.state_monitor.detect_hidden_mint(
            total_supply=self.total_supply,
            total_supply_from_transfers=self.total_supply_from_transfers,
            transaction=transaction,
        )

        self._update_lifecycle_status()

        self.latest_token_assessment = self.token_health_predictor.update_from_transaction(transaction, self)

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------
    def _register_control_addresses(self, addresses: Iterable[Optional[str]]) -> None:
        newly_added = self.control_tracker.register(addresses)
        if newly_added:
            self.pool_state.register_token_control_addresses(self.control_tracker.addresses)

    def _sync_token_decimals(self) -> None:
        if self.decimals is None:
            return
        self.pool_state.set_token_decimals(self.decimals)
        self.transfer_tracker.decimals = int(self.decimals)

    def _update_lifecycle_status(self) -> None:
        if self.is_scam:
            return
        if self.state_monitor.is_scam:
            self.token_life_cycle_status = TokenLifecycleState.INACTIVE_SCAM
        elif not self.is_scam and self.trading_enabled:
            self.token_life_cycle_status = TokenLifecycleState.TRADING_ENABLED
        if self.has_pool and self.token_life_cycle_status != TokenLifecycleState.TRADING_ENABLED:
            self.token_life_cycle_status = TokenLifecycleState.PAIR_CREATION

    def record_transaction_metadata(self, transaction: Dict) -> None:
        self.tx_hashes_to_makers[transaction['hash']] = transaction['from_address']
        self.latest_block_number = transaction['block_number']
        self.latest_block_timestamp = transaction['block_timestamp']

    def is_contract_creation(self, transaction: Dict) -> bool:
        tx_type = transaction.get('tx_type')
        if tx_type and tx_type.lower() == 'contract creation':
            return True
        return bool(transaction.get('contract_creation_events'))

    def handle_contract_creation(self, transaction: Dict) -> None:
        self.creation_block = transaction['block_number']
        self.creation_timestamp = transaction['block_timestamp']
        self.creation_tx = transaction['hash']
        self.creator_address = transaction['from_address']
        self.creator_nonce = transaction['nonce']
        self.token_life_cycle_status = TokenLifecycleState.CREATION

    def update_bribe_amount(self, transaction: Dict) -> None:
        bribe_amount = transaction.get('bribe_amount', 0) or 0
        if bribe_amount > 0:
            briber_address = transaction['from_address']
            self.bribe_amount_dict[briber_address] = bribe_amount
            self.total_bribe_amount += bribe_amount

    @property
    def token_creation_age_blocks(self):
        if self.latest_block_number is None or self.creation_block is None:
            return None
        return self.latest_block_number - self.creation_block

    @property
    def token_creation_age_hours(self):
        if (
            self.latest_block_timestamp is None
            or self.creation_timestamp is None
            or self.creation_timestamp == 0
        ):
            return None
        return (self.latest_block_timestamp - self.creation_timestamp) / 3600

    @property
    def pools(self):
        return self.pool_state.get_pool_collection()

    @property
    def liquidity_matrix(self):
        """Expose pool liquidity matrix directly on the token."""
        return self.pool_state.get_liquidity_matrix()

    @property
    def pool_manager(self):
        return self.pool_state.pool_manager

    @property
    def pool_addresses(self) -> tuple:
        return self.pool_state.get_pool_addresses()

    def get_pool_info_dict(self) -> Dict[str, Dict]:
        return self.pool_state.get_pool_info()

    def get_pool_token_reserve(self, pool_address: str):
        return self.pool_state.get_pool_token_reserve(pool_address)

    def get_pool_reserve(self, pool_address: str):
        return self.pool_state.get_pool_denom_reserve(pool_address)

    @property
    def has_pool(self) -> bool:
        return self.pool_state.has_pools()

    @property
    def all_pool_reserves(self) -> Dict[str, Dict[str, Any]]:
        return self.pool_state.get_all_pool_reserves()

    @property
    def total_liquidity_by_denom(self) -> Dict[str, float]:
        return self.pool_state.get_total_liquidity_by_denom()

    @property
    def current_prices(self) -> Dict[str, Dict[str, Any]]:
        return self.pool_state.get_current_prices()

    @property
    def pool_info(self) -> Dict:
        return self.pool_state.get_pool_info()

    @property
    def latest_pools_price_ratio(self) -> Dict[str, float]:
        return self.pool_state.get_latest_price_ratios()

    @property
    def token_control_addresses(self) -> Set[str]:
        return self.control_tracker.addresses

    @property
    def current_owner(self) -> Optional[str]:
        return self.control_tracker.current_owner

    @property
    def all_owners(self) -> List[Optional[str]]:
        return self.control_tracker.get_owner_history()

    @property
    def owner_events(self) -> List[Dict[str, Any]]:
        return self.control_tracker.get_owner_events()

    @property
    def ownership_renounced(self) -> bool:
        return self.control_tracker.ownership_renounced

    @property
    def ownership_renounced_block(self) -> Optional[int]:
        return self.control_tracker.renouncement_block

    @property
    def ownership_renounced_tx(self) -> Optional[str]:
        return self.control_tracker.renouncement_tx

    @property
    def renouncement_block(self) -> Optional[int]:
        return self.control_tracker.renouncement_block

    @property
    def renouncement_tx(self) -> Optional[str]:
        return self.control_tracker.renouncement_tx

    @property
    def renouncement_event(self) -> Optional[Dict[str, Any]]:
        return self.control_tracker.renouncement_event

    @property
    def renouncement_event_index(self) -> Optional[int]:
        return self.control_tracker.renouncement_event_index

    @property
    def renouncement_event_log_index(self) -> Optional[int]:
        return self.control_tracker.renouncement_event_log_index

    # ------------------------------------------------------------------
    # Transfer tracker proxies
    # ------------------------------------------------------------------

    @property
    def erc20_transfers(self) -> Dict[str, List[Dict[str, Any]]]:
        return self.transfer_tracker.erc20_transfers

    @property
    def eth_transfers(self) -> Dict[str, List[Dict[str, Any]]]:
        return self.transfer_tracker.eth_transfers

    @property
    def other_denom_transfers(self) -> Dict[str, List[Dict[str, Any]]]:
        return self.transfer_tracker.other_denom_transfers

    @property
    def approvals(self) -> List[Dict[str, Any]]:
        return self.transfer_tracker.approvals

    @property
    def approved_addresses(self) -> Set[str]:
        return self.transfer_tracker.approved_addresses

    @property
    def other_currencies(self) -> Dict[str, int]:
        return self.transfer_tracker.other_currencies

    @property
    def address_tx_counter(self) -> Dict[str, int]:
        return self.transfer_tracker.address_tx_counter

    @property
    def total_supply_from_transfers(self) -> float:
        return self.transfer_tracker.total_supply_from_transfers

    # ------------------------------------------------------------------
    # Governance / scam state proxies
    # ------------------------------------------------------------------
    @property
    def trading_enabled(self) -> bool:
        return self.state_monitor.trading_enabled

    @property
    def trading_enabled_block(self) -> Optional[int]:
        return self.state_monitor.trading_enabled_block

    @property
    def trading_enabled_timestamp(self) -> Optional[int]:
        return self.state_monitor.trading_enabled_timestamp

    @property
    def trading_enabled_tx(self) -> Optional[str]:
        return self.state_monitor.trading_enabled_tx

    @property
    def trading_enabled_event_index(self) -> Optional[int]:
        return self.state_monitor.trading_enabled_event_index

    @property
    def trading_enabled_event(self) -> Optional[Dict[str, Any]]:
        return self.state_monitor.trading_enabled_event

    @property
    def tax_event(self) -> Optional[Dict[str, Any]]:
        return self.state_monitor.tax_event

    @property
    def tax_event_block(self) -> Optional[int]:
        return self.state_monitor.tax_event_block

    @property
    def tax_event_tx(self) -> Optional[str]:
        return self.state_monitor.tax_event_tx

    @property
    def tax_event_index(self) -> Optional[int]:
        return self.state_monitor.tax_event_index

    @property
    def tax_event_log_index(self) -> Optional[int]:
        return self.state_monitor.tax_event_log_index

    @property
    def max_buy_limit(self) -> Optional[int]:
        return self.state_monitor.max_buy_limit

    @property
    def max_buy_limit_block(self) -> Optional[int]:
        return self.state_monitor.max_buy_limit_block

    @property
    def max_buy_limit_tx(self) -> Optional[str]:
        return self.state_monitor.max_buy_limit_tx

    @property
    def max_buy_limit_index(self) -> Optional[int]:
        return self.state_monitor.max_buy_limit_index

    @property
    def max_buy_limit_log_index(self) -> Optional[int]:
        return self.state_monitor.max_buy_limit_log_index

    @property
    def max_buy_ratio(self) -> Optional[int]:
        return self.state_monitor.max_buy_ratio

    @property
    def max_buy_ratio_block(self) -> Optional[int]:
        return self.state_monitor.max_buy_ratio_block

    @property
    def max_buy_ratio_tx(self) -> Optional[str]:
        return self.state_monitor.max_buy_ratio_tx

    @property
    def max_buy_ratio_index(self) -> Optional[int]:
        return self.state_monitor.max_buy_ratio_index

    @property
    def max_buy_ratio_log_index(self) -> Optional[int]:
        return self.state_monitor.max_buy_ratio_log_index

    @property
    def is_scam(self) -> bool:
        return self.state_monitor.is_scam

    @property
    def scam_label(self) -> Optional[str]:
        return self.state_monitor.scam_label if self.state_monitor.is_scam else None

    @property
    def scam_block(self) -> Optional[int]:
        return self.state_monitor.scam_block if self.state_monitor.is_scam else None

    @property
    def scam_tx(self) -> Optional[str]:
        return self.state_monitor.scam_tx if self.state_monitor.is_scam else None

    @property
    def scam_reason(self) -> str:
        return self.token_health_predictor.scam_reason

    @property
    def fee_sources(self) -> List[str]:
        return list(set(self.tx_hashes_to_makers.values()))

    @property
    def tx_hashes(self) -> List[str]:
        return list(set(self.tx_hashes_to_makers.keys()))

    def get_address_tx_hashes(self, address: str) -> List[str]:
        return [tx_hash for tx_hash, maker in self.tx_hashes_to_makers.items() if maker == address]

    @property
    def unique_addresses(self) -> List[str]:
        return list(self.address_tx_counter.keys())
    
    def get_token_summary(self) -> Dict[str, Any]:
        summary = {
            'contract_address': self.contract_address,
            'name': self.name,
            'symbol': self.symbol,
            'decimals': self.decimals,
            'total_supply': self.total_supply,
            'token_life_cycle_status': self.token_life_cycle_status.value if self.token_life_cycle_status else None,
            'is_scam': self.is_scam,
            'scam_label': self.scam_label,
            'creation_block': self.creation_block,
            'creator_address': self.creator_address,
            'current_owner': self.current_owner,
            'has_pools': self.has_pool,
            'pool_count': len(self.pool_addresses),
            'protocols': list(set(info['protocol'] for info in self.current_prices.values())),
            'total_transactions': len(self.tx_hashes),
            'unique_addresses': len(self.unique_addresses),
            'total_transfers': sum(len(transfers) for transfers in self.erc20_transfers.values()),
            'latest_block': self.latest_block_number,
            'latest_timestamp': self.latest_block_timestamp,
        }
        if self.has_pool:
            summary.update(
                {
                    'total_liquidity_by_denom': self.total_liquidity_by_denom,
                    'current_prices': self.current_prices,
                }
            )
        return summary
    
    def to_dict(self):
        return {
            'token_data': self._token_snapshot(),
            'latest_token_assessment': self.token_health_predictor.to_dict(),
            'is_scam': self.is_scam,
            'scam_reason': self.scam_reason,
        }

    def _token_snapshot(self) -> Dict[str, Any]:
        excluded = {
            'pool_state',
            'transfer_tracker',
            'control_tracker',
            'state_monitor',
            'token_network',
            'token_health_predictor',
            'latest_token_assessment',
            'token_data',
        }
        data = {k: v for k, v in self.__dict__.items() if k not in excluded}
        data.update(
            {
                'owner_events': self.owner_events,
                'all_owners': self.all_owners,
                'current_owner': self.current_owner,
                'token_control_addresses': list(self.token_control_addresses),
                'ownership_renounced': self.ownership_renounced,
                'ownership_renounced_block': self.ownership_renounced_block,
                'ownership_renounced_tx': self.ownership_renounced_tx,
                'renouncement_block': self.renouncement_block,
                'renouncement_tx': self.renouncement_tx,
                'renouncement_event': self.renouncement_event,
                'renouncement_event_index': self.renouncement_event_index,
                'renouncement_event_log_index': self.renouncement_event_log_index,
                'trading_enabled': self.trading_enabled,
                'trading_enabled_block': self.trading_enabled_block,
                'trading_enabled_timestamp': self.trading_enabled_timestamp,
                'trading_enabled_tx': self.trading_enabled_tx,
                'trading_enabled_event_index': self.trading_enabled_event_index,
                'trading_enabled_event': self.trading_enabled_event,
                'tax_event': self.tax_event,
                'tax_event_block': self.tax_event_block,
                'tax_event_tx': self.tax_event_tx,
                'tax_event_index': self.tax_event_index,
                'tax_event_log_index': self.tax_event_log_index,
                'max_buy_limit': self.max_buy_limit,
                'max_buy_limit_block': self.max_buy_limit_block,
                'max_buy_limit_tx': self.max_buy_limit_tx,
                'max_buy_limit_index': self.max_buy_limit_index,
                'max_buy_limit_log_index': self.max_buy_limit_log_index,
                'max_buy_ratio': self.max_buy_ratio,
                'max_buy_ratio_block': self.max_buy_ratio_block,
                'max_buy_ratio_tx': self.max_buy_ratio_tx,
                'max_buy_ratio_index': self.max_buy_ratio_index,
                'max_buy_ratio_log_index': self.max_buy_ratio_log_index,
                'is_scam': self.is_scam,
                'scam_label': self.scam_label,
                'scam_block': self.scam_block,
                'scam_tx': self.scam_tx,
            }
        )
        return data
    
    def __getitem__(self, item: str):
        if item in self.__dict__:
            return self.__dict__[item]
        raise KeyError(f"'{self.__class__.__name__}' has no attribute '{item}'")
