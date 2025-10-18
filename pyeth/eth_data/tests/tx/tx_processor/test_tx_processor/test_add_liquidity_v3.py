import asyncio

from eth_data.tx_processor.tx_processor import TransactionProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


def _collect_transfer_summary(transfers):
    return {(t.token_address, t.from_address, t.to_address): t.amount for t in transfers}


def test_add_liquidity_v3_round_trip(tx_analyzer: TransactionProcessor, tx_data_fetcher: TransactionDataFetcher):
    tx_hash = "0xb6550ffbe2bbca45edf1ffcdea957a57b4f83a9ac15e1f5d3af3fd032cda1482"
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    result = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    assert result.hash == tx_hash
    assert result.tx_type == "Multicall"
    assert result.value == 10**18  # 1 WETH contributed

    transfer_summary = _collect_transfer_summary(result.erc20_transfers)
    assert transfer_summary[
        (
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
            "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
            "0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
        )
    ] == 10**18
    assert transfer_summary[
        (
            "0xDee6cDd28Da9f51e3A8421395973894a884F3B2D",
            "0x5eA17A4b7477b2bECe0214A40723a2A09b2099D2",
            "0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc",
        )
    ] == 1605886042439757536366

    assert len(result.erc721_transfers) == 1
    assert result.erc721_transfers[0].token_id == 883296

    assert len(result.uniswap_v3_mints) == 1
    mint = result.uniswap_v3_mints[0]
    assert mint.amount0 == 10**18
    assert mint.amount1 == 1605886042439757536366
    assert mint.tick_lower == -887200
    assert mint.tick_upper == 887200

    assert len(result.deposit_events) == 1
    assert result.deposit_events[0].amount == 10**18

    async def run_async():
        return await tx_analyzer.process_transaction_async(
            tx_data["transaction"],
            tx_data["receipt"],
            tx_data["trace"],
        )

    async_result = asyncio.run(run_async())
    assert async_result.uniswap_v3_mints == result.uniswap_v3_mints
    assert async_result.hash == result.hash
