"""
TokenPositionsCache: A cache to store TokenPosition objects with LRU eviction.

This cache uses a composite key of <token_address>:<pool_address> to uniquely track
positions for tokens that are present in multiple pools. It provides thread-safe lookup,
insertion, and removal and is modeled after the LiveTokenObjectsCache.
"""

from collections import OrderedDict
from threading import Lock
from typing import Optional, Dict
from eth_portfolio_manager.core.token_position import TokenPosition


class TokenPositionsCache:
    def __init__(self, max_size: int = 10000):
        self.max_size = max_size
        self.token_positions: OrderedDict[str, TokenPosition] = OrderedDict()
        self._lock = Lock()

    def _compose_key(self, token_address: str, pool_address: str) -> str:
        """
        Compose a composite key using token address and pool address.
        """
        return f"{token_address}-{pool_address}"

    def clear_cache(self) -> None:
        """
        Clear the entire cache.
        """
        with self._lock:
            self.token_positions.clear()

    def get_cached_keys(self) -> Dict[str, TokenPosition]:
        """
        Retrieve a dictionary mapping keys to TokenPosition objects.
        """
        with self._lock:
            return {key: entry for key, entry in self.token_positions.items()}

    def get(self, token_address: str, pool_address: str) -> Optional[TokenPosition]:
        """
        Retrieve a TokenPosition object based on token and pool addresses.
        """
        key = self._compose_key(token_address, pool_address)
        with self._lock:
            if key not in self.token_positions:
                return None
            entry = self.token_positions[key]
            # Move key to the end (LRU behavior)
            self.token_positions.move_to_end(key)
            return entry

    def add(self, token_position: TokenPosition, token_address: str, pool_address: str) -> None:
        """
        Add or update a TokenPosition in the cache.
        """
        key = self._compose_key(token_address, pool_address)
        with self._lock:
            if key in self.token_positions:
                # Update the existing entry with a new timestamp
                self.token_positions[key] = token_position
                self.token_positions.move_to_end(key)
            else:
                if len(self.token_positions) >= self.max_size:
                    # Remove the least-recently used entry
                    self.token_positions.popitem(last=False)
                self.token_positions[key] = token_position
                self.token_positions.move_to_end(key)

    def __contains__(self, key: str) -> bool:
        with self._lock:
            return key in self.token_positions

    def __len__(self) -> int:
        with self._lock:
            return len(self.token_positions)

    def items(self):
        with self._lock:
            return self.token_positions.items()
        
    def values(self) -> Dict[str, TokenPosition]:
        with self._lock:
            return {key: entry for key, entry in self.token_positions.items()}
        
    def keys(self) -> Dict[str, str]:
        with self._lock:
            return self.token_positions.keys()
        
    def __getitem__(self, key: str) -> TokenPosition:
        with self._lock:
            return self.token_positions[key]
        
    def __setitem__(self, key: str, value: TokenPosition):
        with self._lock:
            self.token_positions[key] = value
            
    def __iter__(self):
        with self._lock:
            return iter({key: entry for key, entry in self.token_positions.items()})

    def __repr__(self) -> str:
        return f"TokenPositionsCache(max_size={self.max_size}, current_size={len(self.token_positions)})" 