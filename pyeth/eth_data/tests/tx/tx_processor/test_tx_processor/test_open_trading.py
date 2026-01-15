import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def test_open_trading(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0x9fc6130629c69e689d6023ffb2cfbfcd7df18210e2ba97d527c9b93819c6ec5b"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.tx_type == "Trading Enabled"
    assert result.fees.tx_fee == 1532972864860635
    assert result.trading_enabled_events == [
        result.trading_enabled_events[0]
    ]  # list contains single TradingEnabledEvent
    assert result.trading_enabled_events[0].token_address == "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.trading_enabled_events == result.trading_enabled_events
