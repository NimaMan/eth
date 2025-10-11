import numpy as np
from dataclasses import dataclass, field
import json
from enum import Enum
from typing import List, Optional, Dict, Any, Union, Set
from web3.types import ChecksumAddress, Wei, Hash32
from eth_data.tx_processor.data_models.receipt_models import *
from eth_data.tx_processor.data_models.trace_models import *
from eth_data.utils.type_converter import (
    convert_to_int, convert_to_hex_str, normalize_address, convert_block_number,
    convert_transaction_index, convert_status
)


@dataclass
class ETHTransfer:
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    amount: Wei

    def __post_init__(self):
        # Store ETH amount in wei as string to avoid 64-bit overflows during JSON serialization
        if not isinstance(self.amount, str):
            self.amount = str(self.amount)

@dataclass
class ContractCreationEvent:
    contract_address: ChecksumAddress
    contract_type: str
    symbol: Optional[str]
    decimals: Optional[int]
    name: Optional[str]
    total_supply: Optional[int]

@dataclass
class TransactionFees:
    gas_price: Wei  # Effective gas price paid (for backward compatibility)
    gas_used: int
    tx_fee: Wei    # Total transaction fee in ETH (gas_price * gas_used)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "gas_price": self.gas_price,
            "gas_used": self.gas_used,
            "tx_fee": self.tx_fee,
            "protocol_type": self.protocol_type,
            "max_fee_per_gas": self.max_fee_per_gas,
            "max_priority_fee": self.max_priority_fee,
        }
    # New fields for gas ranking
    protocol_type: str = "unknown"  # "legacy", "eip1559", "eip2930"
    max_fee_per_gas: Optional[Wei] = None  # User's max willingness
    max_priority_fee: Optional[Wei] = None  # User's max tip (EIP-1559 only)
    

@dataclass
class ProcessedTransaction:
    hash: str
    block_number: int
    block_timestamp: int
    tx_index: int
    from_address: ChecksumAddress
    to_address: Optional[ChecksumAddress]
    contract_address: Optional[ChecksumAddress]
    value: int 
    status: str
    nonce: int
    
    tx_type: str
    actions: List[str]
    
    fees: TransactionFees
    bribe_amount: float = 0
    unique_addresses: Set[ChecksumAddress] = field(default_factory=set)
    erc20_contracts: Set[ChecksumAddress] = field(default_factory=set)
    erc721_contracts: Set[ChecksumAddress] = field(default_factory=set)
    erc1155_contracts: Set[ChecksumAddress] = field(default_factory=set)
   
    eth_transfers: List[ETHTransfer] = field(default_factory=list)
    erc20_transfers: List[ERC20Transfer] = field(default_factory=list)
    erc721_transfers: List[ERC721Transfer] = field(default_factory=list)
    erc1155_transfers: List[ERC1155Transfer] = field(default_factory=list)
    internal_transactions: List[InternalTransaction] = field(default_factory=list)
    uniswap_v2_syncs: List[UniswapV2Sync] = field(default_factory=list)
    uniswap_v2_swaps: List[UniswapV2Swap] = field(default_factory=list)
    approvals: List[ERC20Approval] = field(default_factory=list)
    erc721_approvals: List['ERC721Approval'] = field(default_factory=list)
    mints: List[MintAction] = field(default_factory=list)
    burns: List[BurnAction] = field(default_factory=list)
    deposits: List[DepositAction] = field(default_factory=list)
    withdraws: List[WithdrawAction] = field(default_factory=list)
    pair_events: List[PairAction] = field(default_factory=list)
    owner_events: List[OwnerEvent] = field(default_factory=list)
    contract_creation_events: List[ContractCreationEvent] = field(default_factory=list)
    trading_enabled_events: List[TradingEnabledEvent] = field(default_factory=list)
    trading_disabled_events: List[TradingDisabledEvent] = field(default_factory=list)

    # Uniswap V3 specific fields
    uniswap_v3_pools: List[UniswapV3PoolCreated] = field(default_factory=list)
    uniswap_v3_initializations: List[UniswapV3Initialize] = field(default_factory=list)
    uniswap_v3_burns: List[UniswapV3Burn] = field(default_factory=list)
    uniswap_v3_mints: List[UniswapV3Mint] = field(default_factory=list)
    uniswap_v3_swaps: List[UniswapV3Swap] = field(default_factory=list)
    uniswap_v3_positions: List[UniswapV3Position] = field(default_factory=list)
    uniswap_v3_increases: List[UniswapV3IncreaseLiquidity] = field(default_factory=list)
    uniswap_v3_decreases: List[UniswapV3DecreaseLiquidity] = field(default_factory=list)
    
    # Uniswap V4 specific fields
    uniswap_v4_initializes: List[UniswapV4Initialize] = field(default_factory=list)
    uniswap_v4_modifies: List[UniswapV4ModifyLiquidity] = field(default_factory=list)
    uniswap_v4_swaps: List[UniswapV4Swap] = field(default_factory=list)
    permit2_events: List[Permit2] = field(default_factory=list)

    other_events: List[Dict[str, Any]] = field(default_factory=list)
    address_balance_changes: Dict[str, Any] = field(default_factory=dict)
    latest_states: Dict[str, Any] = field(default_factory=dict)
    input: str = ""
    
    @staticmethod
    def _list_to_dicts(items: Optional[List[Any]]) -> Optional[List[Dict[str, Any]]]:
        if items is None:
            return None
        out: List[Dict[str, Any]] = []
        for x in items:
            if hasattr(x, "to_dict"):
                out.append(x.to_dict())
            elif hasattr(x, "__dict__"):
                out.append(dict(x.__dict__))
            else:
                out.append(x)
        return out

    @staticmethod
    def _set_to_list(s: Optional[Set[Any]]) -> Optional[List[Any]]:
        if s is None:
            return None
        return list(s)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "hash": self.hash,
            "block_number": self.block_number,
            "block_timestamp": self.block_timestamp,
            "tx_index": self.tx_index,
            "from_address": self.from_address,
            "to_address": self.to_address,
            "contract_address": self.contract_address,
            "value": self.value,
            "status": self.status,
            "nonce": self.nonce,
            "tx_type": self.tx_type,
            "actions": self.actions,
            "fees": self.fees.to_dict() if isinstance(self.fees, TransactionFees) else self.fees,
            "bribe_amount": self.bribe_amount,
            "unique_addresses": self._set_to_list(self.unique_addresses),
            "erc20_contracts": self._set_to_list(self.erc20_contracts),
            "erc721_contracts": self._set_to_list(self.erc721_contracts),
            "erc1155_contracts": self._set_to_list(self.erc1155_contracts),
            "eth_transfers": self._list_to_dicts(self.eth_transfers),
            "erc20_transfers": self._list_to_dicts(self.erc20_transfers),
            "erc721_transfers": self._list_to_dicts(self.erc721_transfers),
            "erc1155_transfers": self._list_to_dicts(self.erc1155_transfers),
            "internal_transactions": self._list_to_dicts(self.internal_transactions),
            "uniswap_v2_syncs": self._list_to_dicts(self.uniswap_v2_syncs),
            "uniswap_v2_swaps": self._list_to_dicts(self.uniswap_v2_swaps),
            "approvals": self._list_to_dicts(self.approvals),
            "erc721_approvals": self._list_to_dicts(self.erc721_approvals),
            "mints": self._list_to_dicts(self.mints),
            "burns": self._list_to_dicts(self.burns),
            "deposits": self._list_to_dicts(self.deposits),
            "withdraws": self._list_to_dicts(self.withdraws),
            "pair_events": self._list_to_dicts(self.pair_events),
            "owner_events": self._list_to_dicts(self.owner_events),
            "contract_creation_events": self._list_to_dicts(self.contract_creation_events),
            "trading_enabled_events": self._list_to_dicts(self.trading_enabled_events),
            "trading_disabled_events": self._list_to_dicts(self.trading_disabled_events),
            "uniswap_v3_pools": self._list_to_dicts(self.uniswap_v3_pools),
            "uniswap_v3_initializations": self._list_to_dicts(self.uniswap_v3_initializations),
            "uniswap_v3_burns": self._list_to_dicts(self.uniswap_v3_burns),
            "uniswap_v3_mints": self._list_to_dicts(self.uniswap_v3_mints),
            "uniswap_v3_swaps": self._list_to_dicts(self.uniswap_v3_swaps),
            "uniswap_v3_positions": self._list_to_dicts(self.uniswap_v3_positions),
            "uniswap_v3_increases": self._list_to_dicts(self.uniswap_v3_increases),
            "uniswap_v3_decreases": self._list_to_dicts(self.uniswap_v3_decreases),
            "uniswap_v4_initializes": self._list_to_dicts(self.uniswap_v4_initializes),
            "uniswap_v4_modifies": self._list_to_dicts(self.uniswap_v4_modifies),
            "uniswap_v4_swaps": self._list_to_dicts(self.uniswap_v4_swaps),
            "permit2_events": self._list_to_dicts(self.permit2_events),
            "other_events": self.other_events,
            "state_changes": self.address_balance_changes,
            "latest_states": self.latest_states,
            "input": self.input,
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), separators=(",", ":"))
    
    @classmethod
    def from_dict(cls, tx_dict: Dict[str, Any]) -> 'ProcessedTransaction':
        """
        Create ProcessedTransaction from a dictionary.
        
        The dict is expected to be from an already processed transaction,
        so it should have the basic fields populated.
        """
        # Extract fees or create default
        fees_data = tx_dict.get('fees', {})
        if isinstance(fees_data, TransactionFees):
            fees = fees_data
        elif isinstance(fees_data, dict):
            fees = TransactionFees(
                gas_price=fees_data.get('gas_price', 0),
                gas_used=fees_data.get('gas_used', 0),
                tx_fee=fees_data.get('tx_fee', 0),
                protocol_type=fees_data.get('protocol_type', 'unknown'),
                max_fee_per_gas=fees_data.get('max_fee_per_gas'),
                max_priority_fee=fees_data.get('max_priority_fee')
            )
        else:
            fees = TransactionFees(gas_price=0, gas_used=0, tx_fee=0)
        
        return cls(
            hash=tx_dict['hash'],
            block_number=tx_dict['block_number'],
            tx_index=tx_dict.get('tx_index', 0),
            from_address=tx_dict['from_address'],
            to_address=tx_dict.get('to_address'),
            contract_address=tx_dict.get('contract_address'),
            value=tx_dict.get('value', 0),
            status=tx_dict.get('status', 'success'),
            nonce=tx_dict.get('nonce', 0),
            input=tx_dict.get('input', '0x'),
            tx_type=tx_dict.get('tx_type', 'unknown'),
            actions=tx_dict.get('actions'),
            eth_transfers=tx_dict.get('eth_transfers'),
            erc20_transfers=tx_dict.get('erc20_transfers'),
            erc721_transfers=tx_dict.get('erc721_transfers'),
            erc1155_transfers=tx_dict.get('erc1155_transfers'),
            internal_transactions=tx_dict.get('internal_transactions'),
            uniswap_v2_syncs=tx_dict.get('uniswap_v2_syncs'),
            uniswap_v2_swaps=tx_dict.get('uniswap_v2_swaps'),
            approvals=tx_dict.get('approvals'),
            erc721_approvals=tx_dict.get('erc721_approvals'),
            mints=tx_dict.get('mints'),
            burns=tx_dict.get('burns'),
            deposits=tx_dict.get('deposits'),
            withdraws=tx_dict.get('withdraws'),
            pair_events=tx_dict.get('pair_events'),
            owner_events=tx_dict.get('owner_events'),
            contract_creation_events=tx_dict.get('contract_creation_events'),
            trading_enabled_events=tx_dict.get('trading_enabled_events'),
            trading_disabled_events=tx_dict.get('trading_disabled_events'),
            uniswap_v3_pools=tx_dict.get('uniswap_v3_pools'),
            uniswap_v3_initializations=tx_dict.get('uniswap_v3_initializations'),
            uniswap_v3_burns=tx_dict.get('uniswap_v3_burns'),
            uniswap_v3_mints=tx_dict.get('uniswap_v3_mints'),
            uniswap_v3_swaps=tx_dict.get('uniswap_v3_swaps'),
            uniswap_v3_positions=tx_dict.get('uniswap_v3_positions'),
            uniswap_v3_increases=tx_dict.get('uniswap_v3_increases'),
            uniswap_v3_decreases=tx_dict.get('uniswap_v3_decreases'),
            uniswap_v4_initializes=tx_dict.get('uniswap_v4_initializes'),
            uniswap_v4_modifies=tx_dict.get('uniswap_v4_modifies'),
            uniswap_v4_swaps=tx_dict.get('uniswap_v4_swaps'),
            permit2_events=tx_dict.get('permit2_events'),
            other_events=tx_dict.get('other_events'),
            fees=fees,
            unique_addresses=tx_dict.get('unique_addresses'),
            erc20_contracts=tx_dict.get('erc20_contracts'),
            erc721_contracts=tx_dict.get('erc721_contracts'),
            erc1155_contracts=tx_dict.get('erc1155_contracts'),
            state_changes=tx_dict.get('state_changes'),
            latest_states=tx_dict.get('latest_states'),
            bribe_amount=tx_dict.get('bribe_amount', 0),
            block_timestamp=tx_dict.get('block_timestamp', tx_dict.get('timestamp', 0))
        )
    
    def __init__(self, 
                 hash: str,
                 block_number: int,
                 tx_index: int,
                 from_address: str,
                 to_address: Optional[str],
                 contract_address: Optional[str],
                 value: np.float64,
                 status: str,
                 nonce: int,
                 input: str,
                 tx_type: str,
                 actions: Optional[List[str]] = None,
                 eth_transfers: Optional[List[ETHTransfer]] = None,
                 erc20_transfers: Optional[List[ERC20Transfer]] = None,
                 erc721_transfers: Optional[List[ERC721Transfer]] = None,
                 erc1155_transfers: Optional[List[ERC1155Transfer]] = None,
                 internal_transactions: Optional[List[InternalTransaction]] = None,
                 uniswap_v2_syncs: Optional[List[UniswapV2Sync]] = None,
                 uniswap_v2_swaps: Optional[List[UniswapV2Swap]] = None,
                 approvals: Optional[List[ERC20Approval]] = None,
                 erc721_approvals: Optional[List['ERC721Approval']] = None,
                 mints: Optional[List[MintAction]] = None,
                 burns: Optional[List[BurnAction]] = None,
                 deposits: Optional[List[DepositAction]] = None,
                 withdraws: Optional[List[WithdrawAction]] = None,
                 pair_events: Optional[List[PairAction]] = None,
                 owner_events: Optional[List[OwnerEvent]] = None,
                 contract_creation_events: Optional[List[ContractCreationEvent]] = None,
                 trading_enabled_events: Optional[List[TradingEnabledEvent]] = None,
                 trading_disabled_events: Optional[List[TradingDisabledEvent]] = None,
                 uniswap_v3_pools: Optional[List[UniswapV3PoolCreated]] = None,
                 uniswap_v3_initializations: Optional[List[UniswapV3Initialize]] = None,
                 uniswap_v3_burns: Optional[List[UniswapV3Burn]] = None,
                 uniswap_v3_mints: Optional[List[UniswapV3Mint]] = None,
                 uniswap_v3_swaps: Optional[List[UniswapV3Swap]] = None,
                 uniswap_v3_positions: Optional[List[UniswapV3Position]] = None,
                 uniswap_v3_increases: Optional[List[UniswapV3IncreaseLiquidity]] = None,
                 uniswap_v3_decreases: Optional[List[UniswapV3DecreaseLiquidity]] = None,
                 uniswap_v4_initializes: Optional[List[UniswapV4Initialize]] = None,
                 uniswap_v4_modifies: Optional[List[UniswapV4ModifyLiquidity]] = None,
                 uniswap_v4_swaps: Optional[List[UniswapV4Swap]] = None,
                 permit2_events: Optional[List[Permit2]] = None,
                 other_events: Optional[List[Dict[str, Any]]] = None,
                 fees: Optional[TransactionFees] = None,
                 unique_addresses: Optional[Set[ChecksumAddress]] = None,
                 erc20_contracts: Optional[Set[ChecksumAddress]] = None,
                 erc721_contracts: Optional[Set[ChecksumAddress]] = None,
                 erc1155_contracts: Optional[Set[ChecksumAddress]] = None,
                 address_balance_changes: Optional[Dict[str, Any]] = None,
                 latest_states: Optional[Dict[str, Any]] = None,
                 bribe_amount: float = 0,
                 block_timestamp: int = 0):
        """Initialize DetailedTransaction with type conversion handling"""
        
        # Core transaction fields
        self.hash = convert_to_hex_str(hash)
        self.block_number = convert_block_number(block_number)
        self.tx_index = convert_transaction_index(tx_index)
        self.from_address = normalize_address(from_address)
        self.to_address = normalize_address(to_address) if to_address else None
        self.value = value
        self.contract_address = normalize_address(contract_address) if contract_address else None
        self.status = convert_status(status)
        self.nonce = convert_to_int(nonce)
        self.input = convert_to_hex_str(input)
        self.tx_type = tx_type

        # Lists initialization with empty defaults
        self.actions = actions or []
        self.eth_transfers = eth_transfers or []
        self.erc20_transfers = erc20_transfers or []
        self.erc721_transfers = erc721_transfers or []
        self.erc1155_transfers = erc1155_transfers or []
        self.internal_transactions = internal_transactions or []
        self.uniswap_v2_syncs = uniswap_v2_syncs or []
        self.uniswap_v2_swaps = uniswap_v2_swaps or []
        self.approvals = approvals or []
        self.erc721_approvals = erc721_approvals or []
        self.mints = mints or []
        self.burns = burns or []
        self.deposits = deposits or []
        self.withdraws = withdraws or []
        self.pair_events = pair_events or []
        self.owner_events = owner_events or []
        self.contract_creation_events = contract_creation_events or []
        self.trading_enabled_events = trading_enabled_events or []
        self.trading_disabled_events = trading_disabled_events or []
        
        # Uniswap V3 specific fields
        self.uniswap_v3_pools = uniswap_v3_pools or []
        self.uniswap_v3_initializations = uniswap_v3_initializations or []
        self.uniswap_v3_mints = uniswap_v3_mints or []
        self.uniswap_v3_swaps = uniswap_v3_swaps or []
        self.uniswap_v3_positions = uniswap_v3_positions or []
        self.uniswap_v3_burns = uniswap_v3_burns or []
        self.uniswap_v3_increases = uniswap_v3_increases or []
        self.uniswap_v3_decreases = uniswap_v3_decreases or []
        self.other_events = other_events or []

        # Uniswap V4 specific fields
        self.uniswap_v4_initializes = uniswap_v4_initializes or []
        self.uniswap_v4_modifies = uniswap_v4_modifies or []
        self.uniswap_v4_swaps = uniswap_v4_swaps or []
        self.permit2_events = permit2_events or []

        # Complex fields
        self.fees = fees or TransactionFees(gas_price=0, gas_used=0, tx_fee=0)
        self.unique_addresses = unique_addresses or set()
        self.erc20_contracts = erc20_contracts or set()
        self.erc721_contracts = erc721_contracts or set()
        self.erc1155_contracts = erc1155_contracts or set()
        self.address_balance_changes = address_balance_changes or {}
        self.latest_states = latest_states or {}
        self.bribe_amount = float(bribe_amount)
        self.block_timestamp = block_timestamp

    def __post_init__(self):
        """Validate the transaction data after initialization"""
        if not self.hash:
            raise ValueError("Transaction hash cannot be empty")
        if self.block_number < 0:
            raise ValueError("Block number cannot be negative")
        if self.tx_index < 0:
            raise ValueError("Transaction index cannot be negative")

    def __eq__(self, other):
        if not isinstance(other, ProcessedTransaction):
            return False
            
        # Compare all fields except sets
        basic_fields_match = (
            self.hash == other.hash and
            self.block_number == other.block_number and
            self.tx_index == other.tx_index and
            self.from_address == other.from_address and
            self.to_address == other.to_address and
            self.contract_address == other.contract_address and
            self.value == other.value and
            self.status == other.status and
            self.nonce == other.nonce and
            self.tx_type == other.tx_type and
            self.erc20_transfers == other.erc20_transfers and
            self.eth_transfers == other.eth_transfers and
            self.mints == other.mints and
            self.burns == other.burns and
            self.deposits == other.deposits and
            self.withdraws == other.withdraws and
            self.pair_events == other.pair_events and
            self.owner_events == other.owner_events and
            self.contract_creation_events == other.contract_creation_events and
            self.trading_enabled_events == other.trading_enabled_events and
            self.trading_disabled_events == other.trading_disabled_events and
            self.uniswap_v3_pools == other.uniswap_v3_pools and
            self.uniswap_v3_initializations == other.uniswap_v3_initializations and
            self.uniswap_v3_mints == other.uniswap_v3_mints and
            self.uniswap_v3_swaps == other.uniswap_v3_swaps and
            self.uniswap_v3_positions == other.uniswap_v3_positions and
            self.uniswap_v3_increases == other.uniswap_v3_increases and
            self.uniswap_v3_decreases == other.uniswap_v3_decreases and
            self.uniswap_v4_initializes == other.uniswap_v4_initializes and
            self.uniswap_v4_modifies == other.uniswap_v4_modifies and
            self.uniswap_v4_swaps == other.uniswap_v4_swaps and
            self.permit2_events == other.permit2_events and
            self.other_events == other.other_events and
            self.fees == other.fees and
            self.address_balance_changes == other.address_balance_changes and
            self.latest_states == other.latest_states and
            self.bribe_amount == other.bribe_amount
        )
        
        # Compare sets separately (order doesn't matter)
        sets_match = (
            set(self.unique_addresses) == set(other.unique_addresses) and
            set(self.erc20_contracts) == set(other.erc20_contracts) and 
            set(self.actions) == set(other.actions)
        )
        
        return basic_fields_match and sets_match
