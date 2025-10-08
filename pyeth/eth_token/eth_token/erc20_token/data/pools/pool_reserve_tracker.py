"""
Pool Reserve Tracker

This module provides reserve tracking and scam detection for a single pool.
Each pool instance maintains its own reserve tracker for monitoring liquidity
and detecting potential scam patterns.
"""

from typing import List, Optional, Tuple, Any, Dict
from dataclasses import dataclass
from web3 import Web3
from eth_data.chain_utils.common_addresses import DENOM_ADDRESSES
from eth_token.erc20_token.config.scam_thresholds import get_threshold_for_token


@dataclass
class ReserveSnapshot:
    """Snapshot of pool reserves at a specific block and transaction."""
    block_number: int
    tx_hash: str
    denom_reserve: float
    token_reserve: float
    price: float
    timestamp: int


class PoolReserveTracker:
    """
    Tracks reserve changes and detects scams for a single pool.
    
    This class maintains a history of reserve levels and provides
    scam detection based on liquidity thresholds.
    """
    
    def __init__(self, pool_address: str, denom_address: str, history_limit: int = 100):
        self.pool_address = pool_address
        self.denom_address = denom_address
        self.history_limit = history_limit
        self.reserve_history: List[ReserveSnapshot] = [] # Reserve history
        self.latest_snapshot: Optional[ReserveSnapshot] = None # Latest snapshot
                
        # Scam detection state
        self.is_scam: bool = False
        self.scam_label: Optional[str] = None
        self.scam_block: Optional[int] = None
        self.scam_tx_hash: Optional[str] = None
        
    def update_reserves(self, denom_reserve: float, token_reserve: float, 
                       price: float, block_number: int, timestamp: int, tx_hash: str):
        """
        Update reserves and check for scam patterns.
        
        Args:
            denom_reserve: Reserve of the denomination token
            token_reserve: Reserve of the tracked token
            price: Current price in denom per token
            block_number: Block number of the update
            timestamp: Block timestamp
            tx_hash: Transaction hash causing the update
        """
        snapshot = ReserveSnapshot(
            block_number=block_number,
            tx_hash=tx_hash,
            denom_reserve=denom_reserve,
            token_reserve=token_reserve,
            price=price,
            timestamp=timestamp
        )
        
        # Store snapshot
        self.reserve_history.append(snapshot)
        if len(self.reserve_history) > self.history_limit:
            del self.reserve_history[: len(self.reserve_history) - self.history_limit]
        self.latest_snapshot = snapshot
        
        # Check for scam patterns
        self._check_for_scam(snapshot)
        
    def _check_for_scam(self, snapshot: ReserveSnapshot):
        """
        Check if the pool should be marked as a scam based on liquidity.
        """
        # Get denomination token name
        denom_name = DENOM_ADDRESSES.get(self.denom_address)
        if not denom_name:
            return
        
        # Get threshold config
        threshold_config = get_threshold_for_token(denom_name)
        if not threshold_config:
            return
        
        threshold = threshold_config['threshold']
        unit = threshold_config['unit']
        
        # Check if below threshold
        if snapshot.denom_reserve < threshold:
            # Mark as scam
            self.is_scam = True
            self.scam_label = f"Denom_removal ({unit}<{threshold})"
            self.scam_block = snapshot.block_number
            self.scam_tx_hash = snapshot.tx_hash
        else:
            # Pool recovered - clear scam status
            if self.is_scam:
                self.is_scam = False
                self.scam_label = None
                self.scam_block = None
                self.scam_tx_hash = None
    
    def get_latest_price(self) -> Optional[float]:
        """Get the latest recorded price."""
        return self.latest_snapshot.price if self.latest_snapshot else None
    
    def get_latest_reserves(self) -> Tuple[float, float]:
        """Get the latest reserves (denom, token)."""
        if not self.latest_snapshot:
            return 0.0, 0.0
        return self.latest_snapshot.denom_reserve, self.latest_snapshot.token_reserve
    
    def get_price_history(self) -> List[Tuple[str, int, float]]:
        return [(s.tx_hash, s.block_number, s.price) for s in self.reserve_history]
    
    def get_initial_price(self) -> Optional[float]:
        """Get the initial price when the pool was created."""
        return self.reserve_history[0].price if self.reserve_history else None
    
    def get_price_ratio_to_initial(self) -> Optional[float]:
        """Calculate the ratio of current price to initial price."""
        if not self.reserve_history or len(self.reserve_history) < 1:
            return None
            
        initial_price = self.reserve_history[0].price
        latest_price = self.latest_snapshot.price        
        if initial_price > 0:
            return latest_price / initial_price
        return None
    
    def get_scam_label(self) -> Optional[str]:
        return self.scam_label
    
    def get_reserve_history_df(self):
        """Get reserve history as a list of dicts for DataFrame creation."""
        return [
            {
                'block_number': s.block_number,
                'tx_hash': s.tx_hash,
                'denom_reserve': s.denom_reserve,
                'token_reserve': s.token_reserve,
                'price': s.price,
                'timestamp': s.timestamp
            }
            for s in self.reserve_history
        ]
