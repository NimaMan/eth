#!/usr/bin/env python3
"""
FLOKI buy diagnostics using the pool buy/sell simulator.

We attempt a 1 ETH buy on the FLOKI/WETH Uniswap V2 pool at a fixed block and
print detailed balance changes and revert reasons for each leg.
"""

from __future__ import annotations

import pyreth

TOKEN_ADDRESS = "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E"  # FLOKI
POOL_ADDRESS = "0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0"    # FLOKI/WETH V2 pool
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
BLOCK_NUMBER = 23_247_278  # same height used in Rust examples
BUY_AMOUNT_ETH = 1.0
FLOKI_DECIMALS = 9
WETH_DECIMALS = 18


def format_balance_changes(balances, label: str) -> None:
    if balances is None:
        print(f"    {label}: <no balance changes recorded>")
        return
    currency_net = getattr(balances, "currency_net", {})
    token_net = getattr(balances, "token_net", {})
    if currency_net:
        print(f"    {label} currency_net:")
        for sym, amount in currency_net.items():
            print(f"      {sym}: {amount}")
    if token_net:
        print(f"    {label} token_net:")
        for addr, amount in token_net.items():
            print(f"      {addr}: {amount}")


def main() -> None:
    simulator = pyreth_pool_buy_sell_simulator()

    config = pyreth.PoolBuySellParameters.with_denom_amount(BUY_AMOUNT_ETH, FLOKI_DECIMALS, WETH_DECIMALS)
    config.denom_address = WETH_ADDRESS
    config.block_number = BLOCK_NUMBER
    # run immediately after the liquidity tx, so skip prior tx replay here

    print("=== FLOKI Buy → Approve → Sell Diagnostics ===")
    print(f"Token      : {TOKEN_ADDRESS}")
    print(f"Pool       : {POOL_ADDRESS}")
    print(f"Block      : {BLOCK_NUMBER}")
    print(f"Buy Amount : {BUY_AMOUNT_ETH} ETH")
    print()

    result = simulator.check_uniswap_v2_pool(
        token_address=TOKEN_ADDRESS,
        pool_address=POOL_ADDRESS,
        config=config,
    )

    print("Simulation summary:")
    print(f"  can_buy     : {result.can_buy}")
    print(f"  can_approve : {result.can_approve}")
    print(f"  can_sell    : {result.can_sell}")
    print(f"  failure     : {result.error_message}")
    print(f"  tokens_out  : {result.tokens_received_raw}")
    print(f"  denom_spent : {result.denom_spent_raw}")
    print(f"  denom_recv  : {result.denom_received_raw}")
    print()

    def print_tx(label: str, tx):
        print(f"{label} transaction:")
        print(f"  status        : {tx.status}")
        revert_reason = getattr(tx, "revert_reason", None)
        if revert_reason is None:
            revert_reason = tx.to_dict().get("revert_reason")
        print(f"  revert_reason : {revert_reason}")
        balance_map = tx.address_balance_changes.get(config.buyer_address)
        format_balance_changes(balance_map, "Buyer balance changes")
        print()

    print_tx("Buy", result.buy_transaction)
    print_tx("Approve", result.approve_transaction)
    print_tx("Sell", result.sell_transaction)


if __name__ == "__main__":
    main()
