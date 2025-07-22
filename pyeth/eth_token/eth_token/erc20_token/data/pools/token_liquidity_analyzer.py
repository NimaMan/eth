"""
Token Liquidity Analyzer

This module aggregates pool-level data to make token-level scam determinations.
It analyzes liquidity distribution across all pools to detect if a token should
be marked as a scam based on multi-pool patterns.
"""

from typing import Dict, List, Optional, TYPE_CHECKING
from web3 import Web3

if TYPE_CHECKING:
    from .pool_manager import PoolManager


class TokenLiquidityAnalyzer:
    """
    Analyzes liquidity across all pools to determine token-level scam status.
    
    A token is considered a scam if:
    1. All pools are marked as scams, OR
    2. The dominant pool (>80% of total liquidity) is marked as a scam
    """
    
    def __init__(self, token_address: str, logger=None):
        self.token_address = Web3.to_checksum_address(token_address)
        self.logger = logger
        
    def analyze_token_scam_status(self, pool_manager: 'PoolManager') -> Dict:
        """
        Analyze all pools to determine if the token is a scam.
        
        Returns:
            Dict with is_scam, scam_label, scam_block, scam_tx_hash, and analysis details
        """
        pools = pool_manager.get_all_pools()
        
        if not pools:
            return {
                'is_scam': False,
                'scam_label': None,
                'scam_block': None,
                'scam_tx_hash': None,
                'reason': 'No pools found'
            }
        
        # Collect pool data
        pool_data = []
        total_denom_liquidity = 0.0
        
        for pool in pools:
            denom_reserve = pool.get_denom_reserve()
            total_denom_liquidity += denom_reserve
            
            pool_data.append({
                'pool': pool,
                'address': pool.pool_address,
                'protocol': pool.get_protocol(),
                'denom_reserve': denom_reserve,
                'is_scam': pool.is_scam,
                'scam_label': pool.scam_label,
                'scam_block': pool.scam_block,
                'scam_tx_hash': pool.scam_tx_hash
            })
        
        # Check if all pools are scams
        all_pools_scam = all(pd['is_scam'] for pd in pool_data)
        
        # Calculate liquidity shares and find dominant pool
        dominant_pool_scam = False
        dominant_pool_info = None
        
        if total_denom_liquidity > 0:
            for pd in pool_data:
                liquidity_share = pd['denom_reserve'] / total_denom_liquidity
                pd['liquidity_share'] = liquidity_share
                pd['liquidity_share_percent'] = liquidity_share * 100
                
                # Check for dominant scam pool
                if liquidity_share >= 0.8 and pd['is_scam']:
                    dominant_pool_scam = True
                    dominant_pool_info = pd
        
        # Determine token-level scam status
        is_scam = all_pools_scam or dominant_pool_scam
        
        if not is_scam:
            return {
                'is_scam': False,
                'scam_label': None,
                'scam_block': None,
                'scam_tx_hash': None,
                'reason': 'Token has healthy liquidity'
            }
        
        # Generate scam details
        if all_pools_scam:
            # Find the earliest scam detection
            earliest_scam = min(pool_data, key=lambda x: x['scam_block'] or float('inf'))
            return {
                'is_scam': True,
                'scam_label': earliest_scam['scam_label'],
                'scam_block': earliest_scam['scam_block'],
                'scam_tx_hash': earliest_scam['scam_tx_hash'],
                'reason': 'All pools are marked as scams',
                'pool_analysis': pool_data
            }
        
        elif dominant_pool_info:
            return {
                'is_scam': True,
                'scam_label': dominant_pool_info['scam_label'],
                'scam_block': dominant_pool_info['scam_block'],
                'scam_tx_hash': dominant_pool_info['scam_tx_hash'],
                'reason': f"Dominant pool ({dominant_pool_info['liquidity_share_percent']:.1f}% liquidity) is a scam",
                'dominant_pool': dominant_pool_info['address'],
                'pool_analysis': pool_data
            }
        
        return {
            'is_scam': False,
            'scam_label': None,
            'scam_block': None,
            'scam_tx_hash': None,
            'reason': 'Unknown'
        }
    
    def get_best_price(self, pool_manager: 'PoolManager') -> Optional[float]:
        """
        Get the most reliable price by selecting from the pool with highest liquidity.
        
        Returns:
            The price from the pool with highest denomination reserves
        """
        pools = pool_manager.get_all_pools()
        
        if not pools:
            return None
        
        # Find pool with highest denomination reserves
        best_pool = max(pools, key=lambda p: p.get_denom_reserve())
        
        if best_pool and best_pool.get_denom_reserve() > 0:
            return best_pool.get_price()
        
        return None
    
    def get_healthy_pools(self, pool_manager: 'PoolManager') -> List:
        """Get list of pools that are not marked as scams."""
        return [pool for pool in pool_manager.get_all_pools() if not pool.is_scam]
    
    def get_scam_pools(self, pool_manager: 'PoolManager') -> List:
        """Get list of pools that are marked as scams."""
        return [pool for pool in pool_manager.get_all_pools() if pool.is_scam]
    
    def get_liquidity_distribution(self, pool_manager: 'PoolManager') -> Dict:
        """
        Get liquidity distribution across all pools.
        
        Returns:
            Dict mapping pool addresses to their liquidity info
        """
        pools = pool_manager.get_all_pools()
        total_liquidity = sum(p.get_denom_reserve() for p in pools)
        
        distribution = {}
        for pool in pools:
            denom_reserve = pool.get_denom_reserve()
            share = denom_reserve / total_liquidity if total_liquidity > 0 else 0
            
            distribution[pool.pool_address] = {
                'protocol': pool.get_protocol(),
                'denom_reserve': denom_reserve,
                'liquidity_share': share,
                'liquidity_share_percent': share * 100,
                'is_scam': pool.is_scam
            }
        
        return distribution