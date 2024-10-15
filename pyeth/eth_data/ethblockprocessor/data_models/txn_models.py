from dataclasses import dataclass, field
from decimal import Decimal
from enum import Enum
from typing import List, Optional, Dict, Any, Union
from web3.types import ChecksumAddress, Wei, Hash32
from ethblockprocessor.data_models.receipt_models import *
from ethblockprocessor.data_models.trace_models import *


class TransactionType(Enum):
    ETH_TRANSFER = "Ether Transfer"
    CONTRACT_CREATION = "Contract Creation"
    CONTRACT_INTERACTION = "Contract Interaction"
    ERC20_TRANSFER = "ERC20 Transfer"
    ERC721_TRANSFER = "ERC721 Transfer"
    ERC1155_TRANSFER = "ERC1155 Transfer"
    INTERNAL_TRANSACTION = "Internal Transaction"
    DEFI_TRANSACTION = "DeFi Transaction"
    LAYER2_TRANSACTION = "Layer 2 Transaction"
    CONTRACT_UPGRADE = "Contract Upgrade"
    APPROVE = "Approval"
    UNKNOWN = "Unknown"


@dataclass
class ETHTransfer:
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    amount: Wei

@dataclass
class ContractInteraction:
    contract_address: ChecksumAddress
    contract_name: Optional[str]
    method_name: str
    decoded_input: Dict[str, Any]
    value: Wei


@dataclass
class TransactionFees:
    gas_price: Wei
    gas_used: int
    total_fee: Wei
    

@dataclass
class DetailedTransaction:
    hash: str
    block_number: int
    txn_index: int
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    value: Wei
    status: bool
    nonce: int
    input: str
    
    txn_type: TransactionType
    actions: List[TransactionAction]
    
    eth_transfers: List[ETHTransfer]
    erc20_transfers: List[ERC20Transfer]
    erc721_transfers: List[ERC721Transfer]
    erc1155_transfers: List[ERC1155Transfer]
    internal_transactions: List[InternalTransaction]
    uniswap_v2_syncs: List[UniswapV2Sync]
    uniswap_v2_swaps: List[UniswapV2Swap]
    approvals: List[ERC20Approval]
    mints: List[MintAction]
    burns: List[BurnAction]
    deposits: List[DepositAction]
    withdraws: List[WithdrawAction]
    pair_events: List[PairAction]
    owner_events: List[OwnerEvent]
    contract_interactions: List[ContractInteraction]
    other_events: List[Dict[str, Any]]

    fees: TransactionFees

    state_diffs: Dict[str, Any]
    latest_states: Dict[str, Any]
