"""Quick snapshot of canonical token/WETH pools via PoolChainDataFetcher.

The script exercises the PyReth-backed helpers exposed by
`PoolChainDataFetcher` and prints raw liquidity data for a handful of
curated pools (default: USDC/WETH across Uniswap v2 + v3 tiers). Requires a
local Reth database configured via PyReth (``PYRETH_DATADIR``) so the
fetcher can answer without touching RPC.

Usage examples
--------------
python -m eth_token.scripts.pools.token_liquidity_snapshot
python -m eth_token.scripts.pools.token_liquidity_snapshot --token usdc --block 21312345
"""

from __future__ import annotations

import argparse
from pprint import pprint
from typing import Any, Dict, Iterable, Optional, Tuple
from eth_token.erc20_token.pools.pool_chain_data_fetcher import PoolChainDataFetcher


# Canonical mainnet pools keyed by quote token.
TOKEN_POOLS: Dict[str, Dict[str, Tuple[str, str, Optional[int]]]] = {
    "usdc": {
        "uniswap_v2_usdc_weth": ("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc", "v2", None),
        "uniswap_v3_usdc_weth_5bps": ("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", "v3", 500),
        "uniswap_v3_usdc_weth_30bps": ("0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8", "v3", 3000),
    },
    "usdt": {
        "uniswap_v2_usdt_weth": ("0x06DA0fd433C1A5d7a4faa01111c044910A184553", "v2", None),
        "uniswap_v3_usdt_weth_5bps": ("0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36", "v3", 500),
        "uniswap_v3_usdt_weth_30bps": ("0x3416cF6C708Da44DB2624D63ea0AAef7113527C6", "v3", 3000),
    },
    "dai": {
        "uniswap_v2_dai_weth": ("0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", "v2", None),
        "uniswap_v3_dai_weth_5bps": ("0x60594a405d53811d3BC4766596EFD80fd545A270", "v3", 500),
        "uniswap_v3_dai_weth_30bps": ("0xC2e9f25Be6257c210d7Adf0d4cd6E3E881ba25f8", "v3", 3000),
    },
}


def _fetch_liquidity(
    fetcher: PoolChainDataFetcher,
    label: str,
    pool_address: str,
    kind: str,
    fee_tier: Optional[int],
    block: Optional[int],
) -> Tuple[str, Optional[Dict[str, Any]]]:
    """Dispatch to the appropriate PyReth-backed helper."""
    if kind == "v2":
        return label, fetcher.get_v2_liquidity(pool_address, block)
    if kind == "v3":
        if fee_tier is None:
            raise ValueError(f"Pool '{label}' missing fee tier for V3 liquidity fetch")
        return label, fetcher.get_v3_liquidity(pool_address, fee_tier, block)
    raise ValueError(f"Unsupported pool kind '{kind}' for '{label}'")


def run_snapshot(
    token: str,
    block: Optional[int],
) -> Iterable[Tuple[str, Optional[Dict[str, Any]]]]:
    fetcher = PoolChainDataFetcher()
    pools = TOKEN_POOLS[token]
    for label, (pool_address, kind, fee_tier) in pools.items():
        yield _fetch_liquidity(fetcher, label, pool_address, kind, fee_tier, block)


def main() -> None:
    parser = argparse.ArgumentParser(description="Inspect token/WETH liquidity via PyReth")
    parser.add_argument(
        "--token",
        default="usdc",
        choices=sorted(TOKEN_POOLS.keys()),
        help="Quote token to inspect (default: usdc)",
    )
    parser.add_argument(
        "--block",
        type=int,
        help="Optional block number to query (defaults to latest available)",
    )
    args = parser.parse_args()

    token = args.token
    for label, result in run_snapshot(token, args.block):
        print(f"\n=== {label} ===")
        if result is None:
            print("PyReth could not load liquidity information (check DB range)")
            continue
        pprint(result)


if __name__ == "__main__":
    main()
