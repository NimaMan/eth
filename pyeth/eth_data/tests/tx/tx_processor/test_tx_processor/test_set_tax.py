import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def test_set_tax(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0x25d4b79545273e46a2685247138c38ce71ace2ec1398fa5b7a547b55f22efe77"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.tx_type == "Set Tax"
    assert result.fees.tx_fee == 1332073192831920
    assert not result.trading_enabled_events
    assert not result.trading_disabled_events

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.fees.tx_fee == result.fees.tx_fee
