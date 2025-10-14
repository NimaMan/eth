"""
Live ERC20 Token Objects Cache Implementation
--------------------------------
Provides an LRU-based caching mechanism for active token objects with the following features:
- Fixed-size cache with LRU eviction policy
- Thread-safe operations
- Automatic cache invalidation based on age
- Error handling and logging
- Memory usage monitoring
"""
import time
from dataclasses import dataclass
from collections import OrderedDict
from typing import Dict, Set
from threading import Lock
from eth_token.erc20_token.erc20_token import ERC20Token


@dataclass
class CacheEntry:
    token: 'ERC20Token'
    timestamp: float
    token_status: str


class LiveTokensCache:
    def __init__(self, max_size: int = 2000, logger=None, add_pnl_to_db: bool = False):
        """
        Initialize LiveTokensCache with optional PnL database writing.
        
        Args:
            max_size: Maximum cache size
            logger: Logger instance
            add_pnl_to_db: Flag to enable/disable PnL database writing
        """
        self.max_size = max_size
        self.cache: OrderedDict[str, CacheEntry] = OrderedDict()
        self._lock = Lock()
        self.logger = logger
        self.add_pnl_to_db = add_pnl_to_db
        self.pool_to_token: Dict[str, str] = {}
        self._token_pool_addresses: Dict[str, Set[str]] = {}
        
        # Initialize token PnL writer if PnL writing is enabled
        if add_pnl_to_db:
            from eth_data.database.writers.token_pnl_writer import TokenPnLWriter
            self.pnl_writer = TokenPnLWriter(logger=logger)
            self.log("Token PnL writer initialized for database operations")
        else:
            self.pnl_writer = None

    def log(self, message: str, level: str = "info", **kwargs):
        """Log a message with optional level and kwargs passthrough.

        Accepts arbitrary keyword args (e.g., exc_info=True) to mirror the
        standard logging API. If exc_info is provided and level is left as
        default, the log level is escalated to 'error'.
        """
        if not self.logger:
            return
        # Promote to error if exception info is provided
        if kwargs.get("exc_info") and level == "info":
            level = "error"
        log_fn = getattr(self.logger, level, self.logger.info)
        log_fn(message, **kwargs)
 
    def clear_cache(self):
        """Clear the cache"""
        with self._lock:
            self.cache.clear()
            self.pool_to_token.clear()
            self._token_pool_addresses.clear()
            self.log("Cache cleared successfully")
    
    def get_cached_contract_addresses(self):
        """Get a list of cached addresses"""
        with self._lock:
            return tuple(self.cache.keys())
    
    def get_cached_tokens(self) -> Dict[str, 'ERC20Token']:
        """Get a dictionary of cached tokens"""
        with self._lock:
            return {
                addr: entry.token 
                for addr, entry in self.cache.items() 
                if entry.token is not None  # Ensure token exists
            }
        
    def get_active_tokens(self) -> Dict[str, 'ERC20Token']:
        """Get dictionary of active tokens"""
        with self._lock:
            active_tokens = {}
            for addr, entry in self.cache.items():
                if entry.token is not None:
                    if entry.token_status == 'Active':
                        active_tokens[addr] = entry.token
            return active_tokens

    def update_pool_mapping(self, token: ERC20Token) -> None:
        # Pool manager might not be initialized yet
        pool_manager = getattr(getattr(token, "token_data", None), "pool_manager", None)
        if not pool_manager:
            return

        with self._lock:
            current_addresses = set(pool_manager.get_all_pool_addresses())
            previous_addresses = self._token_pool_addresses.get(token.contract_address, set())

            # Remove pools no longer associated with this token
            for pool_addr in previous_addresses - current_addresses:
                self.pool_to_token.pop(pool_addr, None)

            # Index / refresh current pools
            for pool_addr in current_addresses:
                self.pool_to_token[pool_addr] = token.contract_address

            self._token_pool_addresses[token.contract_address] = current_addresses

    def __getitem__(self, contract_address: str):
        """Get token from cache by token address OR pool address"""
        try:
            # First try direct lookup (it's a token address)
            with self._lock:
                if contract_address in self.cache:
                    return self.cache[contract_address].token
                else:
                    # check if the contract_address is a pool address of a cached token
                    if contract_address in self.pool_to_token:
                        token_address = self.pool_to_token[contract_address]
                        return self.cache[token_address].token
            return None
        except Exception as e:
            self.log(f"{__name__}: Error getting item {contract_address}: {str(e)}")
            raise e

    def __setitem__(self, key: str, value: 'ERC20Token'):
        """Set attribute in token_data"""
        assert isinstance(value, ERC20Token), "Value must be a LiveERC20Token instance"
        with self._lock:
            try:    
                if len(self.cache) >= self.max_size:
                    # Remove oldest entry but write its PnL data first if enabled
                    oldest_key, _ = next(iter(self.cache.items()))
                    if self.add_pnl_to_db and self.pnl_writer:
                        self._write_token_pnl(oldest_key)
                    self.cache.popitem(last=False)
                self.cache[key] = CacheEntry(
                    token=value,
                    timestamp=time.time(),
                    token_status="Creation"
                )
                self.cache.move_to_end(key)
                
            except Exception as e:
                self.log(f"{__name__}: Error adding token {key}: {str(e)}")

    def __contains__(self, item: str):
        """Check if item is in cache"""
        return item in self.cache
    
    def __len__(self):
        """Return the number of items in cache"""
        return len(self.cache)
    
    def __iter__(self):
        """Iterate over cache items"""
        return iter(self.cache.values())
    
    def __repr__(self):
        """Return a string representation of the cache"""
        return f"{self.__class__.__name__}(max_size={self.max_size}, current_size={len(self.cache)})"
    
    def __str__(self):
        """Return a string representation of the cache"""
        return f"{self.__class__.__name__}(max_size={self.max_size}, current_size={len(self.cache)})"

    def __delitem__(self, key: str):
        """Delete an item from the cache, optionally writing PnL data first"""
        with self._lock:
            # Write PnL data if enabled
            if self.add_pnl_to_db and self.pnl_writer:
                self._write_token_pnl(key)
            
            # Clean up pool mappings before deleting token
            for pool_addr in self._token_pool_addresses.pop(key, set()):
                self.pool_to_token.pop(pool_addr, None)
            
            # Delete from cache
            del self.cache[key]

    def __getattr__(self, item: str):
        """Get attribute from token_data"""
        return getattr(self.cache, item)
    
    def _write_token_pnl(self, token_address: str) -> bool:
        """
        Write PnL data for a token to the database.
        Only writes if at least one pool has trading enabled (trading_enabled_tx is set).
        
        Args:
            token_address: Address of the token to write PnL data for
            
        Returns:
            bool: True if successful, False otherwise
        """
        if not self.add_pnl_to_db or not self.pnl_writer:
            return False
            
        try:
            token_entry = self.cache.get(token_address)
            if not token_entry or not token_entry.token:
                return False
                
            # Check if any pool has trading enabled (trading_enabled_tx is not None)
            if hasattr(token_entry.token, 'token_data') and token_entry.token.token_data:
                pool_manager = token_entry.token.token_data.pool_manager
                if pool_manager:
                    # Get all pools and check if any have trading enabled
                    pools = pool_manager.get_all_pools()
                    # Trading is enabled if trading_enabled_tx is set
                    has_trading = any(pool.trading_enabled_tx is not None for pool in pools.values())
                    if has_trading:
                        return self.pnl_writer.write_token_pnl_to_db(token_entry.token)
            return False
        except Exception as e:
            self.log(f"Error writing PnL data for token {token_address}: {str(e)}", exc_info=True)
            return False
    
    def write_all_token_pnl(self) -> int:
        """
        Write PnL data for all tokens in the cache to the database.
        
        Returns:
            int: Number of tokens successfully written
        """
        if not self.add_pnl_to_db or not self.pnl_writer:
            self.log("PnL writing is disabled or no database connection is available")
            return 0
            
        success_count = 0
        with self._lock:
            for addr in list(self.cache.keys()):
                if self._write_token_pnl(addr):
                    success_count += 1
                    
        self.log(f"Successfully wrote PnL data for {success_count} tokens")
        return success_count
