"""
Base pool class that manages its own events and state.

Each pool instance tracks its own events and updates its state accordingly.
"""

from abc import ABC, abstractmethod
from typing import Dict, List, Optional, Any, Tuple
from collections import deque
from dataclasses import dataclass, field
from web3 import Web3

from eth_block_processor.data_models.txn_models import ProcessedTransaction
from eth_block_processor.chain_utils.common_addresses import (
    DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, DENOM_NAMES_TO_ADDRESS,
    denominator_addresses_by_name, known_denom_decimals
)
from .pool_reserve_tracker import PoolReserveTracker


# Uniswap V2 Pair contract ABI for getReserves()
UNISWAP_V2_PAIR_ABI = [
    {
        "constant": True,
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"internalType": "uint112", "name": "_reserve0", "type": "uint112"},
            {"internalType": "uint112", "name": "_reserve1", "type": "uint112"},
            {"internalType": "uint32", "name": "_blockTimestampLast", "type": "uint32"}
        ],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token0",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token1",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    }
]

# Minimal ERC20 ABI for fetching decimals
ERC20_DECIMALS_ABI = [
    {
        "constant": True,
        "inputs": [],
        "name": "decimals",
        "outputs": [{"name": "", "type": "uint8"}],
        "type": "function"
    }
]


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
    
    def __init__(self, pool_address: str, token_address: str, 
                 denom_address: str, token1_is_denom: bool = True):
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
        self.token1_is_denom = token1_is_denom
            
        # Current state
        self.state = PoolState()
        
        # Event storage (bounded to prevent memory issues)
        self.sync_events = deque(maxlen=1000)
        self.swap_events = deque(maxlen=5000)
        self.mint_events = deque(maxlen=1000)
        self.burn_events = deque(maxlen=1000)
        
        # Keep track of price history list (each txn -> price)
        self.price_history = []
        
        # Creation info
        self.creation_block: Optional[int] = None
        self.creation_txn: Optional[str] = None
        
        # Trading enabled tracking
        self.trading_enabled = False
        self.trading_enabled_block = 0
        self.trading_enabled_timestamp = 0
        self.trading_enabled_txn = ''
        self.trading_enabled_event_index = 0
        
        # Web3 connection (lazy loaded)
        self._w3: Optional[Web3] = None
        
        # Token decimals (cached)
        self._token_decimals: Optional[int] = None
        self._denom_decimals: Optional[int] = None
        
        # Pool-specific reserve tracker
        self.reserve_tracker = PoolReserveTracker(
            pool_address=pool_address,
            denom_address=denom_address,
            logger=None  # Logger can be set later if needed
        )
        
        # Logger (can be set by subclasses)
        self.logger = None
        
        # Scam detection (from reserve tracker)
        self.scam_label: Optional[str] = None
        self.scam_block: Optional[int] = None        
        self.scam_tx_hash: Optional[str] = None
        
    @abstractmethod
    def get_protocol(self) -> str:
        """Get the protocol name (V2, V3, V4, etc.)."""
        pass
        
    @abstractmethod
    def process_transaction(self, transaction: ProcessedTransaction):
        """
        Process a transaction and extract relevant events.
        
        Each pool type knows how to extract its specific events.
        """
        pass
    
    @abstractmethod
    def get_reserves_from_blockchain(self, block_identifier='latest') -> Tuple[float, float, bool]:
        """
        Fetch actual pool reserves from the blockchain.
        
        Args:
            block_identifier: Block number or 'latest'
            
        Returns:
            Tuple of (denom_reserve, token_reserve, success)
        """
        pass
    
    def get_reserves(self) -> Tuple[float, float]:
        """Get current reserves."""
        return self.state.reserve0, self.state.reserve1
        
    def get_price(self) -> float:
        """Get current price of our token in terms of denom."""
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
            
    def update_reserves(self, reserve0: float, reserve1: float, block_number: int, 
                       timestamp: int = 0, tx_hash: str = ''):
        """Update pool reserves and calculate prices."""
        self.state.reserve0 = reserve0
        self.state.reserve1 = reserve1
        self.state.last_update_block = block_number
        
        # Calculate prices
        if reserve0 > 0:
            self.state.price0 = reserve1 / reserve0
        if reserve1 > 0:
            self.state.price1 = reserve0 / reserve1
            
        # Store price history
        price = self.get_price()
        if price > 0:
            self.price_history.append((block_number, price))
            
        # Update reserve tracker
        denom_reserve = self.get_denom_reserve()
        token_reserve = self.get_token_reserve()
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
        else:
            self.scam_label = None
            self.scam_block = None
            self.scam_tx_hash = None
    
    def _mark_trading_enabled(self, transaction: ProcessedTransaction):
        """Mark trading as enabled for this pool."""
        if not self.trading_enabled:
            self.trading_enabled = True
            self.trading_enabled_block = transaction.block_number
            self.trading_enabled_txn = transaction.hash
    
    def is_trading_enabled(self) -> bool:
        """Check if trading is enabled on this pool."""
        return self.trading_enabled
    
    def get_trading_enabled_info(self) -> Dict[str, Any]:
        """Get trading enabled information for this pool."""
        return {
            'enabled': self.trading_enabled,
            'block': self.trading_enabled_block,
            'txn': self.trading_enabled_txn
        }
    
    @property
    def is_scam(self) -> bool:
        """
        Check if this pool is a scam based on the reserve tracker's detection.
        """
        return self.reserve_tracker.is_scam
    
    @property
    def w3(self) -> Web3:
        """Get Web3 connection (lazy loaded)."""
        if self._w3 is None:
            self._w3 = Web3(Web3.HTTPProvider("http://127.0.0.1:8545"))
        return self._w3
    
    def get_token_decimals(self) -> int:
        """Get our token's decimals from the contract."""
        if self._token_decimals is None:
            try:
                token_contract = self.w3.eth.contract(address=self.token_address, abi=ERC20_DECIMALS_ABI)
                self._token_decimals = token_contract.functions.decimals().call()
            except Exception:
                # Default to 18 if decimals call fails
                self._token_decimals = 18
        return self._token_decimals
    
    def get_denom_decimals(self) -> int:
        """Get denomination token decimals."""
        if self._denom_decimals is None:
            if self.denom_address in DENOM_ADDRESSES:
                denom_name = DENOM_ADDRESSES[self.denom_address]
                self._denom_decimals = known_denom_decimals.get(denom_name)
            else:
                # For unknown tokens, fetch decimals from blockchain
                try:
                    denom_contract = self.w3.eth.contract(address=self.denom_address, abi=ERC20_DECIMALS_ABI)
                    self._denom_decimals = denom_contract.functions.decimals().call()
                except Exception as e:
                    # Log the error and return None
                    if hasattr(self, 'logger') and self.logger:
                        self.logger.error(f"Failed to fetch decimals for token {self.denom_address}: {e}")
                    else:
                        print(f"Failed to fetch decimals for token {self.denom_address}: {e}")
                    self._denom_decimals = None
        return self._denom_decimals
    
    def get_denom_name(self) -> str:
        """Get denomination token name."""
        # Convert to checksum address for lookup
        try:
            checksum_address = Web3.to_checksum_address(self.denom_address)
            return DENOM_ADDRESSES.get(checksum_address)
        except Exception:
            # Fallback to original address if checksum conversion fails
            return DENOM_ADDRESSES.get(self.denom_address)
    
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
    