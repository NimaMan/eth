from __future__ import annotations

from typing import Optional
import pyreth as pr


def make_action(
    route_kind: str,
    pool: str,
    side: str,
    token: str,
    size_raw: int | str,
    *,
    fee: Optional[int] = None,
    exec_mode: Optional[str] = None,
) -> pr.PyStablecoinAction:
    """Convenience wrapper to build a `PyStablecoinAction`.

    - `route_kind`: 'univ2' | 'sushiv2' | 'univ3'
    - `pool`: pool address (checksum or 0x...
    - `side`: 'Buy' | 'Sell'
    - `token`: 'USDC' | 'USDT' | 'DAI' | 'ETH' (ETH only valid with 'Sell' currently unsupported by router)
    - `size_raw`: wei for buys, token raw for sells
    - `fee`: required for 'univ3' (e.g., 500 or 3000)
    - `exec_mode`: optional, e.g., 'WithPermit' (placeholder)
    """
    return pr.PyStablecoinAction(route_kind, pool, side, token, size_raw, fee=fee, exec_mode=exec_mode)


# Common pool constants (Mainnet)
UNIV3_WETH_USDC_500 = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
UNIV3_WETH_USDC_3000 = "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8"
UNIV2_WETH_USDC = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
SUSHI_V2_WETH_USDT = "0x06da0fd433C1A5d7a4faa01111c044910A184553"
UNIV2_WETH_DAI = "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11"

