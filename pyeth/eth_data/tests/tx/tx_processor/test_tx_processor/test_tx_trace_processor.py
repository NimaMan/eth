import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def test_failed_contract_creation(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0x8304000190747e7f8ace1510304ada10d66ec24e05427dc8f994ddbea3b97d6c"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.hash == tx_hash
    assert result.status is False
    assert result.fees.tx_fee == 4643705638618890
    assert result.tx_type == "Contract Interaction"
    assert any(
        tx.trace_type == "CREATE" and tx.error == "execution reverted"
        for tx in result.internal_transactions
    )

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.status is False
    assert async_result.fees.tx_fee == result.fees.tx_fee
    assert async_result.internal_transactions == result.internal_transactions


def test_transaction_bribe_amount(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0xc8e4638975eae8e711b6bdc0f62119d8a9a29a9c7a09c32b62b10274b512d916"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.bribe_amount == 10**16

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.bribe_amount == result.bribe_amount
