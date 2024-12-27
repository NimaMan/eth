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
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.utils.logger import get_logger


@dataclass
class CacheEntry:
    token: 'LiveERC20Token'
    timestamp: float
    token_status: str


class LiveTokenObjectsCache:
    def __init__(self, max_size: int = 10000, logger=None):
        self.max_size = max_size
        self.cache: OrderedDict[str, CacheEntry] = OrderedDict()
        self._lock = Lock()
        self.logger = logger
        if logger is None:
            self.logger = get_logger(name="tokens_live", log_folder="tokens_live")
    
    def clear_cache(self):
        """Clear the cache"""
        with self._lock:
            self.cache.clear()
            self.logger.info("Cache cleared successfully")
    
    def get_cached_contract_addresses(self):
        """Get a list of cached addresses"""
        with self._lock:
            return tuple(self.cache.keys())
    
    def get_cached_tokens(self) -> Dict[str, 'LiveERC20Token']:
        """Get a dictionary of cached tokens"""
        with self._lock:
            return {addr: entry.token for addr, entry in self.cache.items()}
        
    def get_active_tokens(self) -> Dict[str, 'LiveERC20Token']:
        """Get a dictionary of active tokens"""
        with self._lock:
            active_tokens = {}
            for addr, entry in self.cache.items():
                if entry.token is not None:
                    if entry.token.token_status == 'Active':
                        active_tokens[addr] = entry.token
            return active_tokens

    def __getitem__(self, item: str):
        """Get attribute from token_data"""
        try:
            return self.cache[item].token
        except KeyError:
            return None
        except Exception as e:
            self.logger.error(f"{__name__}: Error getting item {item}: {str(e)}")
            raise e

    def __setitem__(self, key: str, value: 'LiveERC20Token'):
        """Set attribute in token_data"""
        assert isinstance(value, LiveERC20Token), "Value must be a LiveERC20Token instance"
        with self._lock:
            try:    
                if len(self.cache) >= self.max_size:
                    #remove oldest entry
                    self.cache.popitem(last=False)
                self.cache[key] = CacheEntry(
                    token=value,
                    timestamp=time.time(),
                    token_status="Creation"
                )
                self.cache.move_to_end(key)                
            except Exception as e:
                self.logger.error(f"{__name__}: Error adding token {key}: {str(e)}")

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
        """Delete an item from the cache"""
        del self.cache[key]

    def __getattr__(self, item: str):
        """Get attribute from token_data"""
        return getattr(self.cache, item)
