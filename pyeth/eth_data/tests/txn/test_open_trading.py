"""
Test for analyzing trading enabled transactions

Objective: Verify correct parsing of trading enabled transactions including:
- TradingEnabledEvent
- Gas fee calculation
- Address normalization
"""

import asyncio
from eth_block_processor.data_models.txn_models import DetailedTransaction, TransactionFees, TradingEnabledEvent


def test_open_trading(txn_analyzer, txn_data_fetcher):
    """Test that both sync and async analysis match the expected trading enabled details"""
    
    expected_trading = DetailedTransaction(
        hash="0x9fc6130629c69e689d6023ffb2cfbfcd7df18210e2ba97d527c9b93819c6ec5b",
        block_number=21423691,
        txn_index=0,  # Position In Block: 0
        from_address="0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
        to_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
        contract_address=None,
        value=0.0,
        status=True,
        nonce=3,
        txn_type="Trading Enabled",
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
        trading_enabled_events=[
            TradingEnabledEvent(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                block_number=21423691,
                log_index=0  # We'll need to verify this from actual logs
            )
        ],
        trading_disabled_events=[],
        other_events=[],
        fees=TransactionFees(
            gas_price=30289321785,
            gas_used=50611,
            txn_fee=0.001532972864860635
        ),
        unique_addresses={
            "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
            "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"
        },
        erc20_contracts={
            "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"  # The token contract
        },
        state_diffs={},
        latest_states={},
        bribe_amount=0.0,
        input="0xc9567bf9"
    )

    txn_hash = "0x9fc6130629c69e689d6023ffb2cfbfcd7df18210e2ba97d527c9b93819c6ec5b"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    
    # Test synchronous analysis
    sync_result = txn_analyzer.process_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    
    assert sync_result == expected_trading 

   
    # Test asynchronous analysis
    async def run_async_analysis():
        return await txn_analyzer.process_transaction_async(
            txn_data['transaction'],
            txn_data['receipt'],
            txn_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result == expected_trading 