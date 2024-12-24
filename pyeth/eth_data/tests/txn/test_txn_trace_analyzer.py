import pytest
import asyncio
from web3 import Web3
from eth_block_processor.txn.txn_trace_analyzer import TransactionTraceAnalyzer
from eth_block_processor.txn.txn_data_fetcher import TransactionDataFetcher
from eth_block_processor.data_models.trace_models import InternalTransaction
from eth_block_processor.data_models.txn_models import (
    DetailedTransaction, 
    TransactionFees,
    ERC20Transfer,
    InternalTransaction
)


def test_failed_contract_creation(txn_analyzer, txn_data_fetcher):
    """Test that both sync and async analysis match the expected failed contract creation details"""
    
    # First get the transaction data to get the actual input
    txn_hash = "0x8304000190747e7f8ace1510304ada10d66ec24e05427dc8f994ddbea3b97d6c"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    actual_input = txn_data['transaction']['input']
    
    expected_transaction = DetailedTransaction(
        hash="0x8304000190747e7f8ace1510304ada10d66ec24e05427dc8f994ddbea3b97d6c",
        block_number=21430859,
        txn_index=5,
        from_address="0x24a0A2E8943330b9e2C26BA3ccb954D9cF76c232",
        to_address="0xf508944ff0D5192B990B7Be13E12e474b8f3cA09",
        contract_address=None,
        value=0.0,
        status=False,
        nonce=1879,
        txn_type="Contract Interaction",
        actions=[],
        eth_transfers=[],
        erc20_transfers=[],
        erc721_transfers=[],
        erc1155_transfers=[],
        internal_transactions=[
            InternalTransaction(
                from_address="0x24a0A2E8943330b9e2C26BA3ccb954D9cF76c232",
                to_address="0xf508944ff0D5192B990B7Be13E12e474b8f3cA09",
                value=0.0,
                depth=0,
                type="CALL",
                gas=900064,
                gas_used=127110,
                error="execution reverted"
            ),
            InternalTransaction(
                from_address="0xf508944ff0D5192B990B7Be13E12e474b8f3cA09",
                to_address="0xCf3A7B2A3EEe85071d054e4C07f30974E8B22543",
                value=0.0,
                depth=1,
                type="STATICCALL",
                gas=848116,
                gas_used=2504,
                error=None
            ),
            InternalTransaction(
                from_address="0xf508944ff0D5192B990B7Be13E12e474b8f3cA09",
                to_address="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                value=0.0,
                depth=1,
                type="CALL",
                gas=840699,
                gas_used=12862,
                error=None
            ),
            InternalTransaction(
                from_address="0xf508944ff0D5192B990B7Be13E12e474b8f3cA09",
                to_address=None,
                value=0.0,
                depth=1,
                type="CREATE",
                gas=796205,
                gas_used=35845,
                error="execution reverted"
            ),
            InternalTransaction(
                from_address="0xB8d75C8d8e4a0fd2e6A36bA7b767fe8580a4C8ec",
                to_address="0xCf3A7B2A3EEe85071d054e4C07f30974E8B22543",
                value=0.0,
                depth=2,
                type="CALL",
                gas=783566,
                gas_used=504,
                error=None
            ),
            InternalTransaction(
                from_address="0xB8d75C8d8e4a0fd2e6A36bA7b767fe8580a4C8ec",
                to_address="0xCf3A7B2A3EEe85071d054e4C07f30974E8B22543",
                value=0.0,
                depth=2,
                type="CALL",
                gas=782844,
                gas_used=34887,
                error="execution reverted"
            ),
            InternalTransaction(
                from_address="0xCf3A7B2A3EEe85071d054e4C07f30974E8B22543",
                to_address="0xCe9dcc28791a98EDfd4175a7d55da9f86C560199",
                value=0.0,
                depth=3,
                type="CALL",
                gas=757416,
                gas_used=21205,
                error="execution reverted"
            )
        ],
        uniswap_v2_syncs=[],
        uniswap_v2_swaps=[],
        approvals=[],
        mints=[],
        burns=[],
        deposits=[],
        withdraws=[],
        pair_events=[],
        owner_events=[],
        contract_interactions=[],
        trading_enabled_events=[],
        trading_disabled_events=[],
        other_events=[],
        fees=TransactionFees(
            gas_price=36532968599,
            gas_used=127110,
            txn_fee=0.00464370563861889
        ),
        unique_addresses={
            "0x24a0A2E8943330b9e2C26BA3ccb954D9cF76c232",
            "0xf508944ff0D5192B990B7Be13E12e474b8f3cA09",
            "0xCf3A7B2A3EEe85071d054e4C07f30974E8B22543",
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
            "0xB8d75C8d8e4a0fd2e6A36bA7b767fe8580a4C8ec",
            "0xCe9dcc28791a98EDfd4175a7d55da9f86C560199"
        },
        erc20_contracts=set(),
        state_diffs={},
        latest_states={},
        bribe_amount=0.0,
        input=actual_input  # Use the actual input from transaction
    )
    
    # Test synchronous analysis
    sync_result = txn_analyzer.analyze_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    
    assert sync_result == expected_transaction

     # Test asynchronous analysis
    async def run_async_analysis():
        return await txn_analyzer.analyze_transaction_async(
            txn_data['transaction'],
            txn_data['receipt'],
            txn_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result == expected_transaction 


def test_transaction_bribe_amount(txn_analyzer, txn_data_fetcher):
    """Test analysis of a complex swap transaction with multiple internal transfers"""
    
    txn_hash = "0xc8e4638975eae8e711b6bdc0f62119d8a9a29a9c7a09c32b62b10274b512d916"
    bribe_amount = 0.01
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    # Test synchronous analysis
    sync_result = txn_analyzer.analyze_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )    
    # Verify specific aspects of the swap
    assert sync_result.bribe_amount == bribe_amount

    # Test async analysis
    async def run_async_analysis():
        return await txn_analyzer.analyze_transaction_async(
            txn_data['transaction'],
            txn_data['receipt'],
            txn_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result.bribe_amount == bribe_amount 

    