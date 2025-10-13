"""
Test for analyzing LP token approval transactions

Objective: Verify correct parsing of LP token approval transactions including:
- Approval event logs
- Maximum approval amount (uint256 max)
- Address normalization
- Gas fee calculation
"""

import asyncio
from eth_data.tx_processor.data_models.receipt_models import *
from eth_data.tx_processor.data_models.trace_models import *
from eth_data.tx_processor.data_models.tx_models import *

def test_approve_lp(tx_analyzer, tx_data_fetcher):
    """Test that both sync and async analysis match the expected approval details"""
    
    expected_approve = ProcessedTransaction(
        hash="0xc98c2b4ddc936ac6dba70bc7bacc9e37d406bcd24b2ecaf58a30f47521dbddf2",
        block_number=21423699,
        tx_index=156,
        from_address="0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
        to_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",
        contract_address=None,
        value=0.0,
        status=True,
        nonce=5,
        tx_type="Approval",
        approvals=[
            ERC20Approval(
                token_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",  # LP token
                owner="0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",          # Token owner
                spender="0xE2fE530C047f2d85298b07D9333C05737f1435fB",        # Staking contract
                amount=115792089237316195423570985008687907853269984665640564039457584007913129639935,  # uint256 max
                log_index=104
            )
        ],
        eth_transfers=[],
        erc20_transfers=[],
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
            gas_price=33931291438,
            gas_used=46386,
            tx_fee=0.001573936884643068
        ),
        unique_addresses={
            "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",  # Token owner
            "0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",  # LP token
            "0xE2fE530C047f2d85298b07D9333C05737f1435fB"   # Staking contract
        },
        erc20_contracts={
            "0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2"   # LP token
        },
        address_balance_changes={},
        latest_states={},
        bribe_amount=0.0,
        input="0x095ea7b3000000000000000000000000e2fe530c047f2d85298b07d9333c05737f1435fbffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    )

    tx_hash = "0xc98c2b4ddc936ac6dba70bc7bacc9e37d406bcd24b2ecaf58a30f47521dbddf2"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)
    
    # Test synchronous analysis
    sync_result = tx_analyzer.process_transaction(
        tx_data['transaction'],
        tx_data['receipt'],
        tx_data['trace']
    )
    
    assert sync_result == expected_approve
    
    # Test asynchronous analysis
    async def run_async_analysis():
        return await tx_analyzer.process_transaction_async(
            tx_data['transaction'],
            tx_data['receipt'],
            tx_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result == expected_approve
