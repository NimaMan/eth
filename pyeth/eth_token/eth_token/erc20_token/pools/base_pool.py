"""
Base pool class that manages its own events and state.

Each pool instance tracks its own events and updates its state accordingly.
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Optional, Any, Tuple, Iterable, Set
from dataclasses import dataclass
from enum import Enum

from eth_token.erc20_token.pools.pool_reserve_tracker import PoolReserveTracker, logger
from eth_token.erc20_token.pools.pool_chain_data_fetcher import PoolChainDataFetcher
from eth_token.erc20_token.data.token_chain_data_fetcher import TokenChainDataFetcher
from eth_data.utils.pyreth_client import PyrethClient, pyreth
from eth_data.chain_utils.common_addresses import DENOM_ADDRESSES, ZERO_ADDRESS
from eth_token.erc20_token.config.scam_thresholds import get_threshold_for_token


class PoolLifecycle(str, Enum):
    """Lifecycle stages shared across Python, Rust, and analytics."""

    DISCOVERED = "DISCOVERED"   # Pool observed on-chain (deployment), zero liquidity
    LIQUIDITY_DEPOSITED = "LIQUIDITY_DEPOSITED"  # Liquidity deposited (non-zero reserves)
    ACTIVE = "ACTIVE"           # Buy/sell viability confirmed
    SCAM = "SCAM"               # Rug detected (Python flag or reserve tracker)
    EVICTED = "EVICTED"         # Removed from active tracking


@dataclass
class PoolState:
    """Current state of a pool."""
    reserve0: float = 0.0
    reserve1: float = 0.0
    total_liquidity: float = 0.0
    price0: float = 0.0  # token1 per token0
    price1: float = 0.0  # token0 per token1
    last_update_block: int = 0
    last_sync_block: int = 0
    lifecycle: PoolLifecycle = PoolLifecycle.DISCOVERED
    can_buy: bool = False
    can_sell: bool = False
    
    # Cumulative volumes
    volume0_in: float = 0.0
    volume1_in: float = 0.0
    volume0_out: float = 0.0
    volume1_out: float = 0.0
    
    # Liquidity events
    total_mints: int = 0
    total_burns: int = 0
    total_swaps: int = 0


class BasePool(ABC):
    """
    Base class for all pool types.
    
    Each pool manages its own events and state updates.
    """
    # Default ETH amount used for viability test buys in simulations
    DEFAULT_TEST_BUY_ETH: float = 0.01
    
    def __init__(
        self,
        pool_address: str,
        token_address: str,
        denom_address: str,
        *,
        token_decimals: int,
        denom_decimals: Optional[int] = None,
        history_limit: int = 100,
        token1_is_denom: bool = None,
        pool_chain_fetcher: Optional[PoolChainDataFetcher] = None,
        token_chain_fetcher: Optional[TokenChainDataFetcher] = None,
    ):
        """
        Initialize a pool.
        
        Args:
            pool_address: The pool contract address
            token_address: Our token's address
            denom_address: The paired token address (WETH, USDC, etc.)
            token1_is_denom: Whether token1 is the denomination token
        """
        self.pool_address = pool_address
        self.token_address = token_address
        self.denom_address = denom_address
        self.denom_threshold = get_threshold_for_token(self.denom_address)
        
        self.token1_is_denom = token1_is_denom
        self.history_limit = history_limit
            
        # Current state
        self.state = PoolState()
        
        # Event storage (bounded to prevent memory issues)
        self.sync_events: List[Dict[str, Any]] = []
        self.swap_events: List[Dict[str, Any]] = []
        self.mint_events: List[Dict[str, Any]] = []
        self.burn_events: List[Dict[str, Any]] = []
        
        # Keep track of price history list (each tx -> price)
        self.price_history = []
        
        # Creation info
        self.creation_block: Optional[int] = None
        self.creation_tx: Optional[str] = None
        self.creation_timestamp: Optional[int] = None
        
        # Trading capability tracking (internal naming for clarity)
        self.can_buy: bool = False
        self.can_buy_block: Optional[int] = None  # Maps to DB: trading_enabled_block
        self.can_buy_tx: Optional[str] = None    # Maps to DB: trading_enabled_tx
        self.can_buy_timestamp: Optional[int] = None
        
        # Runtime-only fields (not persisted to DB)
        self.can_sell: bool = False  # For honeypot detection
        self.buy_tax: Optional[float] = None
        self.sell_tax: Optional[float] = None
        self.tax_check_block: Optional[int] = None
        self.tax_check_tx: Optional[str] = None
        
        # Token decimals (cached)
        self._token_decimals: Optional[int] = int(token_decimals)
        self._denom_decimals: Optional[int] = (
            int(denom_decimals) if denom_decimals is not None else None
        )

        # Simulation settings
        # Per-pool override for test buy amount used in viability checks
        self.test_buy_amount_eth: float = self.DEFAULT_TEST_BUY_ETH
        
        # Shared chain data helpers
        self.pool_chain_fetcher = pool_chain_fetcher or PoolChainDataFetcher()
        self.token_chain_fetcher = token_chain_fetcher or TokenChainDataFetcher()
        self.pyreth_client = PyrethClient.instance()
        self.pool_buy_sell_simulator = self.pyreth_client.pool_buy_sell_simulator()
        self.pool_buy_sell_config = pyreth.PoolBuySellParameters()
        
        # Scam detection (from reserve tracker)
        self.scam_label: Optional[str] = None
        self.scam_block: Optional[int] = None        
        self.scam_tx_hash: Optional[str] = None

        # Reserve tracker with bounded history
        self.reserve_tracker = PoolReserveTracker(
            pool_address=pool_address,
            denom_address=denom_address,
            token_address=token_address,
            pool_type=self.get_protocol(),
            history_limit=history_limit,
        )
        self.token_control_addresses: Set[str] = set()
        self._latest_block_number: Optional[int] = None
        self.latest_block_control_address_txs: Dict[str, Dict[str, Any]] = {}

    @abstractmethod
    def get_protocol(self) -> str:
        """Get the protocol name (V2, V3, V4, etc.)."""
        pass
        
    @abstractmethod
    def process_transaction(self, transaction: Dict):
        """
        Process a transaction and extract relevant events. Each pool type knows how to extract its specific events.
        """
        pass

    def get_reserves(self) -> Tuple[float, float]:
        """Get current reserves."""
        return self.state.reserve0, self.state.reserve1
        
    def get_price(self) -> float:
        """Get current price of our token in terms of denom."""
        # Return zero if the pool is scam
        if self.is_scam:
            return 0.0
        if self.token1_is_denom:
            # Our token is token0, denom is token1
            # Price = denom_per_token = reserve1 / reserve0
            return self.state.price0 if self.state.price0 > 0 else 0
        else:
            # Our token is token1, denom is token0
            # Price = denom_per_token = reserve0 / reserve1
            return self.state.price1 if self.state.price1 > 0 else 0
            
    def get_token_reserve(self) -> float:
        """Get our token's reserve."""
        if self.token1_is_denom:
            return self.state.reserve0
        else:
            return self.state.reserve1
            
    def get_denom_reserve(self) -> float:
        """Get denomination token's reserve."""
        if self.token1_is_denom:
            return self.state.reserve1
        else:
            return self.state.reserve0

    def _append_event(self, collection: List[Any], entry: Any) -> None:
        collection.append(entry)
        if len(collection) > self.history_limit:
            del collection[: len(collection) - self.history_limit]
            
    def update_reserves(self, reserve0: float, reserve1: float, block_number: int, 
                       timestamp: int = 0, tx_hash: str = ''):
        """Update pool reserves and calculate prices."""
        self.state.reserve0 = reserve0
        self.state.reserve1 = reserve1
        self.state.last_update_block = block_number
        # Calculate prices
        if reserve0 > 0:
            self.state.price0 = reserve1 / reserve0
        else:
            self.state.price0 = 0.0
        if reserve1 > 0:
            self.state.price1 = reserve0 / reserve1
        else:
            self.state.price1 = 0.0
            
        # Store price history
        price = self.get_price()
        if price > 0:
            self._append_event(self.price_history, (block_number, price))
            
        # Update reserve tracker
        denom_reserve = self.get_denom_reserve()
        token_reserve = self.get_token_reserve()
        if denom_reserve >= self.denom_threshold:
            self.state.total_liquidity = denom_reserve
        else:
            self.state.total_liquidity = 0.0

        self.reserve_tracker.update_reserves(
            denom_reserve=denom_reserve,
            token_reserve=token_reserve,
            price=price,
            block_number=block_number,
            timestamp=timestamp,
            tx_hash=tx_hash
        )
        
        # Sync scam detection from tracker
        if self.reserve_tracker.is_scam:
            self.scam_label = self.reserve_tracker.scam_label
            self.scam_block = self.reserve_tracker.scam_block
            self.scam_tx_hash = self.reserve_tracker.scam_tx_hash
            self.state.lifecycle = PoolLifecycle.SCAM
        else:
            self.scam_label = None
            self.scam_block = None
            self.scam_tx_hash = None
            if (
                denom_reserve >= self.denom_threshold
                and self.state.lifecycle == PoolLifecycle.DISCOVERED
            ):
                self.state.lifecycle = PoolLifecycle.LIQUIDITY_DEPOSITED

    def mark_can_buy_from_event(self, transaction: Dict, event_type: str = 'swap'):
        """Mark token as buyable when detected from a DEX event (typically first swap)."""
        if not self.can_buy:
            self.can_buy = True
            self.state.can_buy = True
            self.can_buy_block = transaction['block_number']
            self.can_buy_tx = transaction['hash']
            # Persist when the first buy was observed on-chain
            self.can_buy_timestamp = transaction['block_timestamp']
            self.state.lifecycle = PoolLifecycle.ACTIVE
            
    def evaluate_trading_status(self, transaction: Dict) -> None:
        """Pool-specific hook implemented by subclasses to enable trading."""
        return
    
    def is_trading_enabled(self) -> bool:
        return self.trading_enabled
    
    def get_trading_status(self) -> Dict[str, Any]:
        """Get comprehensive trading status information."""
        return {
            'can_buy': self.can_buy,
            'can_sell': self.can_sell,
            'trading_enabled': self.trading_enabled,
            'can_buy_and_sell': self.can_buy_and_sell,
            'block': self.trading_enabled_block,
            'tx': self.trading_enabled_tx,
            'buy_tax': self.buy_tax,
            'sell_tax': self.sell_tax,
            'tax_check_block': self.tax_check_block,
            'tax_check_tx': self.tax_check_tx
        }
    
    @property
    def is_scam(self) -> bool:
        return self.reserve_tracker.is_scam

    @property
    def lifecycle(self) -> PoolLifecycle:
        return self.state.lifecycle

    def get_token_decimals(self) -> int:
        if self._token_decimals is None:
            self._token_decimals = int(
                self.token_chain_fetcher.get_token_decimals(self.token_address)
            )
        return self._token_decimals

    def get_denom_decimals(self) -> int:
        if self._denom_decimals is None:
            self._denom_decimals = int(
                self.token_chain_fetcher.get_token_decimals(self.denom_address)
            )
        return self._denom_decimals
        
    def get_denom_name(self) -> str:
        """Get denomination token name."""
        if self.denom_address == ZERO_ADDRESS:
            # V4 pools encode the native currency with the zero address. Treat it as ETH.
            return "ETH"
        name = DENOM_ADDRESSES.get(self.denom_address)
        if name:
            return name
        symbol = self.token_chain_fetcher.get_token_symbol(self.denom_address)
        return symbol
    
    def pool_age_blocks(self, current_block: int) -> Optional[int]:
        if self.creation_block is None:
            return None
        return current_block - self.creation_block
    
    def trading_age_blocks(self, current_block: int) -> Optional[int]:
        if not self.can_buy or not self.can_buy_block:
            return None
        return current_block - self.can_buy_block
    
    def pool_age_hours(self) -> Optional[float]:        
        if self.creation_timestamp is None or self.creation_timestamp == 0:
            return None
        current_timestamp = self.pyreth_client.get_current_block_timestamp()
        return (current_timestamp - self.creation_timestamp) / 3600

    def trading_age_hours(self) -> Optional[float]:
        current_timestamp = self.pyreth_client.get_current_block_timestamp()
        if not self.can_buy or not self.can_buy_timestamp:
            return None
        return (current_timestamp - self.can_buy_timestamp) / 3600

    @property
    def trading_enabled(self) -> bool:
        """Token is tradeable if can_buy is True."""
        return self.can_buy
    
    @property
    def trading_enabled_block(self) -> Optional[int]:
        """Maps to database column trading_enabled_block."""
        return self.can_buy_block
    
    @property
    def trading_enabled_tx(self) -> Optional[str]:
        """Maps to database column trading_enabled_tx."""
        return self.can_buy_tx
    
    @property
    def can_buy_and_sell(self) -> bool:
        """True if token can be both bought AND sold (not a honeypot)."""
        return self.can_buy and self.can_sell
    
    def check_and_update_trading_status(self, transaction: Dict) -> bool:
        """Check if trading is enabled on this pool and calculate taxes."""
        if self._has_control_address(transaction):
            self.evaluate_trading_status(transaction)   
        return self.trading_enabled                   

    def register_token_control_addresses(self, addresses: Iterable[Optional[str]]) -> None:
        for address in addresses:
            self.token_control_addresses.add(address)

    def _has_control_address(self, transaction: Dict)  -> bool:
        unique_addresses = set(transaction.get('unique_addresses') or [])
        if not unique_addresses or not self.token_control_addresses:
            return False
        return bool(self.token_control_addresses.intersection(unique_addresses))

    def update_latest_block_control_address_transactions(self, transaction: Dict):
        # Update latest block transactions involving control addresses
        if not self._has_control_address(transaction):
            return        
        block_number = transaction.get('block_number')
        # Reset if new block
        if self._latest_block_number != block_number:
            self._latest_block_number = block_number
            self.latest_block_control_address_txs = {}
        self.latest_block_control_address_txs[transaction.get('hash')] = transaction

    def get_stats(self) -> Dict[str, Any]:
        """Get pool statistics."""
        return {
            'protocol': self.get_protocol(),
            'address': self.pool_address,
            'token_reserve': self.get_token_reserve(),
            'denom_reserve': self.get_denom_reserve(),
            'price': self.get_price(),
            'total_swaps': self.state.total_swaps,
            'total_mints': self.state.total_mints,
            'total_burns': self.state.total_burns,
            'last_update_block': self.state.last_update_block,
        }
