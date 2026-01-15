from dataclasses import dataclass
from enum import Enum


class PoolLifecycle(str, Enum):
    """Lifecycle stages shared across Python, Rust, and analytics."""

    DISCOVERED = "DISCOVERED"   # Pool observed on-chain (deployment), zero liquidity
    LIQUIDITY_DEPOSITED = "LIQUIDITY_DEPOSITED"  # Liquidity deposited (non-zero reserves)
    ACTIVE = "ACTIVE"           # Buy/sell viability confirmed
    SCAM = "SCAM"               # Rug detected (Python flag or reserve tracker)
    EVICTED = "EVICTED"         # Removed from active tracking


@dataclass
class PoolRuntimeState:
    """Current mutable state of a pool."""

    denom_reserve: float = 0.0
    token_reserve: float = 0.0
    total_liquidity: float = 0.0
    price_token_per_denom: float = 0.0
    price_denom_per_token: float = 0.0
    last_update_block: int = 0
    last_sync_block: int = 0
    lifecycle: PoolLifecycle = PoolLifecycle.DISCOVERED
    can_buy: bool = False
    can_sell: bool = False

    # Cumulative volumes
    denom_volume_in: float = 0.0
    token_volume_in: float = 0.0
    denom_volume_out: float = 0.0
    token_volume_out: float = 0.0

    # Liquidity events
    total_mints: int = 0
    total_burns: int = 0
    total_swaps: int = 0




@dataclass
class PoolLiquiditySnapshot:
    """Immutable snapshot of a single pool’s liquidity characteristics."""

    pool_address: str
    denom_address: str
    protocol: str
    price: float
    denom_reserve: float
    token_reserve: float
    can_buy: bool
    can_sell: bool
