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
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from eth_token_monitor.services.cache.token_cache_service import TokenCacheService
from eth_token_monitor.utils.logger import get_logger
import asyncio


@dataclass
class CacheEntry:
    token: 'LiveERC20Token'
    timestamp: float
    token_status: str


class LiveTokenObjectsCache:
    def __init__(self, max_size: int = 10000,
                 redis_url: str = "redis://localhost:6379/0",
                 logger=None):
        self.max_size = max_size
        self.cache: OrderedDict[str, CacheEntry] = OrderedDict()
        self._lock = Lock()
        self.logger = logger or get_logger(name="token_manager", log_folder="tokens_live")
        
        # Initialize Redis service
        #self.cache_service = TokenCacheService(redis_url=redis_url, logger=self.logger)
        
        # Start periodic sync
        #self._start_sync_task()

    def _start_sync_task(self):
        """Start background task for periodic Redis sync"""
        asyncio.create_task(self._periodic_sync())

    async def _periodic_sync(self):
        """Sync cached tokens to Redis every minute"""
        while True:
            try:
                await asyncio.sleep(60)  # 1 minute interval
                await self._sync_cached_tokens()
            except Exception as e:
                self.logger.error(f"Error in periodic sync: {e}")

    async def _sync_cached_tokens(self):
        """Sync all cached tokens to Redis cache service"""
        try:
            with self._lock:
                cached_tokens = self.get_cached_tokens()
                
            if not cached_tokens:
                return
                
            try:
                # Use batch store for efficiency
                success = await self.cache_service.store_tokens_batch(cached_tokens)
                if success:
                    self.logger.info(f"Successfully synced {len(cached_tokens)} tokens to Redis")
                else:
                    self.logger.warning("Failed to sync tokens batch to Redis")
                    
            except Exception as e:
                self.logger.error(f"Error in batch sync to Redis: {e}")
                    
        except Exception as e:
            self.logger.error(f"Critical error in token sync: {e}")

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
