from dataclasses import dataclass
from typing import Any, Dict, List, Optional
from web3.types import ChecksumAddress


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


# ------------------------------------------------------------------------------
# ERC standards (log-derived events)
# ------------------------------------------------------------------------------


@dataclass
class ETHTransferEvent:
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class ERC20TransferEvent:
    token_address: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class ERC20ApprovalEvent:
    token_address: ChecksumAddress
    owner: ChecksumAddress
    spender: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class ERC721TransferEvent:
    token_address: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    token_id: int
    log_index: int


@dataclass
class ERC721ApprovalEvent:
    token_address: ChecksumAddress
    owner: ChecksumAddress
    approved_address: ChecksumAddress
    token_id: int
    log_index: int


@dataclass
class ERC1155TransferEvent:
    token_address: ChecksumAddress
    operator: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    token_ids: List[int]
    amounts: List[int]
    log_index: int


@dataclass
class DepositEvent:
    """Generic deposit events emitted by vaults, wrappers, or bridges."""

    id: Optional[int] = None
    token_address: Optional[str] = None
    withdrawal_address: Optional[str] = None
    amount: Optional[int] = None
    unlock_time: Optional[int] = None
    pair_address: Optional[str] = None  # Backwards compatibility for WETH deposits
    sender: Optional[str] = None
    log_index: Optional[int] = None


@dataclass
class WithdrawEvent:
    pair_address: ChecksumAddress
    sender: Optional[ChecksumAddress]
    amount: int
    log_index: int


@dataclass
class OwnershipTransferredEvent:
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


# ------------------------------------------------------------------------------
# Uniswap V2
# ------------------------------------------------------------------------------


@dataclass
class UniswapV2SyncEvent:
    pair_address: ChecksumAddress
    reserve0: int
    reserve1: int
    log_index: int


@dataclass
class UniswapV2SwapEvent:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    to: ChecksumAddress
    amount0In: int
    amount1In: int
    amount0Out: int
    amount1Out: int
    log_index: int


@dataclass
class UniswapV2MintEvent:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount0: int
    amount1: int
    log_index: int


@dataclass
class UniswapV2BurnEvent:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class UniswapV2PairCreatedEvent:
    pair_address: ChecksumAddress
    token0: ChecksumAddress
    token1: ChecksumAddress
    log_index: int


# ------------------------------------------------------------------------------
# Uniswap V3
# ------------------------------------------------------------------------------


@dataclass
class UniswapV3PoolCreatedEvent:
    token0: str
    token1: str
    fee: int
    tick_spacing: int
    pool: str
    log_index: int


@dataclass
class UniswapV3InitializeEvent:
    pool_address: str
    sqrt_price_x96: int
    tick: int
    log_index: int


@dataclass
class UniswapV3MintEvent:
    pool_address: str
    sender: str
    owner: str
    tick_lower: int
    tick_upper: int
    amount: int
    amount0: int
    amount1: int
    log_index: int


@dataclass
class UniswapV3BurnEvent:
    pool_address: str
    owner: str
    tick_lower: int
    tick_upper: int
    amount: int
    amount0: int
    amount1: int
    log_index: int


@dataclass
class UniswapV3SwapEvent:
    pool_address: str
    sender: str
    recipient: str
    amount0: int
    amount1: int
    sqrt_price_x96: int
    liquidity: int
    tick: int
    log_index: int


@dataclass
class UniswapV3PositionEvent:
    token_id: int
    liquidity: int
    amount0: int
    amount1: int
    pool_address: str
    owner: str
    tick_lower: int
    tick_upper: int
    log_index: int


@dataclass
class UniswapV3IncreaseLiquidityEvent:
    token_id: int
    liquidity: int
    amount0: int
    amount1: int
    pool_address: str
    log_index: int


@dataclass
class UniswapV3DecreaseLiquidityEvent:
    token_id: int
    liquidity: int
    amount0: int
    amount1: int
    pool_address: str
    log_index: int


@dataclass
class UniswapV3CollectEvent:
    token_id: int
    recipient: str
    amount0: int
    amount1: int
    pool_address: str
    log_index: int


# ------------------------------------------------------------------------------
# Uniswap V4
# ------------------------------------------------------------------------------


@dataclass
class UniswapV4InitializeEvent:
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


@dataclass
class UniswapV4ModifyLiquidityEvent:
    pool_manager_address: str
    event_id: str
    sender: str
    tick_lower: int
    tick_upper: int
    liquidity_delta: int
    salt: str
    log_index: int


@dataclass
class UniswapV4SwapEvent:
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


@dataclass
class UniswapV4DonateEvent:
    pool_manager_address: str
    event_id: str
    sender: str
    amount0: int
    amount1: int
    log_index: int


@dataclass
class Permit2Event:
    pool_manager_address: str
    owner: str
    token: str
    spender: str
    amount: int
    expiration: int
    nonce: int
    log_index: int


@dataclass
class UniswapV4FeeUpdatedEvent:
    pool_manager_address: str
    event_id: str
    protocol_fee: int
    log_index: int


@dataclass
class UniswapV4DynamicLPFeeUpdatedEvent:
    pool_manager_address: str
    event_id: str
    dynamic_lp_fee: int
    log_index: int


@dataclass
class UniswapV4FeeControllerUpdatedEvent:
    pool_manager_address: str
    protocol_fee_controller: str
    log_index: int


@dataclass
class UniswapV4BalanceDeltaEvent:
    pool_manager_address: str
    pool_id: str
    settler: str
    delta0: int
    delta1: int
    log_index: int


# ------------------------------------------------------------------------------
# Derived summaries
# ------------------------------------------------------------------------------


@dataclass
class DexSwapEvent:
    """High-level summary describing token-in/token-out movements for a swap."""

    dex_name: str
    token_in: ERC20TransferEvent
    token_out: ERC20TransferEvent
