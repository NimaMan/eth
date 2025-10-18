import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def test_eth_transfer_analysis(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0x0d6a7c23ba11f31a01cab82d8ae0b770286d7a189828e57d3f871bed4e0480f3"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.tx_type == "Ether Transfer"
    assert result.value == 2400000000000000000
    assert result.fees.tx_fee == 883470208026000
    assert len(result.eth_transfers) == 1
    transfer = result.eth_transfers[0]
    assert transfer.amount == result.value

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.eth_transfers == result.eth_transfers
