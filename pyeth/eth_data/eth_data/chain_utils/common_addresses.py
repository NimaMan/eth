"""Common address helpers sourced from PyReth."""
from typing import Optional
from web3 import Web3
import pyreth
from .pool_addresses import POOL_FACTORIES, ROUTERS

ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"
ROUTER_ADDRESSES = set(ROUTERS.values())

DEX_POOL_TYPES = tuple(pyreth.dex_pool_types())
DEX_POOL_TYPE_SET = set(DEX_POOL_TYPES)


def canonicalize_dex_pool_type(name: Optional[str]) -> Optional[str]:
    """Return the canonical DEX pool type name defined in DEX_POOL_TYPES."""
    if not isinstance(name, str):
        return name

    stripped = name.strip()
    if not stripped:
        return stripped

    canonical = pyreth.canonicalize_dex_pool_type(stripped)
    if canonical:
        return canonical
    return stripped


# ---------------------------------------------------------------------------
# Shared datasets sourced from PyReth
# ---------------------------------------------------------------------------

_ADDRESS_BOOK = pyreth.address_book()
addresses_by_name = dict(_ADDRESS_BOOK)
names_by_address = {address: name for name, address in addresses_by_name.items()}

byte_addresses_by_name = {
    label: Web3.to_bytes(hexstr=addr) for label, addr in addresses_by_name.items()
}
names_by_byte_addresses = {v: k for k, v in byte_addresses_by_name.items()}

DENOM_ADDRESSES = dict(pyreth.denom_address_map())
DENOM_NAMES_TO_ADDRESS = {symbol: addr for addr, symbol in DENOM_ADDRESSES.items()}
ERC20_TOKEN_DECIMALS = dict(pyreth.token_decimals())

_stablecoin_infos = pyreth.stablecoin_infos()
STABLECOIN_UNIT_BY_NAME = {info.symbol: info.unit for info in _stablecoin_infos}
STABLECOINS_ADDRESS_BY_NAME = {info.symbol: info.address for info in _stablecoin_infos}
STABLECOINS_NAME_BY_ADDRESS = {info.address: info.symbol for info in _stablecoin_infos}

CEX_ADDRESSES_BY_NAME = dict(pyreth.cex_address_map())
CEX_NAMES_BY_ADDRESS = {addr: name for name, addr in CEX_ADDRESSES_BY_NAME.items()}
CEX_ADDRESSES_SET = set(CEX_NAMES_BY_ADDRESS.keys())

ETF_ADDRESSES_BY_NAME = dict(pyreth.etf_address_map())
ETF_NAMES_BY_ADDRESS = {addr: name for name, addr in ETF_ADDRESSES_BY_NAME.items()}
ETF_ADDRESS_SET = set(ETF_ADDRESSES_BY_NAME.values())

fee_recipients = pyreth.fee_recipients()
fee_recipients_set = set(fee_recipients.keys())

denominator_addresses_by_name = {
    key: addresses_by_name[key]
    for key in ("WETH", "USDC", "USDT", "DAI")
    if key in addresses_by_name
}
denominator_names_by_address = {v: k for k, v in denominator_addresses_by_name.items()}
denominator_byte_addresses_by_name = {
    key: byte_addresses_by_name[key] for key in denominator_addresses_by_name
}
denominator_names_by_byte_address = {
    v: k for k, v in denominator_byte_addresses_by_name.items()
}

alleged_mr_beast_wallet = [
    "0x9e67D018488aD636B538e4158E9e7577F2ECac12",
    "0x3640f50C46632E03F2677f85Ec0372a8Dd70b8f4",
    "0xED3F5d401a270416e5008ce35E07Eb0721D6f8B4",
    "0x949cC70bAa140f5b55717ca938E3c7e4C3b3A016",
    "0xb5bf6777e3524aD0ffCC5a37375cc49a4BE92F64",
    "0x2c071Af9dCeFB7155659B662480CbB8679977394",
    "0x4f7B657a2cAe7A8808Df1D889838d5Da33007ae8",
]

sandwich_attackers = [
    "0xae2Fc483527B8EF99EB5D9B44875F005ba1FaE13",
]

__all__ = [
    "ZERO_ADDRESS",
    "ROUTER_ADDRESSES",
    "DEX_POOL_TYPES",
    "DEX_POOL_TYPE_SET",
    "canonicalize_dex_pool_type",
    "addresses_by_name",
    "names_by_address",
    "byte_addresses_by_name",
    "names_by_byte_addresses",
    "DENOM_ADDRESSES",
    "DENOM_NAMES_TO_ADDRESS",
    "ERC20_TOKEN_DECIMALS",
    "STABLECOIN_UNIT_BY_NAME",
    "STABLECOINS_ADDRESS_BY_NAME",
    "STABLECOINS_NAME_BY_ADDRESS",
    "CEX_ADDRESSES_BY_NAME",
    "CEX_NAMES_BY_ADDRESS",
    "CEX_ADDRESSES_SET",
    "ETF_ADDRESSES_BY_NAME",
    "ETF_NAMES_BY_ADDRESS",
    "ETF_ADDRESS_SET",
    "denominator_addresses_by_name",
    "denominator_names_by_address",
    "denominator_byte_addresses_by_name",
    "denominator_names_by_byte_address",
    "fee_recipients",
    "fee_recipients_set",
    "alleged_mr_beast_wallet",
    "sandwich_attackers",
]
