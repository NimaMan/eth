#!/usr/bin/env python3
"""
Uniswap V4 Pools - Viability Check Demo

Demonstrates calling the new check_uniswap_v4_pool() binding. Full swap simulation
is not yet implemented due to PoolManager lock/Router integration requirements,
so this example verifies that the API is present and returns a clear failure reason.
"""

import pyreth

# Example inputs (placeholders). V4 uses PoolManager + PoolId. These values are not
# used for on-chain calls yet; the API returns a failure result with explanation.
TOKEN = "0x0000000000000000000000000000000000000000"
POOL_MANAGER = "0x000000000004444C5DC75cB358380d2E3de08a90"
POOL_ID = "0x" + "00" * 32


def main():
    reth = pyreth.PyReth()
    sim = reth.pool_buy_sell_simulator()

    cfg = sim.default_config()
    cfg.test_amount_eth = 0.01

    try:
        res = sim.check_uniswap_v4_pool(TOKEN, POOL_MANAGER, POOL_ID, cfg)
        print("Pool Type:", res.pool_type)
        print("Can Buy:", res.can_buy)
        print("Can Approve:", res.can_approve)
        print("Can Sell:", res.can_sell)
        print("Buy Tax:", res.buy_tax_percentage)
        print("Sell Tax:", res.sell_tax_percentage)
        print("Block:", res.block_number)
        print("Error:", res.error_message)
        print("✅ V4 API reachable; returns informative failure as expected")
    except Exception as e:
        print("❌ Unexpected error calling V4 check:", e)
        raise


if __name__ == "__main__":
    main()

