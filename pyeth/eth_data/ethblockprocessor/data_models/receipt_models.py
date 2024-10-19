from dataclasses import dataclass, field
from decimal import Decimal
from typing import List, Optional, Dict, Any
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


@dataclass
class ERC721Transfer:
    token_address: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    token_id: int
    log_index: int


@dataclass
class ERC1155Transfer:
    token_address: ChecksumAddress
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    token_ids: List[int]
    amounts: List[int]
    log_index: int


@dataclass
class UniswapV2Sync:
    pair_address: ChecksumAddress
    reserve0: int
    reserve1: int
    log_index: int
    

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


@ dataclass
class UniswapV3Swap:
    pool_address: ChecksumAddress
    sender: ChecksumAddress
    recipient: ChecksumAddress
    amount_in: int
    amount_out: int
    log_index: int


@dataclass
class ERC20Approval:
    token_address: ChecksumAddress
    owner: ChecksumAddress
    spender: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class ERC721Approval:
    token_address: ChecksumAddress
    owner: ChecksumAddress
    approved_address: ChecksumAddress
    token_id: int
    log_index: int


@dataclass
class PairAction:
    pair_address: ChecksumAddress
    token0: ChecksumAddress
    token1: ChecksumAddress
    log_index: int


@dataclass
class DepositAction:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class WithdrawAction:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class MintAction:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount0: int
    amount1: int
    log_index: int


@dataclass
class BurnAction:
    pair_address: ChecksumAddress
    sender: ChecksumAddress
    amount: int
    log_index: int


@dataclass
class SwapAction:
    dex_name: str
    token_in: ERC20Transfer
    token_out: ERC20Transfer


@dataclass
class OwnerEvent:
    pair_address: ChecksumAddress
    previous_owner: ChecksumAddress
    new_owner: ChecksumAddress
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
    