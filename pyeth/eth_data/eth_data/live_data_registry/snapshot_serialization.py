"""
Snapshot builders shared by the live data registry.
"""
from typing import Any, Dict, Mapping, MutableMapping, Optional, Union

import orjson

from eth_data.blockchain.block_data_models import BlockHeader, ProcessedBlockResult

JsonLike = Union[str, Mapping[str, Any], BlockHeader]


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
    resolved_number = (
        block_number
        or (int(header_dict.get("number", "0"), 16) if header_dict.get("number") else None)
    )
    snapshot: Dict[str, Any] = {
        "block_number": resolved_number,
        "header": header_dict,
        "tx_count": len(processed_block.transactions),
    }
    if include_transactions:
        snapshot["transactions"] = processed_block.transactions
    return snapshot


def dumps_snapshot(snapshot: Mapping[str, Any]) -> str:
    """
    Serialize a snapshot dict to JSON using orjson.
    """
    return orjson.dumps(snapshot).decode()


def loads_snapshot(payload: Optional[str]) -> Optional[Dict[str, Any]]:
    if not payload:
        return None
    return orjson.loads(payload)


__all__ = [
    "build_block_snapshot",
    "normalize_block_header",
    "dumps_snapshot",
    "loads_snapshot",
]
