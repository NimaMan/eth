#!/usr/bin/env python3
"""
Chrono pool bootstrap replay

Reproduces the "Contract 0x7a25... reverted without returning data" failure from
logs/pools/PoolState_20251026_124854.log (block 23647432, tx
0x5c8299f80652f2b4671bfae678352b3ce7e586e91b5b1daf5fb95b1917badb74).

The baseline simulation starts from block 23647431 (one block before the pool
exists) and therefore matches the TradingStatus error: Uniswap Router v2
reverts because the pair has not been created and funded yet.

Adding the contract deployment (nonce 0) and the initial liquidity supply
(nonce 1) as prior transactions updates the mempool snapshot, allowing the buy
simulation to succeed.
"""

from typing import Any

import pyreth
from pyreth import block_processor, pool_buy_sell_simulator

WETH_ADDRESS = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"
WETH_DECIMALS = 18


TOKEN_ADDRESS = "0xE875f15C82436c72568a206e50B0e2D44e7ad637"
POOL_ADDRESS = "0x33c34c89735a1ECA866f24d18182cAe84377Ad31"
BLOCK_BEFORE_LAUNCH = 23647431  # block used for the live TradingStatus probes
TOKEN_DECIMALS = 9              # chrono token configuration
BUY_AMOUNT_ETH = 0.5

# Prior transactions that must be replayed when working off the previous block.
PRIOR_TX_HASHES = [
    "0xf882b0ac9cff7877268f62000a57ac06015c0ca75170e53135e701bd6db9f5ff",  # nonce 0: contract deployment
    "0x5c8299f80652f2b4671bfae678352b3ce7e586e91b5b1daf5fb95b1917badb74",  # nonce 1: initial liquidity
]


def prepare_config(priors: list[pyreth.ProcessedTransaction] | None = None) -> pyreth.PoolBuySellParameters:
    """Build a simulator configuration anchored at block 23647431."""
    config = pyreth.PoolBuySellParameters.with_denom_amount(BUY_AMOUNT_ETH, TOKEN_DECIMALS, WETH_DECIMALS)
    config.denom_address = WETH_ADDRESS
    config.block_number = BLOCK_BEFORE_LAUNCH
    if priors:
        config.set_prior_transactions(priors)
    return config


def run_simulation(
    simulator: pyreth.PoolBuySellSimulator,
    label: str,
    config: pyreth.PoolBuySellParameters,
) -> pyreth.PoolBuySellSimulationResult | None:
    """Run the pool viability check and print a compact summary."""
    print(f"=== {label} ===")
    try:
        result = simulator.check_uniswap_v2_pool(
            token_address=TOKEN_ADDRESS,
            pool_address=POOL_ADDRESS,
            config=config,
        )
    except Exception as exc:  # noqa: BLE001 - diagnostics for CLI usage
        print(f"Simulation failed: {exc}\n")
        return None

    print(f"can_buy        : {result.can_buy}")
    print(f"can_sell       : {result.can_sell}")
    print(f"error_message  : {result.error_message}")
    print(f"tokens_received: {result.tokens_received_raw}")
    print(
        f"buy_tax/sell_tax: "
        f"{result.buy_tax_percentage:.4f}% / {result.sell_tax_percentage:.4f}%\n"
    )
    return result


def load_prior_transactions(provider: Any) -> list[pyreth.ProcessedTransaction]:
    """Fetch the deployment + liquidity txs that seed the new pool."""
    priors: list[pyreth.ProcessedTransaction] = []
    for tx_hash in PRIOR_TX_HASHES:
        prior = provider.processed_transaction_by_hash(tx_hash)
        priors.append(prior)
        print(f"  loaded {tx_hash} (nonce {prior.nonce})")
    print()
    return priors


def main() -> None:
    print("Chrono pool bootstrap simulation demo\n")

    simulator = pool_buy_sell_simulator()
    provider = block_processor()

    baseline = run_simulation(
        simulator,
        "Baseline (no prior tx)",
        prepare_config(),
    )
    if baseline and baseline.error_message:
        print("Baseline result matches the live TradingStatus error.\n")

    print("Replaying deployment + initial liquidity before simulating the buy...")
    priors = load_prior_transactions(provider)
    run_simulation(
        simulator,
        "With deployment + liquidity priors",
        prepare_config(priors),
    )


if __name__ == "__main__":
    main()
