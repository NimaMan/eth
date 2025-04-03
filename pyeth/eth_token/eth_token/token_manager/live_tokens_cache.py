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
from typing import Optional, Dict, List
from threading import Lock
from eth_token.live_erc20_token.live_token import LiveERC20Token


@dataclass
class CacheEntry:
    token: 'LiveERC20Token'
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
        
        # Initialize token PnL writer if PnL writing is enabled
        if add_pnl_to_db:
            from sarigoz.data.db.pnl.token_pnl_writer import TokenPnLWriter
            self.pnl_writer = TokenPnLWriter(logger=logger)
            self.log("Token PnL writer initialized for database operations")
        else:
            self.pnl_writer = None

    def log(self, message: str):
        """Log a message"""
        if self.logger:
            self.logger.info(message)
 
    def clear_cache(self):
        """Clear the cache"""
        with self._lock:
            self.cache.clear()
            self.log("Cache cleared successfully")
    
    def get_cached_contract_addresses(self):
        """Get a list of cached addresses"""
        with self._lock:
            return tuple(self.cache.keys())
    
    def get_cached_tokens(self) -> Dict[str, 'LiveERC20Token']:
        """Get a dictionary of cached tokens"""
        with self._lock:
            return {
                addr: entry.token 
                for addr, entry in self.cache.items() 
                if entry.token is not None  # Ensure token exists
            }
        
    def get_active_tokens(self) -> Dict[str, 'LiveERC20Token']:
        """Get dictionary of active tokens"""
        with self._lock:
            active_tokens = {}
            for addr, entry in self.cache.items():
                if entry.token is not None:
                    if entry.token_status == 'Active':
                        active_tokens[addr] = entry.token
            return active_tokens

    def __getitem__(self, item: str):
        """Get attribute from token_data"""
        try:
            return self.cache[item].token
        except KeyError:
            return None
        except Exception as e:
            self.log(f"{__name__}: Error getting item {item}: {str(e)}")
            raise e

    def __setitem__(self, key: str, value: 'LiveERC20Token'):
        """Set attribute in token_data"""
        assert isinstance(value, LiveERC20Token), "Value must be a LiveERC20Token instance"
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
            # Delete from cache
            del self.cache[key]

    def __getattr__(self, item: str):
        """Get attribute from token_data"""
        return getattr(self.cache, item)
    
    def _write_token_pnl(self, token_address: str) -> bool:
        """
        Write PnL data for a token to the database.
        
        Args:
            token_address: Address of the token to write PnL data for
            
        Returns:
            bool: True if successful, False otherwise
        """
        if not self.add_pnl_to_db or not self.pnl_writer:
            return False
            
        try:
            token_entry = self.cache.get(token_address)
            if token_entry and token_entry.token.token_trading_age_blocks is not None:
                self.log(f"Writing PnL data for token {token_address}")
                return self.pnl_writer.write_token_pnl_to_db(token_entry.token)
            return False
        except Exception as e:
            self.log(f"Error writing PnL data for token {token_address}: {str(e)}")
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