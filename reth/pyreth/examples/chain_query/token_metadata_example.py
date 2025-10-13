#!/usr/bin/env python3
"""
Example: Query ERC20 token metadata via pyreth ChainQuery

Demonstrates get_token_name/symbol/decimals/total_supply and get_token_metadata.
"""

import pyreth


def main():
    reth = pyreth.PyReth()
    query = reth.chain_query()

    # USDC
    token = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"

    name = query.get_token_name(token)
    symbol = query.get_token_symbol(token)
    decimals = query.get_token_decimals(token)
    total_supply = query.get_token_total_supply(token)
    meta = query.get_token_metadata(token, None)

    print(f"Name: {name}")
    print(f"Symbol: {symbol}")
    print(f"Decimals: {decimals}")
    print(f"Total Supply (raw): {total_supply}")
    print(f"Metadata: name={meta.name}, symbol={meta.symbol}, decimals={meta.decimals}, total_supply={meta.total_supply}")


if __name__ == "__main__":
    main()
