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
from eth_block_processor.data_models.uniswap_v3_models import *


def test_add_liquidity_v3(txn_analyzer, txn_data_fetcher):
    """Test that both sync and async analysis match the expected add liquidity V3 details"""
    
    expected_add_liquidity = DetailedTransaction(
        hash="0xb6550ffbe2bbca45edf1ffcdea957a57b4f83a9ac15e1f5d3af3fd032cda1482",
        block_number=21479418,
        block_timestamp=1735128851,
        txn_index=143,
        from_address="0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2",
        to_address="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
        contract_address=None,
        value=1.0,
        status=True,
        nonce=1626,
        txn_type="Multicall",
        actions=[], # TODO: Add Liquidity ndassdseed to be added
        erc20_transfers=[
            ERC20Transfer(
                token_address="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                from_address="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                to_address="0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
                amount=1000000000000000000,
                log_index=293
            ),
            ERC20Transfer(
                token_address="0xDee6cDd28Da9f51e3A8421395973894a884F3B2D",
                from_address="0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2",
                to_address="0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
                amount=1605886042439757536366,
                log_index=294
            ),
            ERC20Transfer(
                token_address="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2",
                amount="0",
                log_index=297
            )
        ],
        approvals=[
            ERC20Approval(
                token_address="0xDee6cDd28Da9f51e3A8421395973894a884F3B2D",
                owner="0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2",
                spender="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                amount=115792089237316195423570985008687907853269984665640564037851697965473372103569,
                log_index=295
            )
        ],
        deposits=[
            DepositAction(
                pair_address="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                sender="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                amount=1000000000000000000,
                log_index=292
            )
        ],
        uniswap_v3_pools=[
            UniswapV3PoolCreated(
                token0="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                token1="0xDee6cDd28Da9f51e3A8421395973894a884F3B2D",
                fee=10000,
                tick_spacing=200,
                pool="0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
                log_index=290
            )
        ],
        uniswap_v3_initializations=[
            UniswapV3Initialize(
                pool_address="0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
                sqrt_price_x96=3174950403365673188657906833916,
                tick=73818,
                log_index=291
            )
        ],
        uniswap_v3_mints=[
            UniswapV3Mint(
                pool_address="0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
                sender="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                owner="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                tick_lower=-887200,
                tick_upper=887200,
                amount=40073507987693784378,
                amount0=1000000000000000000,
                amount1=1605886042439757536366,
                log_index=296
            )
        ],
        uniswap_v3_positions=[],
        uniswap_v3_increases=[
            UniswapV3IncreaseLiquidity(
                token_id=883296,
                liquidity=40073507987693784378,
                amount0=1000000000000000000,
                amount1=1605886042439757536366,
                pool_address="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                log_index=298
            )
        ],
        internal_transactions=[
            InternalTransaction(
                from_address="0x1f98431c8ad98523631ae4a59f267346ea31f984",
                to_address="0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
                value=0.0,
                depth=2,
                type="CREATE2",
                gas=4986018,
                gas_used=None,
                error=None
            ),
            InternalTransaction(
                from_address="0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
                to_address="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                value=1.0,
                depth=2,
                type="CALL",
                gas=419994,
                gas_used=None,
                error=None
            )
        ],
        fees=TransactionFees(
            gas_price=4033803965,
            gas_used=5254070,
            txn_fee=0.02119388839838755
        ),
        unique_addresses={
            "0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2",
            "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
            "0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
            "0xDee6cDd28Da9f51e3A8421395973894a884F3B2D",
            "0x1F98431c8aD98523631AE4a59f267346ea31F984",
            "0x0000000000000000000000000000000000000000"
        },
        erc20_contracts={
            "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
            "0xDee6cDd28Da9f51e3A8421395973894a884F3B2D"
        },
        input="0xac9650d8...",
        bribe_amount=0.0,
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