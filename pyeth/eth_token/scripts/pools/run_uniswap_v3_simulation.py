"""Quick Uniswap V3 trading-viability smoke test.

Usage:
    python -m eth_token.scripts.pools.run_uniswap_v3_simulation
"""

from decimal import Decimal

import pyreth

from eth_data.utils.pyreth_client import PyrethClient


# Known mainnet addresses (USDC/WETH 0.3% pool)
USDC_ADDRESS = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606EB48"
USDC_WETH_V3_POOL = "0x8ad599c3a0ff1de082011efddc58f1908eb6e6d8"
FEE_TIER = 3000  # 0.30%
USDC_DECIMALS = 6
ETH_DECIMALS = Decimal(10) ** 18


def run_simulation(test_amount_eth: float = 1.0) -> None:
    client = PyrethClient.instance()
    simulator = client.pool_buy_sell_simulator()

    config = pyreth.PoolBuySellParameters(USDC_DECIMALS, 18)
    config.denom_amount = test_amount_eth
    # Leaving `block_number` unset lets the simulator pick the latest head.

    result = simulator.check_uniswap_v3_pool(
        USDC_ADDRESS,
        USDC_WETH_V3_POOL,
        FEE_TIER,
        config,
    )

    token_factor = Decimal(10) ** USDC_DECIMALS
    tokens_raw = result.tokens_received_raw if hasattr(result, "tokens_received_raw") else None
    denom_spent_raw = result.denom_spent_raw if hasattr(result, "denom_spent_raw") else None
    denom_received_raw = (
        result.denom_received_raw if hasattr(result, "denom_received_raw") else None
    )

    tokens_received = (
        Decimal(tokens_raw) / token_factor if tokens_raw is not None else None
    )
    denom_spent = (
        Decimal(denom_spent_raw) / ETH_DECIMALS if denom_spent_raw is not None else None
    )
    denom_received = (
        Decimal(denom_received_raw) / ETH_DECIMALS
        if denom_received_raw is not None
        else None
    )

    print("=== Uniswap V3 Pool Simulation ===")
    print(f"Pool        : {USDC_WETH_V3_POOL}")
    print(f"Token       : {USDC_ADDRESS}")
    print(f"Fee tier    : {FEE_TIER}")
    print(f"Test amount : {test_amount_eth:.6f} ETH")
    print(f"Can buy     : {result.can_buy}")
    print(f"Can approve : {result.can_approve}")
    print(f"Can sell    : {result.can_sell}")
    print(f"Buy tax %   : {result.buy_tax_percentage:.4f}")
    print(f"Sell tax %  : {result.sell_tax_percentage:.4f}")
    if tokens_received is not None and denom_spent is not None and denom_received is not None:
        print(f"USDC bought : {tokens_received:.6f} USDC")
        print(f"ETH spent   : {denom_spent:.6f} ETH")
        print(f"ETH received: {denom_received:.6f} ETH")
    else:
        print("USDC bought : <upgrade PyReth bindings to view amount metrics>")
    print(f"Block used  : {result.block_number}")
    if result.error_message:
        print(f"Note        : {result.error_message}")


if __name__ == "__main__":
    run_simulation()
