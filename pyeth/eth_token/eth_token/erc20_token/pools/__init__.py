"""
Pool management system with pool instances that handle their own events.

Each pool type (V2, V3, etc.) knows how to process its specific events
from transactions and maintain its own state.
"""

from .base_pool import BasePool, PoolState
from .uniswap_v2_pool import UniswapV2Pool
from .uniswap_v3_pool import UniswapV3Pool
from .uniswap_v4_pool import UniswapV4Pool, PoolKey
from .pool_manager import PoolManager
from .pool_chain_data_fetcher import PoolChainDataFetcher
from .pool_reserve_tracker import PoolReserveTracker, ReserveSnapshot
from .arbitrage_detector import ArbitrageDetector, ArbitrageOpportunity


__all__ = [
    'BasePool',
    'PoolState',
    'UniswapV2Pool',
    'UniswapV3Pool',
    'UniswapV4Pool',
    'PoolKey',
    'PoolManager',
    'PoolChainDataFetcher',
    'PoolReserveTracker',
    'ReserveSnapshot',
    'ArbitrageDetector',
    'ArbitrageOpportunity',
]