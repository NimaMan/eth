#!/usr/bin/env python3
"""
Mind of Pepe Sequential Replay Demo

Replays the creator transaction sequence for Mind of Pepe before running the
standard buy/approve/sell viability probe. Demonstrates the new sequential
prior transaction support in the simulator.
"""

from __future__ import annotations

from typing import List

import pyreth

WETH_ADDRESS = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"
WETH_DECIMALS = 18



TOKEN_ADDRESS = "0x2A179Bfab23c71c733864776bEa6e0daB9eCD79e"
POOL_ADDRESS = "0xFB02Ffd5b26229464C0d41f0fA9B6895CDdDA885"  # Uniswap V2 WETH pair
BLOCK_NUMBER = 23606650

# Deployer transaction sequence (nonce order).
PRIOR_TRANSACTION_HASHES: List[str] = [
    # Contract creation
    "0x027399b9a14e8e7c1d86b8f82f205abdef3b5c4ccbc2475cba70dcbbcf187b2b",
    # openTrading() + liquidity initialization
    "0x7468044e32eeab38ed8afe6ee17648a8effa6057da9f152be03ff3a2aea7af22",
]


def load_prior_transactions(
    tx_processor: "pyreth.TxProcessor", hashes: List[str]
) -> List["pyreth.ProcessedTransaction"]:
    prior = []
    for tx_hash in hashes:
        print(f"  ↳ Fetching processed transaction {tx_hash}")
        prior.append(tx_processor.process_transaction_from_hash_with_simulation(tx_hash))
    return prior


def main() -> None:
    print("=" * 80)
    print("Mind of Pepe – Sequential Prior Replay")
    print("=" * 80)
    print(f"Token: {TOKEN_ADDRESS}")
    print(f"Pool : {POOL_ADDRESS}")
    print(f"Block: {BLOCK_NUMBER}")
    print()

    simulator = pyreth_pool_buy_sell_simulator()
    tx_processor = pyreth_tx_processor()

    print("Loading prior transaction sequence:")
    prior_transactions = load_prior_transactions(tx_processor, PRIOR_TRANSACTION_HASHES)
    print(f"  Loaded {len(prior_transactions)} prior transactions\n")

    config = pyreth.PoolBuySellParameters.with_denom_amount(0.05, 18, WETH_DECIMALS)
    config.denom_address = WETH_ADDRESS
    config.block_number = BLOCK_NUMBER
    config.set_prior_transactions(prior_transactions)

    print("Running buy/approve/sell simulation with sequential priors...\n")
    result = simulator.check_uniswap_v2_pool(
        token_address=TOKEN_ADDRESS,
        pool_address=POOL_ADDRESS,
        config=config,
    )

    print("Simulation Summary")
    print("------------------")
    print(f"can_buy     : {result.can_buy}")
    print(f"can_approve : {result.can_approve}")
    print(f"can_sell    : {result.can_sell}")
    print(f"buy_tax     : {result.buy_tax_percentage:.2f}%")
    print(f"sell_tax    : {result.sell_tax_percentage:.2f}%")
    print(f"error       : {result.error_message}")
    print()

    print("Prior Transactions Applied")
    print("--------------------------")
    for idx, prior in enumerate(result.prior_transactions):
        status = "success" if prior.status else "reverted"
        print(f"[{idx}] {prior.hash} — {status}")
    print()

    buy_selector = result.buy_transaction.input[:10]
    sell_selector = result.sell_transaction.input[:10]
    print("Buy/Sell Details")
    print("----------------")
    print(f"buy selector : {buy_selector}")
    print(f"sell selector: {sell_selector}")
    print(f"tokens received: {result.tokens_received_raw}")
    print(f"eth spent      : {result.denom_spent_raw}")
    print(f"eth received   : {result.denom_received_raw}")


if __name__ == "__main__":
    main()
