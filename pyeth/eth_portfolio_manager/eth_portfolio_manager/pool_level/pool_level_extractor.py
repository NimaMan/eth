"""
PoolLevelExtractor: Extract ETH reserve levels from token updates.

Objective:
---------
1. Extract ETH reserve levels from token updates as blocks are processed
2. Maintain a clean in-memory cache of latest pool levels
3. Provide efficient access to current pool state
4. Support callbacks when pool levels are updated

This component bridges between the LiveBacktestEngine, which processes blocks
and updates token data, and the PoolLevelPublisher, which shares this data
with the Rust mempool processor for scam detection.
"""

import asyncio
from typing import Dict, Set, Optional, Callable, Any, Tuple
import time
from eth_portfolio_manager.utils.logger import get_logger


class PoolLevelExtractor:
    """
    Extracts and maintains pool ETH reserve levels from token updates.
    
    This class is responsible for:
    1. Extracting ETH reserves from token data structures
    2. Maintaining an in-memory cache of current pool levels
    3. Providing access to the latest pool state
    4. Notifying subscribers when pool levels change
    """
    
    def __init__(self, logger=None):
        """Initialize the pool level extractor."""
        self.logger = logger or get_logger("pool_level_extractor")
        self._pool_eth_levels: Dict[str, float] = {}
        self._pool_token_map: Dict[str, str] = {}
        self._token_pools: Dict[str, Set[str]] = {}
        self._last_update_block: int = 0
        self._last_update_time: float = 0
        self._update_callbacks: Dict[str, Callable] = {}
        self._lock = asyncio.Lock()
    
    async def update_pool_levels(self, updated_tokens: Dict[str, Any], block_number: int) -> Dict[str, Dict[str, Any]]:
        """
        Update pool levels based on token updates from a processed block.
        
        Args:
            updated_tokens: Dict mapping token addresses to token objects with pool data
            block_number: The block number these updates came from
            
        Returns:
            Dict of updated pool data in the format:
            {
                pool_address: {
                    'eth_reserve': float,
                    'token_address': str,
                    'block_number': int,
                    'update_time': float
                }
            }
        """
        if not updated_tokens:
            return {}
            
        updated_pools = {}
        current_time = time.time()
        
        async with self._lock:
            for token_address, token in updated_tokens.items():
                pool_addresses = getattr(token.token_data, 'pool_addresses', None)
                
                if not pool_addresses:
                    continue
                
                # Update token to pools mapping
                self._token_pools.setdefault(token_address, set()).update(pool_addresses)
                
                for pool_address in pool_addresses:
                    try:
                        eth_level = token.token_data.get_pool_reserve(pool_address)
                        
                        if eth_level is not None:
                            eth_level_float = float(eth_level)
                            
                            # Update the caches
                            self._pool_eth_levels[pool_address] = eth_level_float
                            self._pool_token_map[pool_address] = token_address
                            
                            # Track which pools were updated
                            updated_pools[pool_address] = {
                                'eth_reserve': eth_level_float,
                                'token_address': token_address,
                                'block_number': block_number,
                                'update_time': current_time
                            }
                            
                    except Exception as e:
                        self.logger.error(
                            f"Error extracting pool level for token {token_address}, "
                            f"pool {pool_address}: {e}"
                        )
            
            self._last_update_block = block_number
            self._last_update_time = current_time
        
        # Notify callbacks about the updates
        await self._notify_updates(updated_pools)
        
        if updated_pools:
            self.logger.info(
                f"Updated {len(updated_pools)} pool levels at block {block_number}, "
                f"total pools tracked: {len(self._pool_eth_levels)}"
            )
            
        return updated_pools
    
    def get_pool_level(self, pool_address: str) -> Optional[float]:
        """Get the current ETH level for a specific pool."""
        return self._pool_eth_levels.get(pool_address)
    
    def get_all_pool_levels(self) -> Dict[str, float]:
        """Get all current pool ETH levels."""
        return self._pool_eth_levels.copy()
    
    def get_pool_data(self, pool_address: str) -> Optional[Dict[str, Any]]:
        """
        Get comprehensive data for a specific pool.
        
        Returns:
            Dict with pool data or None if pool is not found
        """
        eth_level = self._pool_eth_levels.get(pool_address)
        if eth_level is None:
            return None
            
        return {
            'eth_reserve': eth_level,
            'token_address': self._pool_token_map.get(pool_address),
            'block_number': self._last_update_block,
            'update_time': self._last_update_time
        }
    
    def get_token_pools(self, token_address: str) -> Set[str]:
        """Get all pools associated with a specific token."""
        return self._token_pools.get(token_address, set())
    
    def register_update_callback(self, name: str, callback: Callable[[Dict[str, Dict[str, Any]]], None]) -> None:
        """
        Register a callback to be notified when pool levels are updated.
        
        Args:
            name: A unique name for this callback
            callback: Function to call with the updated pool data
        """
        self._update_callbacks[name] = callback
    
    def unregister_update_callback(self, name: str) -> None:
        """Unregister a previously registered callback."""
        if name in self._update_callbacks:
            del self._update_callbacks[name]
    
    async def _notify_updates(self, updated_pools: Dict[str, Dict[str, Any]]) -> None:
        """Notify all registered callbacks about pool updates."""
        if not updated_pools or not self._update_callbacks:
            return
            
        for name, callback in self._update_callbacks.items():
            try:
                if asyncio.iscoroutinefunction(callback):
                    await callback(updated_pools)
                else:
                    callback(updated_pools)
            except Exception as e:
                self.logger.error(f"Error in pool update callback {name}: {e}") 