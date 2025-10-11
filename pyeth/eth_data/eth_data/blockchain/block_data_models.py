"""
Shared block-level data models used across the block processing pipeline.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Optional, Union

import orjson
from hexbytes import HexBytes

__all__ = ["BlockHeader", "ProcessedBlockResult"]

_FIELD_NAME_TO_RPC_KEY = {
    "hash": "hash",
    "parent_hash": "parentHash",
    "ommers_hash": "sha3Uncles",
    "beneficiary": "miner",
    "state_root": "stateRoot",
    "transactions_root": "transactionsRoot",
    "receipts_root": "receiptsRoot",
    "logs_bloom": "logsBloom",
    "difficulty": "difficulty",
    "number": "number",
    "gas_limit": "gasLimit",
    "gas_used": "gasUsed",
    "timestamp": "timestamp",
    "extra_data": "extraData",
    "mix_hash": "mixHash",
    "nonce": "nonce",
    "base_fee_per_gas": "baseFeePerGas",
    "withdrawals_root": "withdrawalsRoot",
    "blob_gas_used": "blobGasUsed",
    "excess_blob_gas": "excessBlobGas",
    "parent_beacon_block_root": "parentBeaconBlockRoot",
    "requests_hash": "requestsHash",
}


@dataclass(frozen=True)
class BlockHeader:
    """Python representation of a Reth `SealedHeader`."""

    hash: Optional[str] = None
    parent_hash: Optional[str] = None
    ommers_hash: Optional[str] = None
    beneficiary: Optional[str] = None
    state_root: Optional[str] = None
    transactions_root: Optional[str] = None
    receipts_root: Optional[str] = None
    logs_bloom: Optional[str] = None
    difficulty: Optional[str] = None
    number: Optional[str] = None
    gas_limit: Optional[str] = None
    gas_used: Optional[str] = None
    timestamp: Optional[str] = None
    extra_data: Optional[str] = None
    mix_hash: Optional[str] = None
    nonce: Optional[str] = None
    base_fee_per_gas: Optional[str] = None
    withdrawals_root: Optional[str] = None
    blob_gas_used: Optional[str] = None
    excess_blob_gas: Optional[str] = None
    parent_beacon_block_root: Optional[str] = None
    requests_hash: Optional[str] = None

    @classmethod
    def from_rpc_dict(
        cls,
        header: Dict[str, Any],
        *,
        value_formatter=None,
    ) -> "BlockHeader":
        """
        Build a BlockHeader from an RPC-style dictionary (camelCase keys).

        Args:
            header: Dictionary returned by web3 for a block header.
            value_formatter: Optional callable applied to each raw value
                (useful for enforcing hex formatting).
        """
        formatter = value_formatter or cls.format_hex
        kwargs = {}
        for field_name, rpc_key in _FIELD_NAME_TO_RPC_KEY.items():
            kwargs[field_name] = formatter(header.get(rpc_key))

        return cls(**kwargs)

    def to_rpc_dict(self) -> Dict[str, Optional[str]]:
        """Return camelCase representation compatible with existing pipelines."""
        return {
            rpc_key: getattr(self, field_name)
            for field_name, rpc_key in _FIELD_NAME_TO_RPC_KEY.items()
        }

    @classmethod
    def format_hex(cls, value: Any) -> Optional[str]:
        if value is None:
            return None
        if isinstance(value, HexBytes):
            hex_value = value.hex()
            return hex_value if hex_value.startswith("0x") else f"0x{hex_value}"
        if isinstance(value, bytes):
            return f"0x{value.hex()}"
        if isinstance(value, int):
            return cls._ensure_even_hex_digits(hex(value))
        if isinstance(value, str):
            if value.startswith(("0x", "0X")):
                return cls._ensure_even_hex_digits(value)
            return value
        return str(value)

    @staticmethod
    def _ensure_even_hex_digits(value: str) -> str:
        prefix = ""
        digits = value
        if value.startswith(("0x", "0X")):
            prefix = value[:2]
            digits = value[2:]
        if len(digits) % 2 == 1:
            digits = f"0{digits}"
        if prefix == "":
            prefix = "0x"
        return f"{prefix}{digits}"


@dataclass
class ProcessedBlockResult:
    """
    Bundle of processed transactions and the serialized block header JSON.

    The `block_header` field is always a JSON string (or ``None``) to simplify
    downstream serialization for messaging and caching layers.
    """

    transactions: List[Any]
    block_header: Optional[Union[str, BlockHeader]] = None

    def __post_init__(self) -> None:
        if self.block_header is None or isinstance(self.block_header, str):
            return
        if isinstance(self.block_header, BlockHeader):
            self.block_header = orjson.dumps(
                self.block_header.to_rpc_dict(),
                option=orjson.OPT_SORT_KEYS,
            ).decode()
        else:
            raise TypeError(
                f"Unsupported block_header type: {type(self.block_header)!r}"
            )

    def __iter__(self) -> Iterable[Any]:
        return iter(self.transactions)

    def __len__(self) -> int:
        return len(self.transactions)

    def __getitem__(self, item: int) -> Any:
        return self.transactions[item]

    def __bool__(self) -> bool:
        return bool(self.transactions)
