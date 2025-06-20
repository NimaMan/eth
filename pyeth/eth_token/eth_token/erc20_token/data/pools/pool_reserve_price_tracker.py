"""
Pool Reserve Tracker

This module provides the core reserve-based scam detection functionality
by tracking liquidity across all of a token's pools. Its primary
responsibility is to determine if a token should be labeled as a scam
due to insufficient liquidity in its pools.
"""

from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass, field
from collections import defaultdict
from eth_token.utils.common_addresses import *


# Reserve thresholds for scam detection
WETH_DENOM_RESERVE_THRESHOLD = 0.1  # 0.1 ETH
USD_DENOM_RESERVE_THRESHOLD = 1000  # $1000 for USDC/USDT/DAI

# Known stablecoin addresses
STABLECOIN_ADDRESSES = set(STABLECOINS_NAME_BY_ADDRESS.keys())

# WETH address
WETH_ADDRESS = '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2'


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
        self.token_address = token_address.lower()
        self.logger = logger
        
        # Reserve snapshots by pool
        self.reserve_history: Dict[str, List[PoolReserveSnapshot]] = defaultdict(list)
        
        # Latest reserves by pool
        self.latest_reserves: Dict[str, PoolReserveSnapshot] = {}
        
        # Scam detection state
        self.is_scam = False
        self.scam_reason: Optional[str] = None
        self.scam_block: Optional[int] = None
        self.scam_tx_hash: Optional[str] = None
        self.scam_pool: Optional[str] = None
        
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
        denomination currency, flagging the token as a scam if it is.
        
        This is the core scam-labeling logic. It prevents false positives by
        applying the correct threshold (WETH vs. USD) based on the pool's
        actual denomination currency.
        """
        # Skip if already flagged
        if self.is_scam:
            return
            
        is_low_liquidity = False
        threshold = 0.0
        denom_symbol = "unknown"

        # Check against WETH threshold
        if denom_address.lower() == WETH_ADDRESS.lower():
            threshold = WETH_DENOM_RESERVE_THRESHOLD
            denom_symbol = "WETH"
            if snapshot.denom_reserve < threshold:
                is_low_liquidity = True
        
        # Check against stablecoin threshold
        elif denom_address.lower() in STABLECOIN_ADDRESSES:
            threshold = USD_DENOM_RESERVE_THRESHOLD
            denom_symbol = STABLECOINS_NAME_BY_ADDRESS.get(denom_address.lower(), "USD")
            if snapshot.denom_reserve < threshold:
                is_low_liquidity = True
        
        if is_low_liquidity:
            self.is_scam = True
            self.scam_reason = (f"Low liquidity: Pool has {snapshot.denom_reserve:.4f} {denom_symbol}, "
                              f"which is below the {threshold} {denom_symbol} threshold.")
            self.scam_block = snapshot.block_number
            self.scam_tx_hash = tx_hash
            self.scam_pool = pool_address
            self.logger.warning(
                f"SCAM DETECTED for token {self.token_address}: {self.scam_reason} "
                f"in pool {pool_address} at block {snapshot.block_number} (tx: {tx_hash[:10]}...)"
            )

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

    def get_best_price(self) -> Optional[float]:
        """
        Determines the most reliable price for the token by finding the pool
        with the highest denomination currency reserve.

        Returns:
            The price from the most liquid pool, or None if no valid pools exist.
        """
        if not self.latest_reserves:
            return None

        best_pool_address = None
        max_denom_reserve = -1

        for pool_address, snapshot in self.latest_reserves.items():
            # We must consider the *value* of the reserves. A simple comparison
            # isn't enough (e.g., 1000 USDC vs 1 WETH). We need to normalize to USD.
            # For this implementation, we assume WETH and stablecoins are the primary
            # denominators and can be compared directly for liquidity ranking.
            # A more advanced version could use a real-time price feed for all denoms.
            if snapshot.denom_reserve > max_denom_reserve:
                max_denom_reserve = snapshot.denom_reserve
                best_pool_address = pool_address

        if best_pool_address:
            return self.latest_reserves[best_pool_address].price

        return None
