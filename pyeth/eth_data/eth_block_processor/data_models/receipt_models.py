from dataclasses import dataclass, field
from decimal import Decimal
from typing import List, Optional, Dict, Any
from pyparsing import Union
from web3.types import TxData, TxReceipt, ChecksumAddress, EventData


@dataclass
class TransactionReceipt:
    transaction_hash: str
    block_hash: str
    block_number: int
    transaction_index: int
    from_address: str
    to_address: Optional[str]
    cumulative_gas_used: int
    gas_used: int
    contract_address: Optional[str]
    logs: List[Dict[str, Any]]
    logs_bloom: str
    status: int


@dataclass
class ERC20Transfer:
    token_address: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    amount: int
    log_index: int

    def __post_init__(self):
        # Convert amount to string if it's not already
        if not isinstance(self.amount, str):
            self.amount = str(self.amount)


@dataclass
class ERC721Transfer:
    token_address: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    token_id: int
    log_index: int

    def __post_init__(self):
        self.token_id = str(self.token_id)


@dataclass
class ERC1155Transfer:
    token_address: ChecksumAddress
    operator: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    token_ids: List[int]
    amounts: List[int]
    log_index: int

    def __post_init__(self):
        self.amounts = [str(amount) for amount in self.amounts]


@dataclass
class UniswapV2Sync:
    pair_address: ChecksumAddress
    reserve0: int
    reserve1: int
    log_index: int

    def __post_init__(self):
        self.reserve0 = str(self.reserve0)
        self.reserve1 = str(self.reserve1)


@dataclass
class UniswapV2Swap:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    to: ChecksumAddress
    amount0In: int
    amount1In: int
    amount0Out: int
    amount1Out: int
    log_index: int

    def __post_init__(self):
        self.amount0In = str(self.amount0In)
        self.amount1In = str(self.amount1In)
        self.amount0Out = str(self.amount0Out)
        self.amount1Out = str(self.amount1Out)


@dataclass
class ERC20Approval:
    token_address: ChecksumAddress
    owner: ChecksumAddress
    spender: ChecksumAddress
    amount: int
    log_index: int

    def __post_init__(self):
        self.amount = str(self.amount)


@dataclass
class ERC721Approval:
    token_address: ChecksumAddress
    owner: ChecksumAddress
    approved_address: ChecksumAddress
    token_id: int
    log_index: int

    def __post_init__(self):
        self.token_id = str(self.token_id)


@dataclass
class PairAction:
    pair_address: ChecksumAddress
    token0: ChecksumAddress
    token1: ChecksumAddress
    log_index: int

    def __post_init__(self):
        self.token0 = str(self.token0)
        self.token1 = str(self.token1)


@dataclass
class DepositAction:
    """Model for deposit events"""
    id: Optional[int] = None
    token_address: Optional[str] = None
    withdrawal_address: Optional[str] = None
    amount: Optional[Union[int, str]] = None
    unlock_time: Optional[int] = None
    pair_address: Optional[str] = None  # For backward compatibility
    sender: Optional[str] = None  # For backward compatibility
    log_index: Optional[int] = None

    def __post_init__(self):
        self.amount = str(self.amount)

@dataclass
class WithdrawAction:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount: int
    log_index: int

    def __post_init__(self):
        self.amount = str(self.amount)

@dataclass
class MintAction:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount0: int
    amount1: int
    log_index: int

    def __post_init__(self):
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)


@dataclass
class BurnAction:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount: int
    log_index: int

    def __post_init__(self):
        self.amount = str(self.amount)


@dataclass
class SwapAction:
    dex_name: str
    token_in: ERC20Transfer
    token_out: ERC20Transfer

    def __post_init__(self):
        self.token_in = str(self.token_in)
        self.token_out = str(self.token_out)


@dataclass
class OwnerEvent:
    """Owner transfer event"""
    contract_address: str
    previous_owner: str
    new_owner: str
    log_index: int


@dataclass
class TradingEnabledEvent:
    token_address: ChecksumAddress
    block_number: int
    log_index: int


@dataclass
class TradingDisabledEvent:
    token_address: ChecksumAddress
    block_number: int
    log_index: int


@dataclass
class TransactionAction:
    action_type: str  # e.g., "Swap", "Approve", "Transfer"
    description: str
    involved_addresses: List[str]
    involved_tokens: List[str]
    amounts: List[Decimal]
    additional_info: Dict[str, Any] = field(default_factory=dict)


@dataclass
class EventLog:
    block_number: int
    log_index: int    
    address: ChecksumAddress
    event_name: str
    params: Dict[str, Any]
    transaction_index: int
    transaction_hash: str



#--------------------------------
# Uniswap V3
#--------------------------------


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
        self.tick = str(self.tick)


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

    def __post_init__(self):
        self.amount = str(self.amount)
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)


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


#--------------------------------
# Uniswap V4
#--------------------------------

@dataclass
class UniswapV4Initialize:
    """
    Uniswap V4 Initialize Event
    Event Format:
      Initialize(bytes32 id, address indexed currency0, address indexed currency1, uint24 fee,
                 int24 tickSpacing, address hooks, uint160 sqrtPriceX96, int24 tick)
    """
    pool_manager_address: str
    event_id: str
    currency0: str
    currency1: str
    fee: int
    tick_spacing: int
    hooks: str
    sqrt_price_x96: int
    tick: int
    log_index: int

    def __post_init__(self):
        self.sqrt_price_x96 = str(self.sqrt_price_x96)


@dataclass
class UniswapV4ModifyLiquidity:
    """
    Uniswap V4 ModifyLiquidity Event
    Event Format:
      ModifyLiquidity(bytes32 id, address indexed sender, int24 tickLower, int24 tickUpper, int256 liquidityDelta, bytes32 salt)
    """
    pool_manager_address: str
    event_id: str
    sender: str
    tick_lower: int
    tick_upper: int
    liquidity_delta: int
    salt: str
    log_index: int


@dataclass
class Permit2:
    """
    Uniswap Protocol: Permit2 Event
    Event Format:
      Permit(address indexed owner, address indexed token, address indexed spender, uint160 amount, uint48 expiration, uint48 nonce)
    Example Log:
      Address: (Permit2 contract address, e.g. Uniswap Protocol: Permit2)
      Topics[1]: owner
      Topics[2]: token
      Topics[3]: spender
      Data: amount, expiration, nonce
    """
    pool_manager_address: str  # The contract address (emitter)
    owner: str
    token: str
    spender: str
    amount: int
    expiration: int
    nonce: int
    log_index: int

    def __post_init__(self):
        # Convert the amount to string for consistency with other models.
        if not isinstance(self.amount, str):
            self.amount = str(self.amount)


@dataclass
class UniswapV4Swap:
    """
    Uniswap V4 Swap Event
    Event Format:
      Swap(bytes32 indexed id, address indexed sender, int128 amount0, int128 amount1,
           uint160 sqrtPriceX96, uint128 liquidity, int24 tick, uint24 fee)
    """
    pool_manager_address: str
    event_id: str
    sender: str
    amount0: int
    amount1: int
    sqrt_price_x96: int
    liquidity: int
    tick: int
    fee: int
    log_index: int

    def __post_init__(self):
        self.amount0 = str(self.amount0)
        self.amount1 = str(self.amount1)
        self.sqrt_price_x96 = str(self.sqrt_price_x96)
        self.liquidity = str(self.liquidity)
