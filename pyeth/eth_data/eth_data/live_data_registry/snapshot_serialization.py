"""
Snapshot builders shared by the live data registry.
"""
import dataclasses
from typing import Any, Dict, Mapping, MutableMapping, Optional, Union

import orjson
from hexbytes import HexBytes

from eth_data.blockchain.block_data_models import BlockHeader, ProcessedBlockResult

JsonLike = Union[str, Mapping[str, Any], BlockHeader]
MAX_I64 = 2**63 - 1


def normalize_block_header(header: JsonLike) -> Dict[str, Any]:
    """
    Convert the block header representation into a plain dict.
    """
    if header is None:
        return {}
    if isinstance(header, BlockHeader):
        return header.to_rpc_dict()
    if isinstance(header, str):
        try:
            return orjson.loads(header)
        except orjson.JSONDecodeError:
            return {"raw": header}
    if isinstance(header, Mapping):
        return dict(header)
    raise TypeError(f"Unsupported block header type: {type(header)!r}")


def build_block_snapshot(
    processed_block: ProcessedBlockResult,
    *,
    block_number: Optional[int] = None,
    include_transactions: bool = False,
) -> Dict[str, Any]:
    """
    Build a JSON-friendly dict for a processed block.
    """
    header_dict = normalize_block_header(processed_block.block_header)
    header_json = orjson.dumps(header_dict).decode()
    resolved_number = (
        block_number
        or (int(header_dict.get("number", "0"), 16) if header_dict.get("number") else None)
    )
    snapshot: Dict[str, Any] = {
        "block_number": resolved_number,
        "header": header_json,
        "tx_count": len(processed_block.transactions),
    }
    if include_transactions:
        snapshot["transactions"] = [json_safe(tx) for tx in processed_block.transactions]
    return snapshot


def dumps_snapshot(snapshot: Mapping[str, Any]) -> str:
    """
    Serialize a snapshot dict to JSON using orjson.
    """
    return orjson.dumps(json_safe(snapshot)).decode()


def loads_snapshot(payload: Optional[str]) -> Optional[Dict[str, Any]]:
    if not payload:
        return None
    return orjson.loads(payload)


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


__all__ = [
    "build_block_snapshot",
    "json_safe",
    "normalize_block_header",
    "dumps_snapshot",
    "loads_snapshot",
]
