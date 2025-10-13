"""
Type Converter Utility

Objective:
---------
Provide consistent type conversion for blockchain data fields, ensuring proper formatting
and type safety across the application.

Key Functions:
------------
1. Hex string to int conversion
2. Wei to Ether conversion
3. Bytes to hex string conversion
4. Standardized address formatting
"""

from typing import Any, Union
from decimal import Decimal
from hexbytes import HexBytes
from web3 import Web3


def convert_to_int(value: Union[str, int, HexBytes]) -> int:
    """Convert hex strings, bytes or ints to integer"""
    if isinstance(value, str):
        return int(value, 16) if value.startswith('0x') else int(value)
    elif isinstance(value, HexBytes):
        return int.from_bytes(value, byteorder='big')
    elif isinstance(value, int):
        return value
    raise ValueError(f"Cannot convert {type(value)} to int")


def convert_to_decimal(value: Union[str, int, HexBytes]) -> Decimal:
    """Convert value to Decimal, handling wei conversions"""
    if isinstance(value, (str, HexBytes)):
        value = convert_to_int(value)
    return Decimal(value)


def convert_to_hex_str(value: Union[str, bytes, HexBytes, int]) -> str:
    """Convert value to '0x' prefixed hex string"""
    if isinstance(value, str):
        return value if value.startswith('0x') else f"0x{value}"
    elif isinstance(value, (bytes, HexBytes)):
        return f"0x{value.hex()}"
    elif isinstance(value, int):
        return hex(value)
    raise ValueError(f"Cannot convert {type(value)} to hex string")


def normalize_address(address: Union[str, bytes, HexBytes]) -> str:
    """Convert address to checksum format"""
    if isinstance(address, (bytes, HexBytes)):
        address = f"0x{address.hex()}"
    return Web3.to_checksum_address(address)


def convert_log_index(value: Union[str, int, HexBytes]) -> int:
    """Convert log index to integer"""
    return convert_to_int(value)


def convert_block_number(value: Union[str, int, HexBytes]) -> int:
    """Convert block number to integer"""
    return convert_to_int(value)


def convert_transaction_index(value: Union[str, int, HexBytes]) -> int:
    """Convert transaction index to integer"""
    return convert_to_int(value)


def convert_status(value: Union[str, int, HexBytes]) -> bool:
    """Convert transaction status to boolean"""
    status_int = convert_to_int(value)
    return bool(status_int)


def convert_scaled_amount(value: Union[str, int, HexBytes], decimals: int | None) -> float:
    """Convert a raw token amount to a float scaled by decimals."""
    amount_int = convert_to_int(value)
    if decimals is None:
        return float(amount_int)
    return amount_int / (10 ** decimals)
