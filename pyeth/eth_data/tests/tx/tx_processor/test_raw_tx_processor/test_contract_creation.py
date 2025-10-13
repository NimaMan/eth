"""Test contract creation transaction analysis"""

import pytest
import asyncio
import numpy as np
from eth_data.tx_processor.data_models.tx_models import *

@pytest.fixture
def expected_contract_creation():
    """Expected DetailedTransaction object for the SIMAI token creation"""
    return ProcessedTransaction(
        hash='0x45fbb2326ee70cbaacb56c12b6a14b2ab5efd41635e9d3ba9ff4fed4eee52b89',
        block_number=21423372,
        block_timestamp=1734451943,
        tx_index=239,
        from_address='0x9e78124aDDDE586983BDD32303616A1Fb9B4F175',
        to_address=None,
        contract_address='0x90f29ccD18c9181A9243EfF8f7546eef4b64994c',
        value=2.0,
        status=True,
        nonce=0,
        tx_type='Contract Creation',
        actions=('Contract Creation', 'Ownership Change'),
        fees=TransactionFees(gas_price=38919347153, gas_used=4591482, tx_fee=0.17869748190475074),
        bribe_amount=0.0,
        unique_addresses={'0x9AE4e9778D8d662462729fbA3F79f4A0c66B0F0c', '0xA7d9F0e487664e57Ebb4A4B8d1d2667A7d7EA307', '0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2', '0x9e78124aDDDE586983BDD32303616A1Fb9B4F175', '0x02d1966AB06F1b1D3Cb11AAfd301eAa0fE437cC2', '0x4Cf62112ba1541ffc84F53B2f4889d22f4d5a409', '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2', '0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f', '0x65a53ac26bd12F3A7D5f7083eF231BBBE852eF1C', '0x90f29ccD18c9181A9243EfF8f7546eef4b64994c', '0x0000000000000000000000000000000000000000', '0xc849543Ea151Eed47a7C9C89BAE7783b95016A3E'},
        erc20_contracts={'0x90f29ccD18c9181A9243EfF8f7546eef4b64994c'},
        eth_transfers=[],
        erc20_transfers=[
            # Initial token distributions
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0x65a53ac26bd12F3A7D5f7083eF231BBBE852eF1C",
                amount='10000000000000000000000000',
                log_index=455
            ),
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0x9AE4e9778D8d662462729fbA3F79f4A0c66B0F0c",
                amount='6000000000000000000000000',
                log_index=456
            ),
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0xc849543Ea151Eed47a7C9C89BAE7783b95016A3E",
                amount='8000000000000000000000000',
                log_index=457
            ),
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0x4Cf62112ba1541ffc84F53B2f4889d22f4d5a409",
                amount='8000000000000000000000000',
                log_index=458
            ),
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0xA7d9F0e487664e57Ebb4A4B8d1d2667A7d7EA307",
                amount='4000000000000000000000000',
                log_index=459
            ),
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0x02d1966AB06F1b1D3Cb11AAfd301eAa0fE437cC2",
                amount='4000000000000000000000000',
                log_index=460
            ),
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                amount='60000000000000000000000000',
                log_index=461
            )
        ],
        erc721_transfers=[],
        erc1155_transfers=[],
        internal_transactions=[InternalTransaction(from_address='0x9e78124aDDDE586983BDD32303616A1Fb9B4F175', to_address='0x90f29ccD18c9181A9243EfF8f7546eef4b64994c', value=2.0, depth=0, type='CREATE', gas=4627084, gas_used=4591482, error=None), InternalTransaction(from_address='0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f', to_address='0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2', value=0.0, depth=2, type='CREATE2', gas=3720562, gas_used=2327803, error=None)],
        uniswap_v2_syncs=[],
        uniswap_v2_swaps=[],
        approvals=[],
        mints=[],
        burns=[],
        deposits=[],
        withdraws=[],
        pair_events=[PairAction(pair_address='0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2', token0='0x90f29ccD18c9181A9243EfF8f7546eef4b64994c', token1='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2', log_index=462)],
        owner_events=[OwnerEvent(contract_address='0x90f29ccD18c9181A9243EfF8f7546eef4b64994c', previous_owner='0x0000000000000000000000000000000000000000', new_owner='0x9e78124aDDDE586983BDD32303616A1Fb9B4F175', log_index=454)],
        contract_creation_events=[ContractCreationEvent(contract_address='0x90f29ccD18c9181A9243EfF8f7546eef4b64994c', contract_type='ERC-20', symbol='SIMAI', decimals=18, name='Simian AI', total_supply=100000000.0)],
        trading_enabled_events=[],
        trading_disabled_events=[],
        other_events=[],
        latest_states={},
        input='0x'
    )

def test_contract_creation(tx_analyzer, tx_data_fetcher, expected_contract_creation):
    """Test that both sync and async analysis match the expected contract creation details"""
    
    tx_hash = "0x45fbb2326ee70cbaacb56c12b6a14b2ab5efd41635e9d3ba9ff4fed4eee52b89"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)
    
    # Test synchronous analysis
    sync_result = tx_analyzer.process_transaction(
        tx_data['transaction'],
        tx_data['receipt'],
        tx_data['trace']
    )
    # Debug differences
    # for key in sync_result.__dict__:
    #     if key == "input":
    #         continue
    #     expected_val = getattr(expected_contract_creation, key, None)
    #     actual_val = getattr(sync_result, key)
    #     if expected_val != actual_val:
    #         print(f"\\nDifference in {key}:")
    #         print(f"Expected: {expected_val}")
    #         print(f"Actual  : {actual_val}")
    assert sync_result == expected_contract_creation
    
    # Test asynchronous analysis
    async def run_async_analysis():
        return await tx_analyzer.process_transaction_async(
            tx_data['transaction'],
            tx_data['receipt'],
            tx_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result == expected_contract_creation 