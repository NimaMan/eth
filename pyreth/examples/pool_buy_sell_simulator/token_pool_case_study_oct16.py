#!/usr/bin/env python3
"""
Case study for 0x162b…2Ec / 0xfD7b…Ee1 around blocks 23,592,071–23,592,072.

Runs two simulator passes:
  1. Block 23,592,071 without any prior transaction.
  2. Block 23,592,072 replaying the LP approval that unblocked trading.

For each pass we log the buyer account's ETH balance and the simulation outcome.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal, getcontext
from typing import Optional, Tuple

import pyreth

WETH_ADDRESS = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"
WETH_DECIMALS = 18

TOKEN_ADDRESS = "0x162ba80A7c8d61090F52Da0e23260313d292b2Ec"
POOL_ADDRESS = "0xfD7b14BcD142d6d55b691ba3FD7211517717EEe1"
TOKEN_DECIMALS = 18
BUY_AMOUNT_ETH = 0.01

BLOCK_NO_PRIORLESS = 23_592_071
BLOCK_NO_WITH_PRIOR = 23_592_072

# LP approval that unblocked trading in the next block.
PRIOR_TX_HASH = "0x478bbb9ad645c0589bf742afbfcd937674e6d25c61cd4cc275f84f3e2aa702c6"

# Configure Decimal to show enough precision for ETH balances.
getcontext().prec = 30


@dataclass
class SimulationOutcome:
    label: str
    block: int
    buyer_address: str
    buyer_balance_wei: int
    can_buy: bool
    can_approve: bool
    can_sell: bool
    buy_tax: float
    sell_tax: float
    error: Optional[str]
    buy_selector: str
    sell_selector: str
    sell_status: bool
    prior_label: Optional[str]
    prior_from_address: Optional[str]
    prior_gas_used: Optional[int]
    prior_gas_price: Optional[int]


def wei_to_eth(wei_value: int) -> Decimal:
    return Decimal(wei_value) / Decimal(10**18)


def format_selector(calldata: str) -> str:
    return calldata[:10]


def fetch_prior_details(prior: pyreth.ProcessedTransaction) -> Tuple[Optional[str], Optional[int], Optional[int]]:
    info = prior.to_dict()
    fees = info.get("fees", {})
    gas_price_raw = fees.get("gas_price")
    gas_used_raw = fees.get("gas_used")
    gas_used = int(gas_used_raw) if gas_used_raw is not None else None
    gas_price = int(gas_price_raw) if gas_price_raw is not None else None
    return (
        info.get("from_address"),
        gas_used,
        gas_price,
    )


def run_simulation(
    block_number: int,
    label: str,
    prior_hash: Optional[str] = None,
) -> SimulationOutcome:
    chain_query = pyreth_chain_query()
    simulator = pyreth_pool_buy_sell_simulator()
    processed_provider = pyreth_processed_tx_provider()

    config = pyreth.PoolBuySellParameters.with_denom_amount(BUY_AMOUNT_ETH, TOKEN_DECIMALS, WETH_DECIMALS)
    config.denom_address = WETH_ADDRESS
    config.block_number = block_number
    buyer_address = config.buyer_address

    buyer_balance_wei = int(chain_query.get_eth_balance(buyer_address, block_number))

    prior_label = None
    prior_from_address = None
    prior_gas_used = None
    prior_gas_price = None

    if prior_hash:
        prior_tx = processed_provider.processed_transaction_by_hash(prior_hash)
        config.set_prior_tx_from_processed(prior_tx)
        prior_label = f"processed tx {prior_hash}"
        (
            prior_from_address,
            prior_gas_used,
            prior_gas_price,
        ) = fetch_prior_details(prior_tx)

    try:
        result = simulator.check_uniswap_v2_pool(
            token_address=TOKEN_ADDRESS,
            pool_address=POOL_ADDRESS,
            config=config,
        )
        return SimulationOutcome(
            label=label,
            block=block_number,
            buyer_address=buyer_address,
            buyer_balance_wei=buyer_balance_wei,
            can_buy=result.can_buy,
            can_approve=getattr(result, "can_approve", False),
            can_sell=result.can_sell,
            buy_tax=result.buy_tax_percentage,
            sell_tax=result.sell_tax_percentage,
            error=result.error_message,
            buy_selector=format_selector(result.buy_transaction.input),
            sell_selector=format_selector(result.sell_transaction.input),
            sell_status=result.sell_transaction.status,
            prior_label=prior_label,
            prior_from_address=prior_from_address,
            prior_gas_used=prior_gas_used,
            prior_gas_price=prior_gas_price,
        )
    except Exception as exc:  # noqa: BLE001 - capture simulator failures
        return SimulationOutcome(
            label=label,
            block=block_number,
            buyer_address=buyer_address,
            buyer_balance_wei=buyer_balance_wei,
            can_buy=False,
            can_approve=False,
            can_sell=False,
            buy_tax=0.0,
            sell_tax=0.0,
            error=str(exc),
            buy_selector="n/a",
            sell_selector="n/a",
            sell_status=False,
            prior_label=prior_label,
            prior_from_address=prior_from_address,
            prior_gas_used=prior_gas_used,
            prior_gas_price=prior_gas_price,
        )


def print_outcome(outcome: SimulationOutcome) -> None:
    print("=" * 80)
    print(f"Scenario: {outcome.label}")
    print(f"Block   : {outcome.block}")
    print(f"Buyer   : {outcome.buyer_address}")
    print(f"Balance : {outcome.buyer_balance_wei} wei ({wei_to_eth(outcome.buyer_balance_wei)} ETH)")
    if outcome.prior_label:
        print(f"Prior   : {outcome.prior_label}")
        print(f"  from         : {outcome.prior_from_address}")
        print(f"  gas_used     : {outcome.prior_gas_used}")
        print(f"  gas_price    : {outcome.prior_gas_price}")
    else:
        print("Prior   : none")
    print()
    print("Simulation summary:")
    print(f"  can_buy     : {outcome.can_buy}")
    print(f"  can_approve : {outcome.can_approve}")
    print(f"  can_sell    : {outcome.can_sell}")
    print(f"  buy_tax     : {outcome.buy_tax:.2f}%")
    print(f"  sell_tax    : {outcome.sell_tax:.2f}%")
    print(f"  error       : {outcome.error}")
    print()
    print("Transaction details:")
    print(f"  Buy selector  : {outcome.buy_selector}")
    print(f"  Sell selector : {outcome.sell_selector}")
    print(f"  Sell status   : {outcome.sell_status}")
    print()


def main() -> None:

    scenario_a = run_simulation(
        block_number=BLOCK_NO_PRIORLESS,
        label="Block 23592071 – no prior",
    )
    print_outcome(scenario_a)

    scenario_b = run_simulation(
        block_number=BLOCK_NO_WITH_PRIOR,
        label="Block 23592072 – prior = LP approval",
        prior_hash=PRIOR_TX_HASH,
    )
    print_outcome(scenario_b)


if __name__ == "__main__":
    main()
