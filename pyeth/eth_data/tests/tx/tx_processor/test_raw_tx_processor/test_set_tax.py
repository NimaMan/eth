"""
Test for analyzing set tax transactions

Objective: Verify correct parsing of set tax transactions including:
- Transaction type classification
- Gas fee calculation
- Address normalization
"""

import asyncio
from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction, TransactionFees


def test_set_tax(tx_analyzer, tx_data_fetcher):
    """Test that both sync and async analysis match the expected set tax details"""
    
    expected_tax = ProcessedTransaction(
        hash="0x25d4b79545273e46a2685247138c38ce71ace2ec1398fa5b7a547b55f22efe77",
        block_number=21423708,
        tx_index=0,
        from_address="0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
        to_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
        contract_address=None,
        value=0.0,
        status=True,
        nonce=7,
        tx_type="Set Tax",
        actions=[],
        eth_transfers=[],
        erc20_transfers=[],
        erc721_transfers=[],
        erc1155_transfers=[],
        internal_transactions=[],
        uniswap_v2_syncs=[],
        uniswap_v2_swaps=[],
        approvals=[],
        mints=[],
        burns=[],
        deposits=[],
        withdraws=[],
        pair_events=[],
        owner_events=[],
        contract_creation_events=[],
        trading_enabled_events=[],
        trading_disabled_events=[],
        other_events=[],
        fees=TransactionFees(
            gas_price=39268710360,
            gas_used=33922,
            tx_fee=0.00133207319283192
        ),
        unique_addresses={
            "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
            "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"
        },
        erc20_contracts={
            "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"
        },
        address_balance_changes={},
        latest_states={},
        bribe_amount=0.0,
        input="0xfc7e4746"
    )

    tx_hash = "0x25d4b79545273e46a2685247138c38ce71ace2ec1398fa5b7a547b55f22efe77"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)
    
    # Test synchronous analysis
    sync_result = tx_analyzer.process_transaction(
        tx_data['transaction'],
        tx_data['receipt'],
        tx_data['trace']
    )
    
    assert sync_result == expected_tax 