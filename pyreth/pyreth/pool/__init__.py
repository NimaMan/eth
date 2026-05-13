"""Pool analysis tools.

Usage:
    from pyreth.pool import assess_pool
    result = assess_pool("0x...", "0x...")
"""
from __future__ import annotations

from typing import Any

from pyreth import chain_query as _chain_query
from pyreth import pool_buy_sell_simulator as _pool_sim
from pyreth import PoolBuySellParameters


def assess_pool(token: str, pool: str, block: int | None = None) -> dict[str, Any]:
    """Assess a token and its primary pool."""
    q = _chain_query()
    sim = _pool_sim()

    name = q.get_token_name(token)
    symbol = q.get_token_symbol(token)
    decimals = q.get_token_decimals(token)
    total_supply_raw = q.get_token_total_supply(token)
    total_supply_scaled = None
    if total_supply_raw and decimals:
        try:
            total_supply_scaled = str(int(total_supply_raw) / (10 ** decimals))
        except Exception:
            pass

    try:
        liq = q.get_uniswap_v2_liquidity(pool, block)
        pool_data = {
            "address": pool,
            "protocol": liq.protocol,
            "token0": liq.token0,
            "token1": liq.token1,
            "reserve0_raw": liq.reserve0_raw,
            "reserve1_raw": liq.reserve1_raw,
            "reserve0_scaled": liq.reserve0_scaled,
            "reserve1_scaled": liq.reserve1_scaled,
            "price_1e18": liq.price_1e18,
            "block_number": liq.block_number,
        }
    except Exception as e:
        pool_data = {"address": pool, "error": str(e)}

    tradable = {"error": None}
    try:
        weth = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        config = PoolBuySellParameters.with_denom_amount(0.01, decimals or 18, 18)
        config.denom_address = weth
        if block:
            config.block_number = block

        result = sim.check_uniswap_v2_pool(
            token_address=token,
            pool_address=pool,
            config=config,
        )
        tradable = {
            "can_buy": result.can_buy,
            "can_approve": result.can_approve,
            "can_sell": result.can_sell,
            "buy_tax_percent": result.buy_tax_percentage,
            "sell_tax_percent": result.sell_tax_percentage,
            "block_number": result.block_number,
            "pool_type": result.pool_type,
            "error": result.error_message,
        }
    except Exception as e:
        tradable = {"error": str(e)}

    classification = {"cohort": "unknown", "reason": None}
    try:
        if "reserve1_scaled" in pool_data:
            reserve1 = float(pool_data["reserve1_scaled"]) if pool_data["reserve1_scaled"] else 0.0
            can_buy = tradable.get("can_buy", False)
            can_sell = tradable.get("can_sell", False)
            if reserve1 < 0.5:
                classification = {"cohort": "ineligible", "reason": "low_liquidity"}
            elif not can_buy:
                classification = {"cohort": "ineligible", "reason": "cannot_buy"}
            elif not can_sell:
                classification = {"cohort": "ineligible", "reason": "cannot_sell"}
            else:
                classification = {"cohort": "eligible", "reason": None}
    except Exception:
        pass

    return {
        "token": {
            "address": token,
            "name": name,
            "symbol": symbol,
            "decimals": decimals,
            "total_supply_raw": total_supply_raw,
            "total_supply_scaled": total_supply_scaled,
        },
        "pool": pool_data,
        "tradable": tradable,
        "classification": classification,
        "market": {
            "dexscreener_url": f"https://dexscreener.com/ethereum/{pool}",
        },
    }


def get_liquidity(pool: str, block: int | None = None) -> dict[str, Any]:
    """Get raw pool reserves and price."""
    q = _chain_query()
    try:
        liq = q.get_uniswap_v2_liquidity(pool, block)
        return {
            "pool": liq.pool,
            "protocol": liq.protocol,
            "token0": liq.token0,
            "token1": liq.token1,
            "reserve0_raw": liq.reserve0_raw,
            "reserve1_raw": liq.reserve1_raw,
            "reserve0_scaled": liq.reserve0_scaled,
            "reserve1_scaled": liq.reserve1_scaled,
            "price_1e18": liq.price_1e18,
            "block_number": liq.block_number,
        }
    except Exception as e:
        return {"pool": pool, "error": str(e)}
