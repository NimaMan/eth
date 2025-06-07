"""
Test for analyzing Uniswap V3 add liquidity transactions

Objective: Verify correct parsing of Uniswap V3 add liquidity transaction including:
- Pool creation event
- Pool initialization
- Token transfers (WETH and HOODRAT)
- NFT position minting
- Liquidity addition
- Internal transactions
"""

import asyncio
from eth_block_processor.data_models.receipt_models import *
from eth_block_processor.data_models.trace_models import *
from eth_block_processor.data_models.txn_models import *
from eth_block_processor.data_models import *
import pytest


def test_add_liquidity_v3(txn_analyzer, txn_data_fetcher):
    """Test that both sync and async analysis match the expected add liquidity V3 details"""
    
    expected_add_liquidity = ProcessedTransaction(
        hash='0xb6550ffbe2bbca45edf1ffcdea957a57b4f83a9ac15e1f5d3af3fd032cda1482',
        block_number=21479418,
        block_timestamp=1735128851,
        txn_index=143,
        from_address='0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2',
        to_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
        contract_address=None,
        value=1.0,
        status=True,
        nonce=1626,
        txn_type='Multicall',
        actions=[],
        fees=TransactionFees(
            gas_price=4033803965,
            gas_used=5254070,
            txn_fee=0.02119388839838755
        ),
        bribe_amount=0.0,
        unique_addresses={
            '0xDee6cDd28Da9f51e3A8421395973894a884F3B2D',
            '0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc',
            '0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2',
            '0x0000000000000000000000000000000000000000',
            '0x1F98431c8aD98523631AE4a59f267346ea31F984',
            '0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
            '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2'
        },
        erc20_contracts={
            '0xDee6cDd28Da9f51e3A8421395973894a884F3B2D'
        },
        eth_transfers=[],
        erc20_transfers=[
            ERC20Transfer(
                token_address='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
                from_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                to_address='0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc',
                amount='1000000000000000000',
                log_index=293
            ),
            ERC20Transfer(
                token_address='0xDee6cDd28Da9f51e3A8421395973894a884F3B2D',
                from_address='0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2',
                to_address='0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc',
                amount='1605886042439757536366',
                log_index=294
            )
        ],
        erc721_transfers=[
            ERC721Transfer(
                token_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                from_address='0x0000000000000000000000000000000000000000',
                to_address='0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2',
                token_id='883296',
                log_index=297
            )
        ],
        erc1155_transfers=[],
        internal_transactions=[
            InternalTransaction(
                from_address='0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2',
                to_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                value=1.0,
                depth=0,
                type='CALL',
                gas=5379017,
                gas_used=5254070,
                error=None
            ),
            InternalTransaction(
                from_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                to_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                value=1.0,
                depth=1,
                type='DELEGATECALL',
                gas=5267259,
                gas_used=4651203,
                error=None
            ),
            InternalTransaction(
                from_address='0x1F98431c8aD98523631AE4a59f267346ea31F984',
                to_address='0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc',
                value=0.0,
                depth=3,
                type='CREATE2',
                gas=4986018,
                gas_used=4435593,
                error=None
            ),
            InternalTransaction(
                from_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                to_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                value=1.0,
                depth=1,
                type='DELEGATECALL',
                gas=687980,
                gas_used=650960,
                error=None
            ),
            InternalTransaction(
                from_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                to_address='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
                value=1.0,
                depth=4,
                type='CALL',
                gas=419994,
                gas_used=23974,
                error=None
            ),
            InternalTransaction(
                from_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                to_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                value=1.0,
                depth=1,
                type='DELEGATECALL',
                gas=46492,
                gas_used=233,
                error=None
            )
        ],
        uniswap_v2_syncs=[],
        uniswap_v2_swaps=[],
        approvals=[
            ERC20Approval(
                token_address='0xDee6cDd28Da9f51e3A8421395973894a884F3B2D',
                owner='0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2',
                spender='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                amount='115792089237316195423570985008687907853269984665640564037851697965473372103569',
                log_index=295
            )
        ],
        mints=[],
        burns=[],
        deposits=[
            DepositAction(
                id=None,
                token_address=None,
                withdrawal_address=None,
                amount='1000000000000000000',
                unlock_time=None,
                pair_address='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
                sender='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                log_index=292
            )
        ],
        withdraws=[],
        pair_events=[],
        owner_events=[],
        contract_creation_events=[],
        trading_enabled_events=[],
        trading_disabled_events=[],
        uniswap_v3_pools=[
            UniswapV3PoolCreated(
                token0='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
                token1='0xDee6cDd28Da9f51e3A8421395973894a884F3B2D',
                fee=10000,
                tick_spacing=200,
                pool='0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc',
                log_index=290
            )
        ],
        uniswap_v3_initializations=[
            UniswapV3Initialize(
                pool_address='0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc',
                sqrt_price_x96='3174950403365673188657906833916',
                tick='73818',
                log_index=291
            )
        ],
        uniswap_v3_burns=[],
        uniswap_v3_mints=[
            UniswapV3Mint(
                pool_address='0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc',
                sender='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                owner='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                tick_lower=-887200,
                tick_upper=887200,
                amount='40073507987693784378',
                amount0='1000000000000000000',
                amount1='1605886042439757536366',
                log_index=296
            )
        ],
        uniswap_v3_swaps=[],
        uniswap_v3_positions=[],
        uniswap_v3_increases=[
            UniswapV3IncreaseLiquidity(
                token_id='883296',
                liquidity='40073507987693784378',
                amount0='1000000000000000000',
                amount1='1605886042439757536366',
                pool_address='0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
                log_index=298
            )
        ],
        uniswap_v3_decreases=[],
        uniswap_v4_initializes=[],
        uniswap_v4_modifies=[],
        uniswap_v4_swaps=[],
        permit2_events=[],
        other_events=[],
        state_changes={},
        latest_states={},
        input='0xac9650d80000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000012000000000000000000000000000000000000000000000000000000000000002c0000000000000000000000000000000000000000000000000000000000000008413ead562000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000dee6cdd28da9f51e3a8421395973894a884f3b2d000000000000000000000000000000000000000000000000000000000000271000000000000000000000000000000000000002812d16b6323bb1f1f8acfd9fc000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000016488316456000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000dee6cdd28da9f51e3a8421395973894a884f3b2d0000000000000000000000000000000000000000000000000000000000002710ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff276600000000000000000000000000000000000000000000000000de0b6b3a7640000000000000000000000000000000000000000000000000570e254f40039f1c6e0000000000000000000000000000000000000000000000000dd7dd74c6d4c907000000000000000000000000000000000000000000000056d65c4d7ba5698a9c0000000000000000000000005ea17a4b7477b2bece0214a40723a2a09b2099d2000000000000000000000000000000000000000000000000000000000676bfdfa000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000412210e8a00000000000000000000000000000000000000000000000000000000'
    )

    txn_hash = "0xb6550ffbe2bbca45edf1ffcdea957a57b4f83a9ac15e1f5d3af3fd032cda1482"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    
    # Test synchronous analysis
    sync_result = txn_analyzer.process_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    assert sync_result == expected_add_liquidity
    
    # Test asynchronous analysis
    async def run_async_analysis():
        return await txn_analyzer.process_transaction_async(
            txn_data['transaction'],
            txn_data['receipt'],
            txn_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result == expected_add_liquidity

    # Fix for HOODRAT transfer decoding
    assert any(t.token_address == "0xDee6cDd28Da9f51e3A8421395973894a884F3B2D" 
               and t.amount == "1605886042439757536366" 
               for t in async_result.erc20_transfers) 