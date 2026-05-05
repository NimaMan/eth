"""Numeric coercion helpers for Redis/RPC event payloads."""

from typing import Any


def parse_raw_int(value: Any, default: int = 0) -> int:
    if value is None:
        return default
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, (bytes, bytearray)):
        return int.from_bytes(value, byteorder="big")
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return default
        lowered = cleaned.lower()
        if lowered.startswith("-0x"):
            return -int(cleaned[3:], 16)
        if lowered.startswith("0x"):
            return int(cleaned, 16)
        return int(cleaned)
    return int(value)


def parse_raw_float(value: Any, default: float = 0.0) -> float:
    if value is None:
        return default
    if isinstance(value, str):
        cleaned = value.strip()
        if not cleaned:
            return default
        lowered = cleaned.lower()
        if lowered.startswith("0x") or lowered.startswith("-0x"):
            return float(parse_raw_int(cleaned))
        return float(cleaned)
    return float(value)
