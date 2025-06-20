"""
TokenInfoExtractor: Extract token and pool information from token updates.

Objective:
---------
1. Extract ETH reserve levels from token updates as blocks are processed
2. Maintain a clean in-memory cache of latest pool levels
3. Provide efficient access to current pool state
4. Support callbacks when pool levels are updated

This component bridges between the LiveBacktestEngine, which processes blocks
and updates token data, and the TokenInfoPublisher, which shares this data
with the Rust mempool processor for scam detection and trading decisions.
"""

import asyncio
from typing import Dict, Set, Optional, Callable, Any, Tuple
import time
from eth_portfolio_manager.utils.logger import get_logger


class TokenInfoExtractor:
    """
    Extracts and maintains token and pool information from token updates.
    
    This class is responsible for:
    1. Extracting token and pool data from token data structures
    2. Maintaining an in-memory cache of current token/pool states
    3. Providing access to the latest token and pool information
    4. Notifying subscribers when token information changes
    """
    
    def __init__(self, logger=None):
        """Initialize the pool level extractor."""
        self.logger = logger or get_logger("token_info_extractor")
        self._pool_eth_levels: Dict[str, float] = {}
        self._pool_token_map: Dict[str, str] = {}
        self._token_pools: Dict[str, Set[str]] = {}
        self._last_update_block: int = 0
        self._last_update_time: float = 0
        self._update_callbacks: Dict[str, Callable] = {}
        self._lock = asyncio.Lock()
    
    async def update_token_info(self, updated_tokens: Dict[str, Any], block_number: int) -> Dict[str, Dict[str, Any]]:
        """
        Update token and pool information based on token updates from a processed block.
        
        Args:
            updated_tokens: Dict mapping token addresses to token objects with pool data
            block_number: The block number these updates came from
            
        Returns:
            Dict of updated pool data in the enhanced format:
            {
                pool_identifier: {
                    'eth_reserve': float,
                    'token_reserve': float,
                    'token_address': str,
                    'pool_address': str,
                    'pool_type': str,
                    'pool_id': str (for V4),
                    'pool_manager': str (for V4),
                    'fee_tier': int,
                    'denom_currency': str,
                    'token_decimals': int,
                    'liquidity': float,
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
                    self.logger.debug(f"Token {token_address} has no pools")
                    continue
                
                self.logger.debug(f"Token {token_address} has {len(pool_addresses)} pools: {pool_addresses}")
                
                # Update token to pools mapping
                self._token_pools.setdefault(token_address, set()).update(pool_addresses)
                
                # Get pool info dictionary using the new API
                pool_info_dict = token.token_data.get_pool_info_dict()
                
                for pool_address in pool_addresses:
                    try:
                        # Get basic reserves
                        eth_level = token.token_data.get_pool_reserve(pool_address)
                        
                        if eth_level is not None:
                            eth_level_float = float(eth_level)
                            self.logger.debug(f"Pool {pool_address} has ETH reserve: {eth_level_float}")
                            
                            # Update the caches
                            self._pool_eth_levels[pool_address] = eth_level_float
                            self._pool_token_map[pool_address] = token_address
                            
                            # Get enhanced pool information
                            pool_info = pool_info_dict.get(pool_address, {})
                            
                            # Extract pool-specific data
                            pool_type = pool_info.get('pool_type', 'UNKNOWN')
                            
                            # Build comprehensive pool data
                            pool_data = {
                                'eth_reserve': eth_level_float,
                                'token_reserve': float(token.token_data.get_pool_token_reserve(pool_address) or 0),
                                'token_address': token_address,
                                'pool_address': pool_address,
                                'pool_type': pool_type,
                                'fee_tier': pool_info.get('fee', 3000),  # Default 0.3%
                                'denom_currency': pool_info.get('denom_currency'),
                                'denom_address': pool_info.get('denom_address', ''),
                                'token_decimals': pool_info.get('decimals'),
                                'latest_block_number': block_number,
                                'update_time': current_time,
                                'token_symbol': getattr(token.token_data, 'symbol', ''),
                                'token_name': getattr(token.token_data, 'name', ''),
                                'trading_enabled': getattr(token.token_data, 'trading_enabled', False)
                            }
                            
                            # Add V4-specific fields if applicable
                            if pool_type == 'V4':
                                if '#' in pool_address:
                                    parts = pool_address.split('#')
                                    pool_data['pool_manager'] = parts[0]
                                    pool_data['pool_id'] = parts[1] if len(parts) > 1 else ''
                                # Also check pool_info for pool_id
                                if 'pool_id' in pool_info:
                                    pool_data['pool_id'] = pool_info['pool_id']
                                    if 'pool_manager' not in pool_data:
                                        pool_data['pool_manager'] = '0x000000000004444c5dc75cb358380d2e3de08a90'
                            
                            # Calculate liquidity in USD (simplified)
                            # Assume ETH price for now, can be enhanced later
                            eth_price_usd = 2000  # Placeholder
                            pool_data['liquidity'] = eth_level_float * eth_price_usd * 2
                            
                            # Track which pools were updated
                            updated_pools[pool_address] = pool_data
                        else:
                            self.logger.debug(f"Pool {pool_address} has no ETH reserve data")
                            continue
                            
                    except Exception as e:
                        self.logger.error(
                            f"Error extracting pool level for token {token_address}, "
                            f"pool {pool_address}: {e}"
                        )
            
            self._last_update_block = block_number
            self._last_update_time = current_time
        
        # Notify callbacks about the updates
        await self._notify_updates(updated_pools)
            
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