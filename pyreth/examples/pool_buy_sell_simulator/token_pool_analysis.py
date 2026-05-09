from textwrap import dedent
import pyreth  # noqa: E402

WETH_ADDRESS = "0xC02aaA39b223FE8D0A0E5C4F27eAD9083C756Cc2"
WETH_DECIMALS = 18

TOKEN = "0x21eFcF554CB113DC23F5767409c780716D08b2ef"
POOL = "0x8aeaa7aC75fB24A9841d38A437d34e19954F261b"
BLOCK = 23567642


def format_selector(input_data: str) -> str:
    """Return the 4-byte selector from a hex calldata string."""
    return input_data[:10]


def main() -> None:
    print("=" * 80)
    print("Sadie Token – Pool Buy/Sell Simulation Debug")
    print("=" * 80)
    print(f"Token: {TOKEN}")
    print(f"Pool : {POOL}")
    print(f"Block: {BLOCK}")
    print()

    simulator = pyreth_pool_buy_sell_simulator()
    decimals = pyreth_chain_query().get_token_decimals(TOKEN, None)

    config = pyreth.PoolBuySellParameters.with_denom_amount(0.01, decimals, WETH_DECIMALS)
    config.denom_address = WETH_ADDRESS
    config.block_number = BLOCK

    result = simulator.check_uniswap_v2_pool(
        token_address=TOKEN,
        pool_address=POOL,
        config=config,
    )

    print("Simulation summary:")
    print(f"  can_buy : {result.can_buy}")
    print(f"  can_sell: {result.can_sell}")
    print(f"  buy_tax : {result.buy_tax_percentage:.2f}%")
    print(f"  sell_tax: {result.sell_tax_percentage:.2f}%")
    print(f"  error   : {result.error_message}")
    print()

    buy_selector = format_selector(result.buy_transaction.input)
    sell_selector = format_selector(result.sell_transaction.input)

    selector_help = dedent(
        """
        Function selectors:
          0x7ff36ab5 -> swapExactETHForTokens
          0x791ac947 -> swapExactTokensForETHSupportingFeeOnTransferTokens
          0x18cbafe5 -> swapExactTokensForETH (non fee-supporting)
        """
    ).strip()

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


if __name__ == "__main__":
    main()
