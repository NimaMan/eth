"""Checksum-address helpers for pool and token tracking code."""

from typing import Any, Iterable, Optional, Set

from web3 import Web3


def checksum_address(value: Any) -> Optional[str]:
    """Return a checksum address, or None for empty/invalid values."""
    if value is None:
        return None
    try:
        return Web3.to_checksum_address(value)
    except (TypeError, ValueError):
        return None


def require_checksum_address(value: Any) -> str:
    """Return a checksum address and raise if the value is invalid."""
    address = checksum_address(value)
    if address is None:
        raise ValueError(f"Invalid Ethereum address: {value!r}")
    return address


def same_address(left: Any, right: Any) -> bool:
    """Compare two Ethereum addresses after checksum normalization."""
    left_address = checksum_address(left)
    right_address = checksum_address(right)
    return left_address is not None and left_address == right_address


def checksum_address_set(values: Iterable[Any]) -> Set[str]:
    """Return valid checksum addresses from an iterable."""
    return {
        address
        for value in values
        if (address := checksum_address(value)) is not None
    }
