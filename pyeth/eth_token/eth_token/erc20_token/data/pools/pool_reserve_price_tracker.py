"""
Pool Reserve Tracker

This module provides the core reserve-based scam detection functionality
by tracking liquidity across all of a token's pools. Its primary
responsibility is to determine if a token should be labeled as a scam
due to insufficient liquidity in its pools.
"""

from typing import Dict, List, Optional, Tuple, Any
from dataclasses import dataclass, field
from collections import defaultdict
from web3 import Web3
from eth_token.utils.common_addresses import *
from eth_token.erc20_token.config.scam_thresholds import get_threshold_for_token


@dataclass
class PoolReserveSnapshot:
    """Snapshot of pool reserves at a specific block and transaction."""
    block_number: int
    tx_hash: str
    denom_reserve: float
    token_reserve: float
    price: float
    timestamp: int


class PoolReservePriceTracker:
    """
    Tracks reserve and price changes across all pools for a token.
    
    This class maintains a history of reserve and price levels for each of a token's 
    liquidity pools. It provides two core functions:
    1. Scam Labeling: Flags the token as a scam if any pool has critically low liquidity.
    2. Best Price Oracle: Determines the most reliable price for the token by identifying
       the pool with the highest denomination currency reserves.
    """
    
    def __init__(self, token_address: str, logger):
        self.token_address = Web3.to_checksum_address(token_address)
        self.logger = logger
        
        # Reserve snapshots by pool
        self.reserve_history: Dict[str, List[PoolReserveSnapshot]] = defaultdict(list)
        
        # Latest reserves by pool
        self.latest_reserves: Dict[str, PoolReserveSnapshot] = {}
        
        # Pool scam detection state (pool -> scam info)
        self.pool_scam_status: Dict[str, Dict[str, Any]] = {}
        
    def update_reserves(self, pool_address: str, denom_address: str, denom_reserve: float,
                       token_reserve: float, price: float,
                       block_number: int, timestamp: int, tx_hash: str):
        """
        Update reserves for a pool and check for low-liquidity scam patterns.

        Args:
            pool_address: Address of the pool.
            denom_address: The address of the denomination currency (e.g., WETH, USDC).
            denom_reserve: Reserve of the denomination token.
            token_reserve: Reserve of the token being tracked.
            price: Current price in denom per token.
            block_number: Block number of the update.
            timestamp: Block timestamp.
            tx_hash: The hash of the transaction causing the update.
        """
        snapshot = PoolReserveSnapshot(
            block_number=block_number,
            tx_hash=tx_hash,
            denom_reserve=denom_reserve,
            token_reserve=token_reserve,
            price=price,
            timestamp=timestamp
        )
        
        # Store snapshot for historical context
        self.reserve_history[pool_address].append(snapshot)
        self.latest_reserves[pool_address] = snapshot
        
        # Check for scam patterns
        self._check_for_low_liquidity_scam(pool_address, denom_address, snapshot, tx_hash)
        
    def _check_for_low_liquidity_scam(self, pool_address: str, denom_address: str, snapshot: PoolReserveSnapshot, tx_hash: str):
        """
        Checks if the pool's liquidity is below the acceptable threshold for its
        denomination currency, flagging the specific pool as a scam if it is.
        
        This tracks scam status per pool, not per token. Each pool can independently
        be marked as a scam based on its own liquidity levels.
        """
        # Get the denomination token name
        denom_name = DENOM_ADDRESSES.get(denom_address, None)
        if not denom_name:
            # Can't determine threshold for unknown denomination
            return
        
        # Get threshold config
        threshold_config = get_threshold_for_token(denom_name)
        if not threshold_config:
            # No threshold defined for this token
            return
        
        threshold = threshold_config['threshold']
        unit = threshold_config['unit']
        
        is_low_liquidity = snapshot.denom_reserve < threshold
        
        if is_low_liquidity:
            # Mark this specific pool as scam
            self.pool_scam_status[pool_address] = {
                'is_scam': True,
                'reason': (f"Low liquidity: Pool has {snapshot.denom_reserve:.4f} {unit}, "
                          f"< {threshold} {unit} threshold."),
                'block_number': snapshot.block_number,
                'tx_hash': tx_hash,
                'detected_at': snapshot.timestamp
            }
        else:
            # Pool is healthy, remove from scam status if it was there
            if pool_address in self.pool_scam_status:
                del self.pool_scam_status[pool_address]

    def get_latest_price(self, pool_address: str) -> Optional[float]:
        """Gets the latest recorded price for a given pool."""
        if pool_address in self.latest_reserves:
            return self.latest_reserves[pool_address].price
        return None

    def get_price_history(self, pool_address: str) -> List[Tuple[str, int, float]]:
        """
        Gets the full, transaction-level price history for a specific pool.
        
        Returns:
            A list of (tx_hash, block_number, price) tuples.
        """
        history = self.reserve_history.get(pool_address, [])
        return [(s.tx_hash, s.block_number, s.price) for s in history]

    def get_best_price(self) -> Optional[float]:
        """
        Gets the price from the pool with the highest denomination reserves.
        
        Returns the most reliable price by selecting from the pool with the 
        highest liquidity (denomination reserves).
        """
        if not self.latest_reserves:
            return None
            
        # Find pool with highest denomination reserves
        best_pool = max(self.latest_reserves.items(), 
                       key=lambda x: x[1].denom_reserve)
        
        return best_pool[1].price if best_pool else None
    
    def get_price_ratio_to_initial(self, pool_address: str) -> Optional[float]:
        """
        Calculates the ratio of the latest price to the initial price for a pool.
        
        This is a key indicator for initial price momentum. A ratio > 1 indicates
        the price has increased from its starting point, while a ratio < 1
        indicates a decrease.
        """
        history = self.reserve_history.get(pool_address, [])
        if len(history) < 1:
            return None
            
        initial_price = history[0].price
        latest_price = history[-1].price
        
        if initial_price > 0:
            return latest_price / initial_price
        
        # Avoid division by zero if the initial price was 0
        return None
    
    def is_pool_scam(self, pool_address: str) -> bool:
        """Check if a specific pool is marked as scam."""
        return pool_address in self.pool_scam_status and self.pool_scam_status[pool_address].get('is_scam', False)
    
    def get_pool_scam_info(self, pool_address: str) -> Optional[Dict[str, Any]]:
        """Get scam information for a specific pool."""
        return self.pool_scam_status.get(pool_address)
    
    def get_healthy_pools(self) -> List[str]:
        """Get list of pools that are not marked as scam."""
        return [pool for pool in self.latest_reserves.keys() if not self.is_pool_scam(pool)]
    
    def get_scammed_pools(self) -> List[str]:
        """Get list of pools that are marked as scam."""
        return list(self.pool_scam_status.keys())
    
    @property
    def all_pools_are_scam(self) -> bool:
        """Check if all tracked pools are marked as scam."""
        if not self.latest_reserves:
            return False
        return all(self.is_pool_scam(pool) for pool in self.latest_reserves.keys())

    def _is_dominant_pool_scam(self, dominance_threshold: float = 0.8) -> bool:
        """
        Check if a dominant pool (one with significant liquidity share) is marked as scam.
        
        Args:
            dominance_threshold: The fraction of total liquidity a pool must have to be considered dominant (default: 0.8)
        
        Returns:
            True if a dominant pool is marked as scam, False otherwise
        """
        if not self.latest_reserves:
            return False
            
        # Calculate total liquidity across all pools (in denomination currency)
        total_liquidity = sum(snapshot.denom_reserve for snapshot in self.latest_reserves.values())
        
        if total_liquidity == 0:
            return False
            
        # Check each pool's dominance
        for pool_address, snapshot in self.latest_reserves.items():
            pool_liquidity_share = snapshot.denom_reserve / total_liquidity
            
            # If this pool is dominant and is a scam, mark token as scam
            if pool_liquidity_share >= dominance_threshold and self.is_pool_scam(pool_address):
                return True
                
        return False

    def get_pool_dominance_info(self) -> Dict[str, Dict[str, Any]]:
        """
        Get dominance information for all pools.
        
        Returns:
            Dictionary mapping pool addresses to their dominance info including:
            - liquidity_share: Percentage of total liquidity
            - denom_reserve: Actual reserve amount
            - is_dominant: Whether the pool is considered dominant
            - is_scam: Whether the pool is marked as scam
        """
        if not self.latest_reserves:
            return {}
            
        total_liquidity = sum(snapshot.denom_reserve for snapshot in self.latest_reserves.values())
        if total_liquidity == 0:
            return {}
            
        dominance_info = {}
        for pool_address, snapshot in self.latest_reserves.items():
            liquidity_share = snapshot.denom_reserve / total_liquidity
            dominance_info[pool_address] = {
                'liquidity_share': liquidity_share,
                'liquidity_share_percent': liquidity_share * 100,
                'denom_reserve': snapshot.denom_reserve,
                'is_dominant': liquidity_share >= 0.8,
                'is_scam': self.is_pool_scam(pool_address)
            }
            
        return dominance_info

    @property
    def is_scam(self) -> bool:
        """
        Determines if the token should be labeled as a scam based on its pools.
        
        A token is considered a scam if:
        1. All pools are marked as scams, OR
        2. The dominant pool (>80% of total liquidity) is marked as a scam
        """
        # If all pools are scams, definitely a scam
        if self.all_pools_are_scam:
            return True
            
        # Check for dominant pool scam
        return self._is_dominant_pool_scam()
    
    @property
    def scam_reason(self) -> Optional[str]:
        """
        Provides a concise reason label for the scam if the token is marked as a scam.
        Only considers pools with at least 20% of total pool liquidity.
        
        Returns:
            A concise string label like "Denom_removal (ETH<0.05)" or "hidden_mint", or None if not a scam.
        """
        if not self.is_scam:
            return None
        
        # Get dominance info to find pools with significant liquidity share
        dominance_info = self.get_pool_dominance_info()
        
        # Find the dominant scam pool (20%+ liquidity threshold)
        dominant_scam_reason = None
        highest_liquidity_pct = 0
        
        for pool_address, dom_info in dominance_info.items():
            liquidity_pct = dom_info.get('liquidity_share_percent', 0)
            
            # Only consider pools with at least 20% of total pool liquidity
            if liquidity_pct >= 20 and pool_address in self.pool_scam_status:
                scam_info = self.pool_scam_status[pool_address]
                
                # Use the pool with highest liquidity percentage
                if liquidity_pct > highest_liquidity_pct:
                    highest_liquidity_pct = liquidity_pct
                    reason = scam_info.get('reason', '')
                    
                    # Parse and simplify the reason
                    if 'Low liquidity' in reason:
                        # Extract the value and unit from reason string
                        import re
                        match = re.search(r'Pool has ([\d.]+) (\w+),', reason)
                        if match:
                            value = float(match.group(1))
                            unit = match.group(2)
                            dominant_scam_reason = f"Denom_removal ({unit}<{value:.2f})"
                        else:
                            dominant_scam_reason = "Denom_removal"
                    elif 'hidden mint' in reason.lower():
                        dominant_scam_reason = "hidden_mint"
                    else:
                        # For other reasons, extract key phrase or use generic label
                        dominant_scam_reason = reason.split(':')[0].strip() if ':' in reason else "scam_detected"
        
        # If no pool has 20%+ liquidity, return generic reason
        return dominant_scam_reason or "scam_detected"
    
    @property
    def scam_block(self) -> Optional[int]:
        """
        Returns the earliest block number where a scam was detected.
        
        For dominant pool scams, returns the block when the dominant pool was marked as scam.
        For all-pool scams, returns the earliest detection block.
        
        Returns:
            The block number where scam was first detected, or None if not a scam.
        """
        if not self.is_scam or not self.pool_scam_status:
            return None
            
        # If there's a dominant pool scam, return its detection block
        dominance_info = self.get_pool_dominance_info()
        for pool_address, dom_info in dominance_info.items():
            if dom_info['is_dominant'] and dom_info['is_scam']:
                return self.pool_scam_status[pool_address]['block_number']
        
        # Otherwise return the earliest scam detection
        return min(info['block_number'] for info in self.pool_scam_status.values())
    