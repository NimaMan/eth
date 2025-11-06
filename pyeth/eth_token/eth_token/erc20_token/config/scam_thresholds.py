"""
Scam Detection Thresholds Configuration

This module defines the liquidity thresholds below which pools are considered scams.
Different thresholds apply to different denomination currencies based on their value.
"""

from typing import Dict, Optional

from eth_data.chain_utils.common_addresses import DENOM_NAMES_TO_ADDRESS

# Thresholds for different asset categories
THRESHOLDS = {
    # Native/Wrapped ETH (assuming ETH = $2000)
    "ETH": {
        "threshold": 0.05,
        "unit": "ETH",
        "tokens": ["WETH", "cETH", "stETH", "cbETH", "rETH"],
    },
    # Major USD stablecoins
    "USD_MAJOR": {
        "threshold": 200,  # $200
        "unit": "USD",
        "tokens": ["USDC", "USDT", "DAI", "BUSD", "FRAX", "LUSD", "TUSD", "USDP", "GUSD", "PYUSD"],
    },
    # Secondary USD stablecoins
    "USD_SECONDARY": {
        "threshold": 200,  # $200
        "unit": "USD",
        "tokens": [
            "FDUSD",
            "FEI",
            "GHO",
            "MIM",
            "MKUSD",
            "OUSD",
            "sUSD",
            "USD0",
            "USD1",
            "USDS",
            "USDD",
            "USDE",
            "XUSD",
            "ZUSD",
            "DOLA",
        ],
    },
    # Euro stablecoins
    "EUR": {
        "threshold": 200,  # €200
        "unit": "EUR",
        "tokens": ["EUROC", "EURCV", "EURS", "EURT"],
    },
    # Other fiat stablecoins
    "JPY": {
        "threshold": 30000,  # ¥30,000 (~$200)
        "unit": "JPY",
        "tokens": ["GYEN", "JPYC"],
    },
    "SGD": {
        "threshold": 270,  # S$270 (~$200)
        "unit": "SGD",
        "tokens": ["XSGD"],
    },
    "CAD": {
        "threshold": 270,  # C$270 (~$200)
        "unit": "CAD",
        "tokens": ["CADC"],
    },
    "BRL": {
        "threshold": 1000,  # R$1000 (~$200)
        "unit": "BRL",
        "tokens": ["BRZ"],
    },
    "IDR": {
        "threshold": 3000000,  # Rp3,000,000 (~$200)
        "unit": "IDR",
        "tokens": ["IDRT", "XIDR"],
    },
    # Commodity-backed tokens (assuming gold = $2000/oz)
    "GOLD": {
        "threshold": 0.1,  # 0.1 troy ounce (~$200)
        "unit": "oz",
        "tokens": ["PAXG", "XAUt"],
    },
    # Wrapped BTC (assuming BTC = $50k)
    "BTC": {
        "threshold": 0.004,  # 0.004 BTC (~$200)
        "unit": "BTC",
        "tokens": ["WBTC"],
    },
    # Algorithmic/special stablecoins
    "SPECIAL": {
        "threshold": 200,  # $200 equivalent
        "unit": "units",
        "tokens": ["RAI"],  # RAI is not pegged to $1
    },
}

# Create reverse mappings from token symbol or address to category
TOKEN_TO_CATEGORY: Dict[str, str] = {}
TOKEN_ADDRESS_TO_CATEGORY: Dict[str, str] = {}

for category, config in THRESHOLDS.items():
    for token in config["tokens"]:
        TOKEN_TO_CATEGORY[token.upper()] = category

        address = DENOM_NAMES_TO_ADDRESS.get(token)
        if address:
            TOKEN_ADDRESS_TO_CATEGORY[address] = category


def get_threshold_for_token(token_identifier: str) -> Optional[Dict[str, any]]:
    """
    Get the scam threshold configuration for a specific token.

    Args:
        token_identifier: Token symbol (e.g., "USDC") or contract address.

    Returns:
        Dict with 'threshold', 'unit', and 'category' or None if not found.
    """
    if not token_identifier:
        return None

    identifier = token_identifier.strip()
    if not identifier:
        return None

    # Try symbol lookup first
    category = TOKEN_TO_CATEGORY.get(identifier.upper())

    # Fall back to address lookup
    if not category:
        category = TOKEN_ADDRESS_TO_CATEGORY.get(identifier)

    if category:
        config = THRESHOLDS[category].copy()
        config["category"] = category
        return config

    return None
