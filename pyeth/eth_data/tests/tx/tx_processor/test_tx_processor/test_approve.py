import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def test_approve_lp(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0xc98c2b4ddc936ac6dba70bc7bacc9e37d406bcd24b2ecaf58a30f47521dbddf2"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.tx_type == "Approval"
    assert result.fees.tx_fee == 1573936884643068

    assert len(result.erc20_approval_events) == 1
    approval = result.erc20_approval_events[0]
    assert approval.token_address == "0x0341Bc2f4Ee5ccc7558e0e2aD1c9C682c95512B2"
    assert approval.owner == "0x9e78124aDDDE586983BDD32303616A1Fb9B4F175"
    assert approval.spender == "0xE2fE530C047f2d85298b07D9333C05737f1435fB"
    assert approval.amount == 115792089237316195423570985008687907853269984665640564039457584007913129639935

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.erc20_approval_events == result.erc20_approval_events
