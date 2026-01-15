#!/usr/bin/env python3
"""
Token Pool Analysis With Optional Prior Transaction

Extends the baseline `token_pool_analysis.py` helper to allow supplying a
prior transaction before running the buy → approve → sell simulation.
Useful for debugging scenarios where live trading is enabled but the
one-shot simulation reports `can_buy = False`.
"""

from __future__ import annotations

import sys
from textwrap import dedent
from typing import Any, Dict, Optional, Tuple
import pyreth

WETH_ADDRESS = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"
WETH_DECIMALS = 18



SIMULATION_BLOCK = 23592070
TOKEN_ADDRESS = "0x162ba80A7c8d61090F52Da0e23260313d292b2Ec"
POOL_ADDRESS = "0xfD7b14BcD142d6d55b691ba3FD7211517717EEe1"
TX_HASH = "0x9d7631da9121c13b6b5451d21337843e74867f5b522012230d555d294f9356da"
BUY_AMOUNT_ETH = 0.01
TOKEN_DECIMALS = 18


def format_selector(input_data: str) -> str:
    """Return the 4-byte selector from a hex calldata string."""
    return input_data[:10]


def configure_prior_transaction(
    cfg: "pyreth.PoolBuySellParameters",
    reth: pyreth.PyReth,
) -> Tuple[Optional[str], Optional[Dict[str, Any]]]:
    """Apply the static prior transaction to the simulator parameters."""
    if not TX_HASH:
        return None, None

    processed_provider = reth.processed_tx_provider()
    try:
        prior = processed_provider.processed_transaction_by_hash(TX_HASH)
        cfg.set_prior_tx_from_processed(prior)
        return f"processed tx {TX_HASH}", extract_prior_fee_info(prior)
    except Exception as exc:  # noqa: BLE001 - want helpful diagnostics for CLI usage
        print(f"⚠️  Failed to load prior processed tx {TX_HASH}: {exc}", file=sys.stderr)
        return None, None


def extract_prior_fee_info(prior: Any) -> Dict[str, Any]:
    """Return a compact summary of the prior transaction's gas parameters."""

    def parse_optional_int(value: Any) -> Optional[int]:
        if value in (None, "None"):
            return None
        try:
            return int(value)
        except (TypeError, ValueError):
            return None

    try:
        fees = prior.to_dict().get("fees", {})
    except Exception:  # noqa: BLE001 - don't let logging break the flow
        return {}

    summary: Dict[str, Any] = {
        "protocol_type": fees.get("protocol_type"),
        "gas_price": parse_optional_int(fees.get("gas_price")),
        "gas_used": fees.get("gas_used"),
        "max_fee_per_gas": parse_optional_int(fees.get("max_fee_per_gas")),
        "max_priority_fee": parse_optional_int(fees.get("max_priority_fee")),
    }
    # Reuse gas_used as the replay gas limit when the original limit is unavailable.
    summary["gas_limit"] = summary.get("gas_used")
    return summary


def main() -> None:
    print("=" * 80)
    print("Token Pool Analysis – Prior Tx Enabled")
    print("=" * 80)
    print(f"Token: {TOKEN_ADDRESS}")
    print(f"Pool : {POOL_ADDRESS}")
    print(f"Block: {SIMULATION_BLOCK}")
    print(f"Buy amount (ETH): {BUY_AMOUNT_ETH}")
    print()

    reth = pyreth.PyReth()
    simulator = reth.pool_buy_sell_simulator()

    config = pyreth.PoolBuySellParameters.with_denom_amount(BUY_AMOUNT_ETH, TOKEN_DECIMALS, WETH_DECIMALS)
    config.denom_address = WETH_ADDRESS
    config.block_number = SIMULATION_BLOCK

    prior_source, prior_fee_info = configure_prior_transaction(config, reth)
    if prior_source:
        print(f"Prior transaction: {prior_source}")
    else:
        print("Prior transaction: none")
    print()

    try:
        result = simulator.check_uniswap_v2_pool(
            token_address=TOKEN_ADDRESS,
            pool_address=POOL_ADDRESS,
            config=config,
        )
    except Exception as exc:  # noqa: BLE001 - surface simulator failures cleanly
        print(f"Simulation failed: {exc}")
        return

    print("Simulation summary:")
    print(f"  can_buy     : {result.can_buy}")
    print(f"  can_approve : {getattr(result, 'can_approve', 'n/a')}")
    print(f"  can_sell    : {result.can_sell}")
    print(f"  buy_tax     : {result.buy_tax_percentage:.2f}%")
    print(f"  sell_tax    : {result.sell_tax_percentage:.2f}%")
    print(f"  error       : {result.error_message}")
    print()

    selector_help = dedent(
        """
        Function selectors:
          0x7ff36ab5 -> swapExactETHForTokens
          0x791ac947 -> swapExactTokensForETHSupportingFeeOnTransferTokens
          0x18cbafe5 -> swapExactTokensForETH (non fee-supporting)
        """
    ).strip()

    buy_selector = format_selector(result.buy_transaction.input)
    sell_selector = format_selector(result.sell_transaction.input)

    print("Transaction details:")
    print(f"  Buy tx selector : {buy_selector}")
    print(f"  Sell tx selector: {sell_selector}")
    print(f"  Sell status     : {result.sell_transaction.status}")
    print()
    print(selector_help)
    print()

    if result.sell_transaction.status:
        print("✅ Sell simulation succeeded using the fee-supporting router variant.")
    else:
        print("⚠️ Sell simulation failed. Inspect `result.sell_transaction` for details.")

    if prior_source:
        print()
        print("Prior transaction gas settings applied:")
        if prior_fee_info:
            print(f"  protocol_type       : {prior_fee_info.get('protocol_type')}")
            print(f"  gas_limit (used)    : {prior_fee_info.get('gas_limit')}")
            print(f"  gas_price (wei)     : {prior_fee_info.get('gas_price')}")
            print(f"  max_fee_per_gas     : {prior_fee_info.get('max_fee_per_gas')}")
            print(f"  max_priority_fee    : {prior_fee_info.get('max_priority_fee')}")
            if "value_wei" in prior_fee_info:
                print(f"  value_wei           : {prior_fee_info.get('value_wei')}")
        else:
            print("  (fee details unavailable)")
        print("  simulator gas limits (wei):")
        print(f"    buy      : {getattr(config, 'buy_gas_limit', None)}")
        print(f"    approve  : {getattr(config, 'approve_gas_limit', None)}")
        print(f"    sell     : {getattr(config, 'sell_gas_limit', None)}")


if __name__ == "__main__":
    main()
