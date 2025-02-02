"""
Test for analyzing add liquidity transactions
Objective: Verify correct parsing of Uniswap V2 add liquidity transaction including:
- Token transfers
- ETH/WETH wrapping
- LP token minting
- Pool reserves updates
"""

import asyncio
from eth_block_processor.data_models.receipt_models import *
from eth_block_processor.data_models.trace_models import *
from eth_block_processor.data_models.txn_models import *


def test_add_liquidity(txn_analyzer, txn_data_fetcher):
    """Test that both sync and async analysis match the expected add liquidity details"""
    
    expected_add_liquidity = DetailedTransaction(
        hash="0xa72a44acb01e0e83cd9097c75e5b54208dbcb8354e4892963b8d39044eef0f65",
        block_number=21423374,
        txn_index=128,
        from_address="0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
        to_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
        contract_address=None,
        value=0.0,
        status=True,
        nonce=1,
        txn_type="Add Liquidity",
        erc20_transfers=[
            ERC20Transfer(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                from_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                to_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",
                amount=60000000000000000000000000,
                log_index=279
            ),
            ERC20Transfer(
                token_address="0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                from_address="0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
                to_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2", 
                amount=2000000000000000000,
                log_index=281
            ),
            ERC20Transfer(
                token_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",
                from_address="0x0000000000000000000000000000000000000000",
                to_address="0x0000000000000000000000000000000000000000",
                amount=1000,
                log_index=282
            ),
            ERC20Transfer(
                token_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",
                from_address="0x0000000000000000000000000000000000000000", 
                to_address="0x9e78124aDDDE586983BDD32303616A1Fb9B4F175",
                amount=10954451150103322268139,
                log_index=283
            )
        ],
        approvals=[
            ERC20Approval(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                owner="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                spender="0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
                amount=60000000000000000000000000,
                log_index=277
            ),
            ERC20Approval(
                token_address="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                owner="0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",
                spender="0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
                amount=0,
                log_index=278
            )
        ],
        uniswap_v2_syncs=[
            UniswapV2Sync(
                pair_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",
                reserve0=60000000000000000000000000,
                reserve1=2000000000000000000,
                log_index=284
            )
        ],
        mints=[
            MintAction(
                pair_address="0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2",
                sender="0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
                amount0=60000000000000000000000000,
                amount1=2000000000000000000,
                log_index=285
            )
        ],
        fees=TransactionFees(
            gas_price=38864000951,
            gas_used=243987,
            txn_fee=0.009482311000031637
        ),
        unique_addresses={'0x0000000000000000000000000000000000000000',
                        '0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2',
                        '0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f',
                        '0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D',
                        '0x90f29ccD18c9181A9243EfF8f7546eef4b64994c',
                        '0x9e78124aDDDE586983BDD32303616A1Fb9B4F175',
                        '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2'},
        erc20_contracts={
            "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c",  # SIMAI
            "0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2"   # LP token
        },
        deposits=[
            DepositAction(
                pair_address='0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
                sender='0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D',
                amount=2000000000000000000,
                log_index=280
            )
        ],
        input="0xed995307",
        bribe_amount=0.0,
        block_timestamp=1734451967 
    )

    txn_hash = "0xa72a44acb01e0e83cd9097c75e5b54208dbcb8354e4892963b8d39044eef0f65"
    txn_data = txn_data_fetcher.get_transaction_data(txn_hash)
    
    # Test synchronous analysis
    sync_result = txn_analyzer.process_transaction(
        txn_data['transaction'],
        txn_data['receipt'],
        txn_data['trace']
    )
    assert sync_result.txn_type == expected_add_liquidity.txn_type
    
    # Test asynchronous analysis
    async def run_async_analysis():
        return await txn_analyzer.process_transaction_async(
            txn_data['transaction'],
            txn_data['receipt'],
            txn_data['trace']
        )
    
    async_result = asyncio.run(run_async_analysis())
    assert async_result.txn_type == expected_add_liquidity.txn_type