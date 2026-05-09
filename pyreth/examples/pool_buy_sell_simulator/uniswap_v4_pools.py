#!/usr/bin/env python3
"""
Uniswap V4 Pools - Buy → Approve → Sell Simulation

Runs the Baygus executor powered pipeline against a known Uniswap v4 USDC/WETH pool.
Prints the swap outcomes and gas metrics for each leg if the pool is tradeable.
"""

from pyreth import pool_buy_sell_simulator as pyreth_pool_buy_sell_simulator

WETH_ADDRESS = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"
WETH_DECIMALS = 18

# Mainnet PoolManager, pool id, and currencies for the hookless USDC/WETH v4 pool.
TOKEN = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"  # USDC
POOL_MANAGER = "0x000000000004444C5DC75cB358380d2E3de08a90"
POOL_ID = (
    "0x11142dd4ac627021305b9349c2167d89744c4e45c92ce383c04120337f86495c"
)
CURRENCY0 = TOKEN
CURRENCY1 = WETH_ADDRESS
FEE = 490
TICK_SPACING = 10
HOOKS = "0x0000000000000000000000000000000000000000"
BLOCK_NUMBER = 23_560_197


def main():
    sim = pyreth_pool_buy_sell_simulator()

    cfg = sim.default_config(6, WETH_DECIMALS)
    cfg.denom_address = WETH_ADDRESS
    cfg.denom_amount = 0.01
    cfg.block_number = BLOCK_NUMBER
    cfg.buy_gas_limit = 800_000
    cfg.approve_gas_limit = 250_000
    cfg.sell_gas_limit = 800_000
    cfg.set_uniswap_v4_config(
        POOL_MANAGER,
        POOL_ID,
        CURRENCY0,
        CURRENCY1,
        FEE,
        TICK_SPACING,
        HOOKS,
    )

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
        print("Tokens Received:", res.tokens_received_raw)
        print("ETH Spent:", res.denom_spent_raw)
        print("ETH Received:", res.denom_received_raw)
        if res.error_message:
            print("❌ Swap pipeline failed")
        else:
            print("✅ Baygus executor v4 buy → approve → sell succeeded")
    except Exception as e:
        print("❌ Unexpected error calling V4 check:", e)
        raise


if __name__ == "__main__":
    main()
