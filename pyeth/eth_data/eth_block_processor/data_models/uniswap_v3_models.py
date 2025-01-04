from dataclasses import dataclass
from typing import Optional

@dataclass
class UniswapV3PoolCreated:
    """Pool Created Event
    event PoolCreated(address indexed token0, address indexed token1, uint24 indexed fee, int24 tickSpacing, address pool)
    """
    token0: str
    token1: str
    fee: int
    tick_spacing: int
    pool: str
    log_index: int

    def get_tick_spacing(self) -> int:
        """Calculate tick spacing based on fee tier"""
        if self.fee == 100:
            return 1
        elif self.fee == 500:
            return 10
        elif self.fee == 3000:
            return 60
        elif self.fee == 10000:
            return 200
        return 0

@dataclass
class UniswapV3Initialize:
    """Initialize Event
    event Initialize(uint160 sqrtPriceX96, int24 tick)
    """
    pool_address: str
    sqrt_price_x96: int
    tick: int
    log_index: int

    def __post_init__(self):
        self.sqrt_price_x96 = str(self.sqrt_price_x96)

@dataclass
class UniswapV3Mint:
    """Mint Event
    event Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
    """
    pool_address: str
    sender: str
    owner: str
    tick_lower: int
    tick_upper: int
    amount: int
    amount0: int
    amount1: int
    log_index: int

    def __post_init__(self):
        self.amount = str(self.amount)
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)

@dataclass
class UniswapV3Position:
    token_id: int
    liquidity: int
    amount0: int
    amount1: int
    pool_address: str
    owner: str
    tick_lower: int
    tick_upper: int
    log_index: int

    def __post_init__(self):
        self.token_id = str(self.token_id)
        self.liquidity = str(self.liquidity)
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)

@dataclass
class UniswapV3Swap:
    """Swap Event
    event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
    """
    pool_address: str
    sender: str
    recipient: str
    amount0: int
    amount1: int
    sqrt_price_x96: int
    liquidity: int
    tick: int
    log_index: int

    def __post_init__(self):
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)
        self.sqrt_price_x96 = str(self.sqrt_price_x96)
        self.liquidity = str(self.liquidity)

@dataclass
class UniswapV3Burn:
    """Burn Event
    event Burn(address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
    """
    pool_address: str
    owner: str
    tick_lower: int
    tick_upper: int
    amount: int
    amount0: int
    amount1: int
    log_index: int

@dataclass
class UniswapV3DecreaseLiquidity:
    """DecreaseLiquidity Event
    event DecreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
    """
    token_id: int
    liquidity: int
    amount0: int
    amount1: int
    pool_address: str
    log_index: int

    def __post_init__(self):
        self.token_id = str(self.token_id)
        self.liquidity = str(self.liquidity)
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)

@dataclass
class UniswapV3IncreaseLiquidity:
    """IncreaseLiquidity Event
    event IncreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
    """
    token_id: int
    liquidity: int
    amount0: int
    amount1: int
    pool_address: str
    log_index: int

    def __post_init__(self):
        self.token_id = str(self.token_id)
        self.liquidity = str(self.liquidity)
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)

@dataclass
class UniswapV3Collect:
    """Collect Event
    event Collect(uint256 indexed tokenId, address recipient, uint256 amount0, uint256 amount1)
    """
    token_id: int
    recipient: str
    amount0: int
    amount1: int
    pool_address: str
    log_index: int