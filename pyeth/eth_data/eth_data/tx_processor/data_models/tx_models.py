from dataclasses import dataclass, field, is_dataclass
import json
from typing import List, Optional, Dict, Any, Set, Union, get_args, get_origin, get_type_hints
from web3.types import ChecksumAddress
from hexbytes import HexBytes
from web3 import Web3
from eth_data.tx_processor.data_models.receipt_models import *
from eth_data.tx_processor.data_models.trace_models import *

def _ensure_int(value: Any, field: str) -> int:
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        if value.is_integer():
            return int(value)
        raise TypeError(f"{field} float values must be integral")
    if isinstance(value, str):
        if value.startswith("0x"):
            return int(value, 16)
        return int(value)
    if isinstance(value, (bytes, bytearray, HexBytes)):
        return int.from_bytes(bytes(value), byteorder="big")
    raise TypeError(f"{field} must be an int-compatible value, got {type(value).__name__}")


def _ensure_hex_str(value: Any, field: str) -> str:
    if isinstance(value, str):
        return value if value.startswith("0x") else f"0x{value}"
    if isinstance(value, (bytes, bytearray, HexBytes)):
        return "0x" + bytes(value).hex()
    if isinstance(value, int):
        return hex(value)
    raise TypeError(f"{field} must be bytes or hex string, got {type(value).__name__}")


def _ensure_status_bool(value: Any) -> bool:
    if isinstance(value, str):
        stripped = value.strip()
        lowered = stripped.lower()
        if lowered.startswith("0x"):
            try:
                parsed = int(stripped, 16)
            except ValueError as exc:
                raise ValueError(f"Unrecognized status string: {value}") from exc
            return parsed != 0
        if lowered in {"success", "succeeded", "ok", "true", "1"}:
            return True
        if lowered in {"failed", "fail", "reverted", "false", "0"}:
            return False
        raise ValueError(f"Unrecognized status string: {value}")
    return bool(value)


def _coerce_checksum_address(address: Any, allow_none: bool = False) -> Optional[ChecksumAddress]:
    if address is None:
        if allow_none:
            return None
        raise TypeError("Address cannot be None")
    if isinstance(address, str):
        if address == "":
            if allow_none:
                return None
            raise ValueError("Empty string is not a valid address")
        return Web3.to_checksum_address(address)
    if isinstance(address, (bytes, bytearray, HexBytes)):
        return Web3.to_checksum_address("0x" + bytes(address).hex())
    raise TypeError(f"Address must be hex string/bytes, got {type(address).__name__}")


@dataclass
class ETHTransfer:
    from_address: ChecksumAddress
    to_address: ChecksumAddress
    amount: int

    @property
    def amount_eth(self) -> float:
        return self.amount / 1e18

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
    gas_price: int  # Effective gas price paid (wei)
    gas_used: int
    tx_fee: int  # Total fee in wei (gas_price * gas_used)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "gas_price": self.gas_price,
            "gas_used": self.gas_used,
            "tx_fee": self.tx_fee,
            "protocol_type": self.protocol_type,
            "max_fee_per_gas": self.max_fee_per_gas,
            "max_priority_fee": self.max_priority_fee,
        }

    @property
    def tx_fee_eth(self) -> float:
        return self.tx_fee / 1e18

    protocol_type: str = "unknown"  # "legacy", "eip1559", "eip2930"
    max_fee_per_gas: Optional[int] = None  # User's max willingness (wei)
    max_priority_fee: Optional[int] = None  # User's max tip (wei)

    def __post_init__(self) -> None:
        self.gas_price = _ensure_int(self.gas_price, "fees.gas_price")
        self.gas_used = _ensure_int(self.gas_used, "fees.gas_used")
        self.tx_fee = _ensure_int(self.tx_fee, "fees.tx_fee")
        if self.max_fee_per_gas is not None:
            self.max_fee_per_gas = _ensure_int(self.max_fee_per_gas, "fees.max_fee_per_gas")
        if self.max_priority_fee is not None:
            self.max_priority_fee = _ensure_int(self.max_priority_fee, "fees.max_priority_fee")

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
    status: bool
    nonce: int
    
    tx_type: str
    actions: List[str]
    
    fees: TransactionFees
    bribe_amount: int = 0
    unique_addresses: Set[ChecksumAddress] = field(default_factory=set)
    erc20_contracts: Set[ChecksumAddress] = field(default_factory=set)
    erc721_contracts: Set[ChecksumAddress] = field(default_factory=set)
    erc1155_contracts: Set[ChecksumAddress] = field(default_factory=set)
   
    eth_transfers: List[ETHTransfer] = field(default_factory=list)
    erc20_transfers: List[ERC20TransferEvent] = field(default_factory=list)
    erc721_transfers: List[ERC721TransferEvent] = field(default_factory=list)
    erc1155_transfers: List[ERC1155TransferEvent] = field(default_factory=list)
    internal_transactions: List[InternalTransaction] = field(default_factory=list)
    uniswap_v2_syncs: List[UniswapV2SyncEvent] = field(default_factory=list)
    uniswap_v2_swaps: List[UniswapV2SwapEvent] = field(default_factory=list)
    erc20_approval_events: List[ERC20ApprovalEvent] = field(default_factory=list)
    erc721_approval_events: List[ERC721ApprovalEvent] = field(default_factory=list)
    uniswap_v2_mints: List[UniswapV2MintEvent] = field(default_factory=list)
    uniswap_v2_burns: List[UniswapV2BurnEvent] = field(default_factory=list)
    deposit_events: List[DepositEvent] = field(default_factory=list)
    withdraw_events: List[WithdrawEvent] = field(default_factory=list)
    uniswap_v2_pair_created_events: List[UniswapV2PairCreatedEvent] = field(default_factory=list)
    ownership_transferred_events: List[OwnershipTransferredEvent] = field(default_factory=list)
    contract_creation_events: List[ContractCreationEvent] = field(default_factory=list)
    trading_enabled_events: List[TradingEnabledEvent] = field(default_factory=list)
    trading_disabled_events: List[TradingDisabledEvent] = field(default_factory=list)

    # Uniswap V3 specific fields
    uniswap_v3_pools: List[UniswapV3PoolCreatedEvent] = field(default_factory=list)
    uniswap_v3_initializations: List[UniswapV3InitializeEvent] = field(default_factory=list)
    uniswap_v3_burns: List[UniswapV3BurnEvent] = field(default_factory=list)
    uniswap_v3_mints: List[UniswapV3MintEvent] = field(default_factory=list)
    uniswap_v3_swaps: List[UniswapV3SwapEvent] = field(default_factory=list)
    uniswap_v3_positions: List[UniswapV3PositionEvent] = field(default_factory=list)
    uniswap_v3_increases: List[UniswapV3IncreaseLiquidityEvent] = field(default_factory=list)
    uniswap_v3_decreases: List[UniswapV3DecreaseLiquidityEvent] = field(default_factory=list)
    
    # Uniswap V4 specific fields
    uniswap_v4_initializes: List[UniswapV4InitializeEvent] = field(default_factory=list)
    uniswap_v4_modifies: List[UniswapV4ModifyLiquidityEvent] = field(default_factory=list)
    uniswap_v4_swaps: List[UniswapV4SwapEvent] = field(default_factory=list)
    uniswap_v4_donates: List[UniswapV4DonateEvent] = field(default_factory=list)
    uniswap_v4_protocol_fee_updates: List[UniswapV4FeeUpdatedEvent] = field(default_factory=list)
    uniswap_v4_dynamic_lp_fee_updates: List[UniswapV4DynamicLPFeeUpdatedEvent] = field(default_factory=list)
    uniswap_v4_protocol_fee_controller_updates: List[UniswapV4FeeControllerUpdatedEvent] = field(default_factory=list)
    uniswap_v4_balance_deltas: List[UniswapV4BalanceDeltaEvent] = field(default_factory=list)
    permit2_events: List[Permit2Event] = field(default_factory=list)

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

    @classmethod
    def _coerce_field(cls, parent: Optional[type], field_name: str, annotation: Any, value: Any) -> Any:
        if value is None:
            return None

        origin = get_origin(annotation)
        if origin in (list, List):
            (inner_type,) = get_args(annotation)
            return [cls._coerce_field(parent, field_name, inner_type, item) for item in value]
        if origin in (set, Set):
            (inner_type,) = get_args(annotation)
            return {cls._coerce_field(parent, field_name, inner_type, item) for item in value}
        if origin is Union:
            args = [arg for arg in get_args(annotation) if arg is not type(None)]
            if not args:
                return None
            return cls._coerce_field(parent, field_name, args[0], value)
        if annotation is int:
            return _ensure_int(value, field_name)
        if annotation is float:
            return float(value)
        if annotation is ChecksumAddress:
            return _coerce_checksum_address(value)
        if is_dataclass(annotation):
            if isinstance(value, annotation):
                return value
            if isinstance(value, dict):
                return cls._materialize_dataclass(annotation, value)
        return value

    @classmethod
    def _materialize_dataclass(cls, dataclass_type: Any, data: Dict[str, Any]) -> Any:
        if isinstance(data, dataclass_type):
            return data
        type_hints = get_type_hints(dataclass_type, globalns=globals())
        init_kwargs: Dict[str, Any] = {}
        for field_name, field_def in dataclass_type.__dataclass_fields__.items():  # type: ignore[attr-defined]
            annotation = type_hints.get(field_name, field_def.type)
            init_kwargs[field_name] = cls._coerce_field(dataclass_type, field_name, annotation, data.get(field_name))
        return dataclass_type(**init_kwargs)

    @classmethod
    def _coerce_sequence(cls, field_name: str, items: Optional[List[Any]], item_type: Any) -> List[Any]:
        if not items:
            return []
        coerced: List[Any] = []
        for item in items:
            if isinstance(item, item_type):
                coerced.append(item)
            elif item is None:
                continue
            elif is_dataclass(item_type) and isinstance(item, dict):
                coerced.append(cls._materialize_dataclass(item_type, item))
            else:
                coerced.append(cls._coerce_field(item_type if is_dataclass(item_type) else None, field_name, item_type, item))
        return coerced

    @classmethod
    def _normalize_address_set(cls, field_name: str, addresses: Optional[Union[Set[ChecksumAddress], List[ChecksumAddress]]]) -> Set[ChecksumAddress]:
        if not addresses:
            return set()
        normalized: Set[ChecksumAddress] = set()
        for address in addresses:
            addr = _coerce_checksum_address(address, allow_none=True)
            if addr:
                normalized.add(addr)
        return normalized

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
            "erc20_approval_events": self._list_to_dicts(self.erc20_approval_events),
            "erc721_approval_events": self._list_to_dicts(self.erc721_approval_events),
            "uniswap_v2_mints": self._list_to_dicts(self.uniswap_v2_mints),
            "uniswap_v2_burns": self._list_to_dicts(self.uniswap_v2_burns),
            "deposit_events": self._list_to_dicts(self.deposit_events),
            "withdraw_events": self._list_to_dicts(self.withdraw_events),
            "uniswap_v2_pair_created_events": self._list_to_dicts(self.uniswap_v2_pair_created_events),
            "ownership_transferred_events": self._list_to_dicts(self.ownership_transferred_events),
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
            "uniswap_v4_donates": self._list_to_dicts(self.uniswap_v4_donates),
            "uniswap_v4_protocol_fee_updates": self._list_to_dicts(self.uniswap_v4_protocol_fee_updates),
            "uniswap_v4_dynamic_lp_fee_updates": self._list_to_dicts(self.uniswap_v4_dynamic_lp_fee_updates),
            "uniswap_v4_protocol_fee_controller_updates": self._list_to_dicts(self.uniswap_v4_protocol_fee_controller_updates),
            "uniswap_v4_balance_deltas": self._list_to_dicts(self.uniswap_v4_balance_deltas),
            "permit2_events": self._list_to_dicts(self.permit2_events),
            "other_events": self.other_events,
            "address_balance_changes": self.address_balance_changes,
            "latest_states": self.latest_states,
            "input": self.input,
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), separators=(",", ":"))

    @property
    def value_eth(self) -> float:
        return self.value / 1e18

    @property
    def bribe_amount_eth(self) -> float:
        return self.bribe_amount / 1e18
    
    @classmethod
    def from_dict(cls, tx_dict: Dict[str, Any]) -> 'ProcessedTransaction':
        """
        Create ProcessedTransaction from a canonical dictionary.
        """
        fees_data = tx_dict.get('fees') or {}
        if isinstance(fees_data, TransactionFees):
            fees = fees_data
        elif fees_data:
            fees = cls._materialize_dataclass(TransactionFees, fees_data)
        else:
            fees = TransactionFees(gas_price=0, gas_used=0, tx_fee=0)

        address_balance_changes = tx_dict.get('address_balance_changes') or {}

        value_raw = tx_dict.get('value', 0)
        if value_raw is None:
            value_raw = 0
        bribe_raw = tx_dict.get('bribe_amount', 0)
        if bribe_raw is None:
            bribe_raw = 0

        return cls(
            hash=tx_dict['hash'],
            block_number=_ensure_int(tx_dict['block_number'], "block_number"),
            block_timestamp=_ensure_int(tx_dict.get('block_timestamp', tx_dict.get('timestamp', 0)), "block_timestamp"),
            tx_index=_ensure_int(tx_dict.get('tx_index', 0), "tx_index"),
            from_address=tx_dict['from_address'],
            to_address=tx_dict.get('to_address'),
            contract_address=tx_dict.get('contract_address'),
            value=_ensure_int(value_raw, "value"),
            status=_ensure_status_bool(tx_dict.get('status', True)),
            nonce=_ensure_int(tx_dict.get('nonce', 0), "nonce"),
            input=tx_dict.get('input', '0x'),
            tx_type=tx_dict.get('tx_type', 'unknown'),
            actions=list(tx_dict.get('actions') or []),
            eth_transfers=cls._coerce_sequence("eth_transfers", tx_dict.get('eth_transfers'), ETHTransfer),
            erc20_transfers=cls._coerce_sequence("erc20_transfers", tx_dict.get('erc20_transfers'), ERC20TransferEvent),
            erc721_transfers=cls._coerce_sequence("erc721_transfers", tx_dict.get('erc721_transfers'), ERC721TransferEvent),
            erc1155_transfers=cls._coerce_sequence("erc1155_transfers", tx_dict.get('erc1155_transfers'), ERC1155TransferEvent),
            internal_transactions=cls._coerce_sequence("internal_transactions", tx_dict.get('internal_transactions'), InternalTransaction),
            uniswap_v2_syncs=cls._coerce_sequence("uniswap_v2_syncs", tx_dict.get('uniswap_v2_syncs'), UniswapV2SyncEvent),
            uniswap_v2_swaps=cls._coerce_sequence("uniswap_v2_swaps", tx_dict.get('uniswap_v2_swaps'), UniswapV2SwapEvent),
            erc20_approval_events=cls._coerce_sequence("erc20_approval_events", tx_dict.get('erc20_approval_events'), ERC20ApprovalEvent),
            erc721_approval_events=cls._coerce_sequence("erc721_approval_events", tx_dict.get('erc721_approval_events'), ERC721ApprovalEvent),
            uniswap_v2_mints=cls._coerce_sequence("uniswap_v2_mints", tx_dict.get('uniswap_v2_mints'), UniswapV2MintEvent),
            uniswap_v2_burns=cls._coerce_sequence("uniswap_v2_burns", tx_dict.get('uniswap_v2_burns'), UniswapV2BurnEvent),
            deposit_events=cls._coerce_sequence("deposit_events", tx_dict.get('deposit_events'), DepositEvent),
            withdraw_events=cls._coerce_sequence("withdraw_events", tx_dict.get('withdraw_events'), WithdrawEvent),
            uniswap_v2_pair_created_events=cls._coerce_sequence("uniswap_v2_pair_created_events", tx_dict.get('uniswap_v2_pair_created_events'), UniswapV2PairCreatedEvent),
            ownership_transferred_events=cls._coerce_sequence("ownership_transferred_events", tx_dict.get('ownership_transferred_events'), OwnershipTransferredEvent),
            contract_creation_events=cls._coerce_sequence("contract_creation_events", tx_dict.get('contract_creation_events'), ContractCreationEvent),
            trading_enabled_events=cls._coerce_sequence("trading_enabled_events", tx_dict.get('trading_enabled_events'), TradingEnabledEvent),
            trading_disabled_events=cls._coerce_sequence("trading_disabled_events", tx_dict.get('trading_disabled_events'), TradingDisabledEvent),
            uniswap_v3_pools=cls._coerce_sequence("uniswap_v3_pools", tx_dict.get('uniswap_v3_pools'), UniswapV3PoolCreatedEvent),
            uniswap_v3_initializations=cls._coerce_sequence("uniswap_v3_initializations", tx_dict.get('uniswap_v3_initializations'), UniswapV3InitializeEvent),
            uniswap_v3_burns=cls._coerce_sequence("uniswap_v3_burns", tx_dict.get('uniswap_v3_burns'), UniswapV3BurnEvent),
            uniswap_v3_mints=cls._coerce_sequence("uniswap_v3_mints", tx_dict.get('uniswap_v3_mints'), UniswapV3MintEvent),
            uniswap_v3_swaps=cls._coerce_sequence("uniswap_v3_swaps", tx_dict.get('uniswap_v3_swaps'), UniswapV3SwapEvent),
            uniswap_v3_positions=cls._coerce_sequence("uniswap_v3_positions", tx_dict.get('uniswap_v3_positions'), UniswapV3PositionEvent),
            uniswap_v3_increases=cls._coerce_sequence("uniswap_v3_increases", tx_dict.get('uniswap_v3_increases'), UniswapV3IncreaseLiquidityEvent),
            uniswap_v3_decreases=cls._coerce_sequence("uniswap_v3_decreases", tx_dict.get('uniswap_v3_decreases'), UniswapV3DecreaseLiquidityEvent),
            uniswap_v4_initializes=cls._coerce_sequence("uniswap_v4_initializes", tx_dict.get('uniswap_v4_initializes'), UniswapV4InitializeEvent),
            uniswap_v4_modifies=cls._coerce_sequence("uniswap_v4_modifies", tx_dict.get('uniswap_v4_modifies'), UniswapV4ModifyLiquidityEvent),
            uniswap_v4_swaps=cls._coerce_sequence("uniswap_v4_swaps", tx_dict.get('uniswap_v4_swaps'), UniswapV4SwapEvent),
            uniswap_v4_donates=cls._coerce_sequence("uniswap_v4_donates", tx_dict.get('uniswap_v4_donates'), UniswapV4DonateEvent),
            uniswap_v4_protocol_fee_updates=cls._coerce_sequence("uniswap_v4_protocol_fee_updates", tx_dict.get('uniswap_v4_protocol_fee_updates'), UniswapV4FeeUpdatedEvent),
            uniswap_v4_dynamic_lp_fee_updates=cls._coerce_sequence("uniswap_v4_dynamic_lp_fee_updates", tx_dict.get('uniswap_v4_dynamic_lp_fee_updates'), UniswapV4DynamicLPFeeUpdatedEvent),
            uniswap_v4_protocol_fee_controller_updates=cls._coerce_sequence("uniswap_v4_protocol_fee_controller_updates", tx_dict.get('uniswap_v4_protocol_fee_controller_updates'), UniswapV4FeeControllerUpdatedEvent),
            uniswap_v4_balance_deltas=cls._coerce_sequence("uniswap_v4_balance_deltas", tx_dict.get('uniswap_v4_balance_deltas'), UniswapV4BalanceDeltaEvent),
            permit2_events=cls._coerce_sequence("permit2_events", tx_dict.get('permit2_events'), Permit2Event),
            other_events=list(tx_dict.get('other_events') or []),
            fees=fees,
            unique_addresses=cls._normalize_address_set("unique_addresses", tx_dict.get('unique_addresses')),
            erc20_contracts=cls._normalize_address_set("erc20_contracts", tx_dict.get('erc20_contracts')),
            erc721_contracts=cls._normalize_address_set("erc721_contracts", tx_dict.get('erc721_contracts')),
            erc1155_contracts=cls._normalize_address_set("erc1155_contracts", tx_dict.get('erc1155_contracts')),
            address_balance_changes=address_balance_changes,
            latest_states=tx_dict.get('latest_states') or {},
            bribe_amount=_ensure_int(bribe_raw, "bribe_amount")
        )
    
    def __init__(self,
                 hash: str,
                 block_number: int,
                 block_timestamp: int,
                 tx_index: int,
                 from_address: str,
                 to_address: Optional[str],
                 contract_address: Optional[str],
                 value: int,
                 status: str,
                 nonce: int,
                 input: str,
                 tx_type: str,
                 actions: Optional[List[str]] = None,
                 eth_transfers: Optional[List[ETHTransfer]] = None,
                 erc20_transfers: Optional[List[ERC20TransferEvent]] = None,
                 erc721_transfers: Optional[List[ERC721TransferEvent]] = None,
                 erc1155_transfers: Optional[List[ERC1155TransferEvent]] = None,
                 internal_transactions: Optional[List[InternalTransaction]] = None,
                 uniswap_v2_syncs: Optional[List[UniswapV2SyncEvent]] = None,
                 uniswap_v2_swaps: Optional[List[UniswapV2SwapEvent]] = None,
                 erc20_approval_events: Optional[List[ERC20ApprovalEvent]] = None,
                 erc721_approval_events: Optional[List[ERC721ApprovalEvent]] = None,
                 uniswap_v2_mints: Optional[List[UniswapV2MintEvent]] = None,
                 uniswap_v2_burns: Optional[List[UniswapV2BurnEvent]] = None,
                 deposit_events: Optional[List[DepositEvent]] = None,
                 withdraw_events: Optional[List[WithdrawEvent]] = None,
                 uniswap_v2_pair_created_events: Optional[List[UniswapV2PairCreatedEvent]] = None,
                 ownership_transferred_events: Optional[List[OwnershipTransferredEvent]] = None,
                 contract_creation_events: Optional[List[ContractCreationEvent]] = None,
                 trading_enabled_events: Optional[List[TradingEnabledEvent]] = None,
                 trading_disabled_events: Optional[List[TradingDisabledEvent]] = None,
                 uniswap_v3_pools: Optional[List[UniswapV3PoolCreatedEvent]] = None,
                 uniswap_v3_initializations: Optional[List[UniswapV3InitializeEvent]] = None,
                 uniswap_v3_burns: Optional[List[UniswapV3BurnEvent]] = None,
                 uniswap_v3_mints: Optional[List[UniswapV3MintEvent]] = None,
                 uniswap_v3_swaps: Optional[List[UniswapV3SwapEvent]] = None,
                 uniswap_v3_positions: Optional[List[UniswapV3PositionEvent]] = None,
                 uniswap_v3_increases: Optional[List[UniswapV3IncreaseLiquidityEvent]] = None,
                 uniswap_v3_decreases: Optional[List[UniswapV3DecreaseLiquidityEvent]] = None,
                 uniswap_v4_initializes: Optional[List[UniswapV4InitializeEvent]] = None,
                 uniswap_v4_modifies: Optional[List[UniswapV4ModifyLiquidityEvent]] = None,
                 uniswap_v4_swaps: Optional[List[UniswapV4SwapEvent]] = None,
                 uniswap_v4_donates: Optional[List[UniswapV4DonateEvent]] = None,
                 uniswap_v4_protocol_fee_updates: Optional[List[UniswapV4FeeUpdatedEvent]] = None,
                 uniswap_v4_dynamic_lp_fee_updates: Optional[List[UniswapV4DynamicLPFeeUpdatedEvent]] = None,
                 uniswap_v4_protocol_fee_controller_updates: Optional[List[UniswapV4FeeControllerUpdatedEvent]] = None,
                 uniswap_v4_balance_deltas: Optional[List[UniswapV4BalanceDeltaEvent]] = None,
                 permit2_events: Optional[List[Permit2Event]] = None,
                 other_events: Optional[List[Dict[str, Any]]] = None,
                 fees: Optional[TransactionFees] = None,
                 unique_addresses: Optional[Set[ChecksumAddress]] = None,
                 erc20_contracts: Optional[Set[ChecksumAddress]] = None,
                 erc721_contracts: Optional[Set[ChecksumAddress]] = None,
                 erc1155_contracts: Optional[Set[ChecksumAddress]] = None,
                 address_balance_changes: Optional[Dict[str, Any]] = None,
                 latest_states: Optional[Dict[str, Any]] = None,
                 bribe_amount: int = 0,
                 ):
        """Initialize DetailedTransaction with type conversion handling"""
        
        # Core transaction fields
        self.hash = _ensure_hex_str(hash, "hash")
        self.block_number = _ensure_int(block_number, "block_number")
        self.block_timestamp = _ensure_int(block_timestamp, "block_timestamp")
        self.tx_index = _ensure_int(tx_index, "tx_index")
        self.from_address = _coerce_checksum_address(from_address)
        self.to_address = _coerce_checksum_address(to_address, allow_none=True)
        self.value = _ensure_int(value, "value")
        self.contract_address = _coerce_checksum_address(contract_address, allow_none=True)
        self.status = _ensure_status_bool(status)
        self.nonce = _ensure_int(nonce, "nonce")
        self.input = _ensure_hex_str(input, "input")
        self.tx_type = tx_type

        # Lists initialization with empty defaults
        self.actions = list(actions or [])
        self.eth_transfers = list(eth_transfers or [])
        self.erc20_transfers = list(erc20_transfers or [])
        self.erc721_transfers = list(erc721_transfers or [])
        self.erc1155_transfers = list(erc1155_transfers or [])
        self.internal_transactions = list(internal_transactions or [])
        self.uniswap_v2_syncs = list(uniswap_v2_syncs or [])
        self.uniswap_v2_swaps = list(uniswap_v2_swaps or [])
        self.erc20_approval_events = list(erc20_approval_events or [])
        self.erc721_approval_events = list(erc721_approval_events or [])
        self.uniswap_v2_mints = list(uniswap_v2_mints or [])
        self.uniswap_v2_burns = list(uniswap_v2_burns or [])
        self.deposit_events = list(deposit_events or [])
        self.withdraw_events = list(withdraw_events or [])
        self.uniswap_v2_pair_created_events = list(uniswap_v2_pair_created_events or [])
        self.ownership_transferred_events = list(ownership_transferred_events or [])
        self.contract_creation_events = list(contract_creation_events or [])
        self.trading_enabled_events = list(trading_enabled_events or [])
        self.trading_disabled_events = list(trading_disabled_events or [])
        
        # Uniswap V3 specific fields
        self.uniswap_v3_pools = list(uniswap_v3_pools or [])
        self.uniswap_v3_initializations = list(uniswap_v3_initializations or [])
        self.uniswap_v3_mints = list(uniswap_v3_mints or [])
        self.uniswap_v3_swaps = list(uniswap_v3_swaps or [])
        self.uniswap_v3_positions = list(uniswap_v3_positions or [])
        self.uniswap_v3_burns = list(uniswap_v3_burns or [])
        self.uniswap_v3_increases = list(uniswap_v3_increases or [])
        self.uniswap_v3_decreases = list(uniswap_v3_decreases or [])
        self.other_events = list(other_events or [])

        # Uniswap V4 specific fields
        self.uniswap_v4_initializes = list(uniswap_v4_initializes or [])
        self.uniswap_v4_modifies = list(uniswap_v4_modifies or [])
        self.uniswap_v4_swaps = list(uniswap_v4_swaps or [])
        self.uniswap_v4_donates = list(uniswap_v4_donates or [])
        self.uniswap_v4_protocol_fee_updates = list(uniswap_v4_protocol_fee_updates or [])
        self.uniswap_v4_dynamic_lp_fee_updates = list(uniswap_v4_dynamic_lp_fee_updates or [])
        self.uniswap_v4_protocol_fee_controller_updates = list(uniswap_v4_protocol_fee_controller_updates or [])
        self.uniswap_v4_balance_deltas = list(uniswap_v4_balance_deltas or [])
        self.permit2_events = list(permit2_events or [])

        # Complex fields
        self.fees = fees or TransactionFees(gas_price=0, gas_used=0, tx_fee=0)
        self.unique_addresses = self._normalize_address_set("unique_addresses", unique_addresses)
        self.erc20_contracts = self._normalize_address_set("erc20_contracts", erc20_contracts)
        self.erc721_contracts = self._normalize_address_set("erc721_contracts", erc721_contracts)
        self.erc1155_contracts = self._normalize_address_set("erc1155_contracts", erc1155_contracts)
        self.address_balance_changes = dict(address_balance_changes or {})
        self.latest_states = dict(latest_states or {})
        self.bribe_amount = _ensure_int(bribe_amount if bribe_amount is not None else 0, "bribe_amount")

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
            self.erc20_approval_events == other.erc20_approval_events and
            self.erc721_approval_events == other.erc721_approval_events and
            self.uniswap_v2_mints == other.uniswap_v2_mints and
            self.uniswap_v2_burns == other.uniswap_v2_burns and
            self.deposit_events == other.deposit_events and
            self.withdraw_events == other.withdraw_events and
            self.uniswap_v2_pair_created_events == other.uniswap_v2_pair_created_events and
            self.ownership_transferred_events == other.ownership_transferred_events and
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
            self.uniswap_v4_donates == other.uniswap_v4_donates and
            self.uniswap_v4_protocol_fee_updates == other.uniswap_v4_protocol_fee_updates and
            self.uniswap_v4_dynamic_lp_fee_updates == other.uniswap_v4_dynamic_lp_fee_updates and
            self.uniswap_v4_protocol_fee_controller_updates == other.uniswap_v4_protocol_fee_controller_updates and
            self.uniswap_v4_balance_deltas == other.uniswap_v4_balance_deltas and
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
