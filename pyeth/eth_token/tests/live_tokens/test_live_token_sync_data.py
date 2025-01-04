"""
Test LiveERC20Token Data Synchronization

Objective:
---------
Verify that LiveERC20Token correctly syncs historical data with live updates

Test Cases:
----------
1. Sync from creation transaction
2. Sync from specific block number
3. Verify price data matches historical records
"""

import pytest
from eth_token_analyzer.erc20_token.erc20_token import ERC20Token
from eth_token_monitor.live_erc20_token.live_token import LiveERC20Token
from dataclasses import asdict


# Test tokens with their addresses and expected properties
TEST_TOKENS = [
    "0x1bC9cCCCF92839894D615Fe292AE45E30fF7ecae",  # Token 1
]


@pytest.mark.parametrize("contract_address", TEST_TOKENS)
def test_live_token_sync_from_creation(
    contract_address,
    txn_data_fetcher,
    txn_analyzer
):
    """Test syncing token data from creation transaction"""
    
    # Load historical token for comparison
    historical_token = ERC20Token(contract_address)
    
    # Get first 10 transactions including creation
    txn_hashes = [historical_token["txn_hash_creation"]] + historical_token.price_df.index.tolist()[:10]
    
    # Create and sync live token
    live_token = LiveERC20Token(contract_address=contract_address)
    
    # Process transactions sequentially
    for txn_hash in txn_hashes:
        txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
        dtxn = txn_analyzer.analyze_transaction(
            txn_data['transaction'],
            txn_data['receipt'],
            txn_data['trace']
        )
        live_token.update_from_transaction(asdict(dtxn))
    
    # Verify token properties
    assert live_token.symbol == historical_token.symbol, \
        f"Symbol mismatch for {contract_address}"
    assert live_token.decimals == historical_token.decimals, \
        f"Decimals mismatch for {contract_address}"
    assert live_token.creation_block == historical_token.creation_block, \
        f"Creation block mismatch for {contract_address}"
    
    # Verify price data
    historical_prices = historical_token.price_df.iloc[0:10].relative_price
    live_prices = live_token.price_df.iloc[0:10].relative_price
    
    assert len(live_prices) == len(historical_prices), \
        f"Price data length mismatch for {contract_address}"
    
    # Compare each price point
    for idx, (hist_price, live_price) in enumerate(zip(historical_prices, live_prices)):
        assert abs(hist_price - live_price) < 1e-10, \
            f"Price mismatch at index {idx} for {contract_address}: historical={hist_price}, live={live_price}"

