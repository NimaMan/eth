"""
TokenPositionsCache: A cache to store TokenPosition objects with LRU eviction.

This cache uses a composite key of <token_address>:<pool_address> to uniquely track
positions for tokens that are present in multiple pools. It provides thread-safe lookup,
insertion, and removal and is modeled after the LiveTokenObjectsCache.
"""

import time
from collections import OrderedDict
from dataclasses import dataclass
from threading import Lock
from typing import Optional, Dict
from eth_portfolio_manager.core.token_position import TokenPosition


@dataclass
class CacheEntry:
    token_position: TokenPosition
    timestamp: float


class TokenPositionsCache:
    def __init__(self, max_size: int = 10000):
        self.max_size = max_size
        self.cache: OrderedDict[str, CacheEntry] = OrderedDict()
        self._lock = Lock()

    def _compose_key(self, token_address: str, pool_address: str) -> str:
        """
        Compose a composite key using token address and pool address.
        """
        return f"{token_address.lower()}:{pool_address.lower()}"

    def clear_cache(self) -> None:
        """
        Clear the entire cache.
        """
        with self._lock:
            self.cache.clear()

    def get_cached_keys(self) -> Dict[str, TokenPosition]:
        """
        Retrieve a dictionary mapping keys to TokenPosition objects.
        """
        with self._lock:
            return {key: entry.token_position for key, entry in self.cache.items()}

    def get(self, token_address: str, pool_address: str) -> Optional[TokenPosition]:
        """
        Retrieve a TokenPosition object based on token and pool addresses.
        """
        key = self._compose_key(token_address, pool_address)
        with self._lock:
            if key not in self.cache:
                return None
            entry = self.cache[key]
            # Move key to the end (LRU behavior)
            self.cache.move_to_end(key)
            return entry.token_position

    def add(self, token_position: TokenPosition, token_address: str, pool_address: str) -> None:
        """
        Add or update a TokenPosition in the cache.
        """
        key = self._compose_key(token_address, pool_address)
        with self._lock:
            if key in self.cache:
                # Update the existing entry with a new timestamp
                self.cache[key].timestamp = time.time()
                self.cache[key].token_position = token_position
                self.cache.move_to_end(key)
            else:
                if len(self.cache) >= self.max_size:
                    # Remove the least-recently used entry
                    self.cache.popitem(last=False)
                self.cache[key] = CacheEntry(token_position=token_position, timestamp=time.time())
                self.cache.move_to_end(key)

    def __contains__(self, key: str) -> bool:
        with self._lock:
            return key in self.cache

    def __len__(self) -> int:
        with self._lock:
            return len(self.cache)

    def items(self) -> Dict[str, CacheEntry]:
        with self._lock:
            return self.cache.items()
        
    def values(self) -> Dict[str, CacheEntry]:
        with self._lock:
            return self.cache.values()
        
    def keys(self) -> Dict[str, CacheEntry]:
        with self._lock:
            return self.cache.keys()
        
    def __getitem__(self, key: str) -> CacheEntry:
        with self._lock:
            return self.cache[key]
        
    def __setitem__(self, key: str, value: CacheEntry):
        with self._lock:
            self.cache[key] = value
            
    def __iter__(self):
        with self._lock:
            return iter(self.cache.values())

    def __repr__(self) -> str:
        return f"TokenPositionsCache(max_size={self.max_size}, current_size={len(self.cache)})" 