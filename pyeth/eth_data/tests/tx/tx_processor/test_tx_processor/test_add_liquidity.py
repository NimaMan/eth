import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def test_add_liquidity(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0xa72a44acb01e0e83cd9097c75e5b54208dbcb8354e4892963b8d39044eef0f65"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.tx_type == "Add Liquidity"
    assert len(result.erc20_transfers) == 4
    amounts = [t.amount for t in result.erc20_transfers]
    assert 60000000000000000000000000 in amounts
    assert 2000000000000000000 in amounts
    assert result.fees.tx_fee == 9482311000031637

    assert result.uniswap_v2_mints and result.uniswap_v2_mints[0].amount0 == 60000000000000000000000000
    assert result.deposit_events and result.deposit_events[0].amount == 2000000000000000000

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.uniswap_v2_mints == result.uniswap_v2_mints
