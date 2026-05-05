#!/usr/bin/env python3
"""
Single-hop denomination swap demo

Simulates a Uniswap V2 buy/sell sequence where the denomination asset is USDC
and trading a single additional ERC-20 token in the same pool snapshot.
"""

from __future__ import annotations

import sys

import pyreth

TOKEN_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"  # WETH
POOL_ADDRESS = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"  # USDC/WETH Uniswap V2 pool
USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
ANCHOR_BLOCK = 23696698
BUY_AMOUNT_DENOM = 100.0  # 100 USDC
USDC_WHALE = "0x55FE002aefF02F77364de339a1292923A15844B8"  # Circle treasury (has deep USDC balance)
TOP_UP_BUFFER = BUY_AMOUNT_DENOM + 10.0  # send buy amount + buffer


def encode_erc20_transfer(to_address: str, amount: int) -> str:
    selector = "0xa9059cbb"
    addr = to_address.lower().replace("0x", "").rjust(64, "0")
    value = hex(amount)[2:].rjust(64, "0")
    return selector + addr + value


def main() -> None:
    chain_query = pyreth_chain_query()
    simulator = pyreth_pool_buy_sell_simulator()

    print("Single-hop denom swap simulation\n")
    token_symbol = chain_query.get_token_symbol(TOKEN_ADDRESS, ANCHOR_BLOCK)
    denom_symbol = chain_query.get_token_symbol(USDC_ADDRESS, ANCHOR_BLOCK)

    print(f"Token: {TOKEN_ADDRESS} ({token_symbol})")
    print(f"Pool : {POOL_ADDRESS}")
    print(f"Block: {ANCHOR_BLOCK}")
    print(f"Denom: {USDC_ADDRESS} ({denom_symbol})\n")

    token_decimals = chain_query.get_token_decimals(TOKEN_ADDRESS, ANCHOR_BLOCK)
    denom_decimals = chain_query.get_token_decimals(USDC_ADDRESS, ANCHOR_BLOCK)
    config = pyreth.PoolBuySellParameters.with_denom_amount(
        BUY_AMOUNT_DENOM, token_decimals, denom_decimals
    )
    config.denom_address = USDC_ADDRESS
    config.block_number = ANCHOR_BLOCK
    config.buyer_address = "0x28C6c06298d514Db089934071355E5743bf21d60"

    # Prefund buyer with USDC so the swap can execute even if the chosen wallet
    # has a low balance in the historical snapshot.
    try:
        whale_nonce = chain_query.get_nonce(USDC_WHALE, ANCHOR_BLOCK)
        top_up_amount = int(TOP_UP_BUFFER * (10 ** denom_decimals))
        config.set_prior_tx_from_unsigned(
            from_address=USDC_WHALE,
            to_address=USDC_ADDRESS,
            data_hex=encode_erc20_transfer(config.buyer_address, top_up_amount),
            nonce=whale_nonce,
            gas_limit=200_000,
        )
        print(f"Prefunded buyer with {TOP_UP_BUFFER} USDC from {USDC_WHALE} (nonce {whale_nonce}).")
    except Exception as err:
        print(f"⚠️  Failed to enqueue USDC top-up prior transaction: {err}")

    try:
        result = simulator.check_uniswap_v2_pool(
            token_address=TOKEN_ADDRESS,
            pool_address=POOL_ADDRESS,
            config=config,
        )
    except RuntimeError as exc:
        print("Simulation failed before producing a result:")
        print(f"  {exc}")
        sys.exit(1)

    print("=== PoolBuySellSimulator result ===")
    print(f"can_buy        : {result.can_buy}")
    print(f"can_sell       : {result.can_sell}")
    print(f"can_approve    : {result.can_approve}")
    print(f"buy_tax%       : {result.buy_tax_percentage:.4f}")
    print(f"sell_tax%      : {result.sell_tax_percentage:.4f}")
    print(f"error          : {result.error_message}")
    print(f"tokens received: {result.tokens_received_raw}")
    print(f"denom spent    : {result.denom_spent_raw}")
    print(f"denom received : {result.denom_received_raw}")
    print("")

    def log_transaction(label: str, tx) -> None:
        tx_dict = tx.to_dict()
        print(f"{label} transaction:")
        print(f"  status        : {tx_dict.get('status')}")
        print(f"  revert_reason : {tx_dict.get('revert_reason')}")
        internal = tx_dict.get("internal_transactions", [])
        failing = [call for call in internal if call.get("error")]
        if failing:
            print("  internal call errors:")
            for call in failing:
                depth = call.get("depth")
                target = call.get("to_address")
                error = call.get("error")
                print(f"    depth={depth} to={target} error={error}")
        balance = tx_dict.get("address_balance_changes", {}).get(config.buyer_address)
        if balance:
            print(f"  buyer deltas : {balance}")
        print("")

    if not result.can_buy:
        log_transaction("Buy", result.buy_transaction)
        sys.exit("❌ Buy leg failed in simulation.")
    if not result.can_sell:
        log_transaction("Sell", result.sell_transaction)
        sys.exit("❌ Sell leg failed in simulation.")

    log_transaction("Buy", result.buy_transaction)
    log_transaction("Approve", result.approve_transaction)
    log_transaction("Sell", result.sell_transaction)

    print("✅ Single-hop simulation succeeded.")


if __name__ == "__main__":
    main()
