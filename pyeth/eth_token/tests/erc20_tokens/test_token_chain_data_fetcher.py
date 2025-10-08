"""Smoke tests for `TokenChainDataFetcher` using well-known stablecoins."""

import pytest

from eth_token.erc20_token.data.token_chain_data_fetcher import (
    TokenChainDataFetcher,
)


USDC = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"


@pytest.mark.parametrize(
    "token_address,expected_symbol,expected_decimals",
    [
        (USDC, "USDC", 6),
    ],
)
def test_token_metadata_from_chain(token_address: str, expected_symbol: str, expected_decimals: int) -> None:
    fetcher = TokenChainDataFetcher()

    symbol = fetcher.get_token_symbol(token_address)
    decimals = fetcher.get_token_decimals(token_address)
    name = fetcher.get_token_name(token_address)
    metadata = fetcher.get_token_metadata(token_address)

    assert symbol == expected_symbol
    assert decimals == expected_decimals
    assert name.lower().startswith("usd")
    assert metadata.symbol == expected_symbol
    assert int(metadata.decimals) == expected_decimals
