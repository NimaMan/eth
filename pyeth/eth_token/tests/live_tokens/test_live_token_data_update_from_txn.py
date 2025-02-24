"""
Test LiveTokenData Basic Information Updates

Objective:
---------
Verify that LiveTokenData correctly updates basic token information from transactions

Test Cases:
----------
1. Basic token info (name, symbol, decimals)
2. Contract and ownership info
3. Creation data
4. Pair addresses and denominator
"""

import pytest
from eth_token.live_erc20_token.data.live_token_data import LiveTokenData
from dataclasses import asdict

# Test tokens with their expected properties
TEST_TOKENS = [
    {
        "address": "0xDee6cDd28Da9f51e3A8421395973894a884F3B2D",
        "expected": {
            "name": "Hoodrat",
            "symbol": "HOODRAT",
            "decimals": 9,
            "total_supply": 420689899999994.0,
            "owner": "0x53fC9704E24329e9373CF0863dB6a20aFe64fe35",
            "creator_address": "0x53fC9704E24329e9373CF0863dB6a20aFe64fe35",
            "creator_nonce": 0,
            "creation_block": 21448578,
            "creation_txn": "0x9c05dee28f40d7a04d2d24527d87b681872a891e3c5c874da846443f89424b1e",
            "pair_addresses": {"0x8e9A26b8334c4870702b23cFA2Ed73B22733bd49"},
            "denom_address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        }
    }
]

@pytest.mark.parametrize("token_info", TEST_TOKENS)
def test_live_token_basic_info(
    token_info,
    txn_data_fetcher,
    txn_analyzer
):
    """Test that basic token information is correctly updated from transactions"""
    
    contract_address = token_info["address"]
    expected = token_info["expected"]
    
    # Create live token data instance
    token_data = LiveTokenData(contract_address=contract_address)
    
    # Get creation transaction
    txn_data = txn_data_fetcher.get_transaction_data(expected["creation_txn"])
    dtxn = txn_analyzer.analyze_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    
    # Update token data
    token_data.update_from_transaction(asdict(dtxn))
    
    # Verify basic token information
    assert token_data.name == expected["name"], \
        f"Name mismatch: expected {expected['name']}, got {token_data.name}"
    assert token_data.symbol == expected["symbol"], \
        f"Symbol mismatch: expected {expected['symbol']}, got {token_data.symbol}"
    assert token_data.decimals == expected["decimals"], \
        f"Decimals mismatch: expected {expected['decimals']}, got {token_data.decimals}"
    
    # Verify contract and ownership info
    assert token_data.contract_address == contract_address, \
        f"Contract address mismatch: expected {contract_address}, got {token_data.contract_address}"
    assert token_data.current_owner == expected["owner"], \
        f"Owner mismatch: expected {expected['owner']}, got {token_data.current_owner}"
    
    # Verify creation data
    assert token_data.creator_address == expected["creator_address"], \
        f"Creator address mismatch: expected {expected['creator_address']}, got {token_data.creator_address}"
    assert token_data.creator_nonce == expected["creator_nonce"], \
        f"Creator nonce mismatch: expected {expected['creator_nonce']}, got {token_data.creator_nonce}"
    assert token_data.creation_block == expected["creation_block"], \
        f"Creation block mismatch: expected {expected['creation_block']}, got {token_data.creation_block}"
    assert token_data.creation_txn == expected["creation_txn"], \
        f"Creation transaction mismatch: expected {expected['creation_txn']}, got {token_data.creation_txn}"
    
    # Verify pair addresses and denominator
    assert token_data.pair_addresses == expected["pair_addresses"], \
        f"Pair addresses mismatch: expected {expected['pair_addresses']}, got {token_data.pair_addresses}"
    assert token_data.denom_address == expected["denom_address"], \
        f"Denominator address mismatch: expected {expected['denom_address']}, got {token_data.denom_address}"
    
    # Verify total supply
    assert abs(token_data.total_supply - expected["total_supply"]) < 1e-6, \
        f"Total supply mismatch: expected {expected['total_supply']}, got {token_data.total_supply}"
    assert abs(token_data.total_supply_from_transfers - expected["total_supply"]) < 1e-6, \
        f"Total supply from transfers mismatch: expected {expected['total_supply']}, got {token_data.total_supply_from_transfers}"
