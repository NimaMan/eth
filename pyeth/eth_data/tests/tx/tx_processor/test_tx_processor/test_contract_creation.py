import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def test_contract_creation(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0x45fbb2326ee70cbaacb56c12b6a14b2ab5efd41635e9d3ba9ff4fed4eee52b89"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.tx_type == "Contract Creation"
    assert result.contract_address == "0x90f29ccD18c9181A9243EfF8f7546eef4b64994c"
    assert result.value == 2 * 10**18
    assert result.fees.tx_fee == 178697481904750746
    assert "Contract Creation" in result.actions
    assert "Ownership Change" in result.actions
    assert len(result.erc20_transfers) == 7
    assert result.uniswap_v2_pair_created_events and result.uniswap_v2_pair_created_events[0].pair_address == "0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2"
    assert result.ownership_transferred_events and result.ownership_transferred_events[0].new_owner == "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175"

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.contract_address == result.contract_address
    assert async_result.fees.tx_fee == result.fees.tx_fee
