"""Serialization helpers for Python-owned live snapshots."""
import dataclasses
from typing import Any, Dict, Mapping, Optional

import orjson
from hexbytes import HexBytes

MAX_I64 = 2**63 - 1


def normalize_block_header(header: Any) -> Dict[str, Any]:
    """
    Convert the block header representation into a plain dict.
    """
    if header is None:
        return {}
    if hasattr(header, "to_rpc_dict"):
        return header.to_rpc_dict()
    if isinstance(header, str):
        try:
            return orjson.loads(header)
        except orjson.JSONDecodeError:
            return {"raw": header}
    if isinstance(header, Mapping):
        return dict(header)
    return _block_header_from_processed_block(header)


def json_safe(value: Any) -> Any:
    """
    Recursively convert values to JSON-serializable structures.
    """
    if dataclasses.is_dataclass(value):
        value = dataclasses.asdict(value)

    if hasattr(value, "to_dict"):
        value = value.to_dict()

    if isinstance(value, dict):
        return {k: json_safe(v) for k, v in value.items()}

    if isinstance(value, (list, tuple)):
        return [json_safe(v) for v in value]

    if isinstance(value, set):
        return [json_safe(v) for v in value]

    if isinstance(value, (bytes, bytearray, memoryview, HexBytes)):
        return bytes(value).hex()

    if isinstance(value, int) and (value > MAX_I64 or value < -MAX_I64 - 1):
        return str(value)

    return value


def _block_header_from_processed_block(block: Any) -> Dict[str, Any]:
    if block is None:
        return {}
    return {
        "hash": _format_optional_hex(_get_field(block, "hash")),
        "parentHash": _format_optional_hex(_get_field(block, "parent_hash")),
        "number": _format_optional_hex(_get_field(block, "number")),
        "gasLimit": _format_optional_hex(_get_field(block, "gas_limit")),
        "gasUsed": _format_optional_hex(_get_field(block, "gas_used")),
        "timestamp": _format_optional_hex(_get_field(block, "timestamp")),
        "baseFeePerGas": _format_optional_hex(_get_field(block, "base_fee_per_gas")),
    }


def _get_field(value: Any, field: str, default: Any = None) -> Any:
    if isinstance(value, Mapping):
        return value.get(field, default)
    return getattr(value, field, default)


def _format_optional_hex(value: Any) -> Optional[str]:
    if value is None:
        return None
    if isinstance(value, str):
        stripped = value.strip()
        if not stripped:
            return None
        if stripped.startswith(("0x", "0X")):
            return stripped
        if stripped.isdecimal():
            return hex(int(stripped))
        return f"0x{stripped}"
    if isinstance(value, (bytes, bytearray, memoryview, HexBytes)):
        return f"0x{bytes(value).hex()}"
    if isinstance(value, int):
        return hex(value)
    return str(value)


__all__ = [
    "json_safe",
    "normalize_block_header",
]
