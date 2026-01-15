"""Common address helpers sourced from PyReth."""
from typing import Optional
from web3 import Web3
import pyreth

ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"
POOL_FACTORIES = dict(pyreth.pool_factories())
ROUTERS = dict(pyreth.routers())
ROUTER_ADDRESSES = set(ROUTERS.values())


def get_pool_protocol(factory_address: str) -> Optional[str]:
    """Return the protocol label for a known factory address."""
    return pyreth.get_pool_protocol(factory_address)


def is_v4_pool_manager(address: str) -> bool:
    return pyreth.is_uniswap_v4_pool_manager(address)


def is_known_factory(address: str) -> bool:
    return pyreth.is_known_factory(address)

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

def _pair_dict(info, include_fee=False):
    data = {
        "token": info.symbol,
        "token_address": info.token_address,
        "token_decimals": info.decimals,
        "denom": info.denom_symbol,
        "denom_address": info.denom_address,
        "denom_decimals": info.denom_decimals,
    }
    if include_fee:
        data["fee"] = info.fee_tier
    return data

UNISWAP_V2_PAIRS = [_pair_dict(info) for info in pyreth.uniswap_v2_pairs()]
UNISWAP_V2_PAIR_LOOKUP = {(entry["token"], entry["denom"]): entry for entry in UNISWAP_V2_PAIRS}
UNISWAP_V3_PAIRS = [_pair_dict(info, include_fee=True) for info in pyreth.uniswap_v3_pairs()]
UNISWAP_V3_PAIR_LOOKUP = {(entry["token"], entry["denom"], entry["fee"]): entry for entry in UNISWAP_V3_PAIRS}
SUSHISWAP_PAIRS = [_pair_dict(info) for info in pyreth.sushiswap_pairs()]
SUSHISWAP_PAIR_LOOKUP = {(entry["token"], entry["denom"]): entry for entry in SUSHISWAP_PAIRS}

fee_recipients = pyreth.fee_recipients()
fee_recipients_set = set(fee_recipients.keys())

denominator_addresses_by_name = {
    key: addresses_by_name[key]
    for key in ("WETH", "USDC", "USDT", "DAI")
    if key in addresses_by_name
}

denominator_names_by_address = {v: k for k, v in denominator_addresses_by_name.items()}
denominator_byte_addresses_by_name = {key: byte_addresses_by_name[key] for key in denominator_addresses_by_name}
denominator_names_by_byte_address = {v: k for k, v in denominator_byte_addresses_by_name.items()}


__all__ = [
    "ZERO_ADDRESS",
    "POOL_FACTORIES",
    "ROUTERS",
    "ROUTER_ADDRESSES",
    "get_pool_protocol",
    "is_v4_pool_manager",
    "is_known_factory",
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
    "UNISWAP_V2_PAIRS",
    "UNISWAP_V2_PAIR_LOOKUP",
    "UNISWAP_V3_PAIRS",
    "UNISWAP_V3_PAIR_LOOKUP",
    "SUSHISWAP_PAIRS",
    "SUSHISWAP_PAIR_LOOKUP",
    "denominator_addresses_by_name",
    "denominator_names_by_address",
    "denominator_byte_addresses_by_name",
    "denominator_names_by_byte_address",
    "fee_recipients",
    "fee_recipients_set",
    "alleged_mr_beast_wallet",
    "sandwich_attackers",
]
