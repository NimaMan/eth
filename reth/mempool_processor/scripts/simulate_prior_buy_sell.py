#!/usr/bin/env python3
"""
Re-run the pool buy/approve/sell simulation with a specified prior transaction.

This mirrors the mempool processor's sequence:
  1. Execute the pending liquidity transaction (prior tx)
  2. Attempt buy → approve → sell on the target pool
"""

from __future__ import annotations

import argparse
from textwrap import dedent

import pyreth


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Simulate buy/approve/sell with a prior transaction executed first.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--token", required=True, help="Token address")
    parser.add_argument("--pool", required=True, help="Pool address")
    parser.add_argument(
        "--block",
        type=int,
        required=True,
        help="State block number to simulate against (typically block before the liquidity tx mined)",
    )
    parser.add_argument("--prior-tx", required=True, help="Liquidity transaction hash used as prior")
    parser.add_argument(
        "--buy-amount-eth",
        type=float,
        default=0.01,
        help="ETH amount to spend on the buy leg",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    print("=" * 80)
    print("Prior TX + Buy/Approve/Sell Simulation")
    print("=" * 80)
    print(f"Token       : {args.token}")
    print(f"Pool        : {args.pool}")
    print(f"Sim block   : {args.block}")
    print(f"Prior tx    : {args.prior_tx}")
    print(f"Buy amount  : {args.buy_amount_eth} ETH")
    print()

    reth = pyreth.PyReth()
    tx_processor = reth.tx_processor()
    simulator = reth.pool_buy_sell_simulator()

    prior_processed = tx_processor.process_transaction_from_hash_with_simulation(args.prior_tx)
    print(f"Prior tx block: {prior_processed.block_number}")
    print(f"Prior tx status: {prior_processed.status}")
    print()

    config = pyreth.PoolBuySellParameters.with_buy_amount(args.buy_amount_eth)
    config.block_number = args.block
    config.set_prior_tx_from_processed(prior_processed)

    result = simulator.check_uniswap_v2_pool(
        token_address=args.token,
        pool_address=args.pool,
        config=config,
    )

    print("Simulation result:")
    print(f"  can_buy      : {result.can_buy}")
    print(f"  can_approve  : {getattr(result, 'can_approve', 'n/a')}")
    print(f"  can_sell     : {result.can_sell}")
    print(f"  buy_tax (%)  : {result.buy_tax_percentage:.2f}")
    print(f"  sell_tax (%) : {result.sell_tax_percentage:.2f}")
    print(f"  error        : {result.error_message}")
    print()

    selector_help = dedent(
        """
        Function selectors:
          0x7ff36ab5 -> swapExactETHForTokens
          0x791ac947 -> swapExactTokensForETHSupportingFeeOnTransferTokens
          0x18cbafe5 -> swapExactTokensForETH (non fee-supporting)
        """
    ).strip()

    buy_selector = result.buy_transaction.input[:10]
    sell_selector = result.sell_transaction.input[:10]

    print("Buy/Sell call selectors:")
    print(f"  buy selector : {buy_selector}")
    print(f"  sell selector: {sell_selector}")
    print(selector_help)

    if not result.sell_transaction.status:
        print("⚠️ Sell leg failed; inspect simulation traces for revert reason.")


if __name__ == "__main__":
    main()
