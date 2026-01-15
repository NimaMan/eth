import pytest
from web3 import Web3

from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
from eth_data.tx_processor.tx_log_processor import TransactionLogProcessor


@pytest.fixture
def log_processor(w3):
    return TransactionLogProcessor(w3)


@pytest.fixture
def tx_fetcher(w3):
    return TransactionDataFetcher(w3)


def _process_receipt_logs(log_processor: TransactionLogProcessor, receipt) -> dict:
    logs = [dict(log) for log in receipt["logs"]]
    return log_processor.process_logs(logs)


def test_process_logs_uniswap_v2(log_processor, tx_fetcher):
    """
    Real mainnet add-liquidity transaction that emits Uniswap V2 events
    (hash observed in existing integration tests).
    """
    tx_hash = "0xf292850dd459fe39a7692077b88112b2771ccaa427e20ed611d8a377fcbd6c66"
    txn_data = tx_fetcher.get_transaction_data(tx_hash)

    result = _process_receipt_logs(log_processor, txn_data["receipt"])

    assert result["uniswap_v2_mints"], "Expected Uniswap V2 mint events"
    assert result["uniswap_v2_syncs"], "Expected Uniswap V2 sync events"
    assert result["erc20_approval_events"], "Expected ERC20 approvals"
    assert any(event.pair_address == "0x52FD104d206EE67f5d26bb66F64781aF887B2Eb0" for event in result["uniswap_v2_mints"])


def test_process_logs_uniswap_v3(log_processor, tx_fetcher):
    """
    Real mainnet multicall transaction that emits Uniswap V3 mint/increase events.
    """
    tx_hash = "0xb6550ffbe2bbca45edf1ffcdea957a57b4f83a9ac15e1f5d3af3fd032cda1482"
    txn_data = tx_fetcher.get_transaction_data(tx_hash)

    result = _process_receipt_logs(log_processor, txn_data["receipt"])

    assert result["uniswap_v3_mints"], "Expected Uniswap V3 mint events"
    assert result["uniswap_v3_increases"], "Expected Uniswap V3 increase liquidity events"
    assert result["erc721_transfers"], "Expected NFT position transfer"
    assert any(event.pool_address == "0xd71a4cb350d5aC4b6DA5e7f8Dd13702dC2A813Bc" for event in result["uniswap_v3_mints"])


def test_process_logs_permit2_event(log_processor, tx_fetcher, w3):
    """
    Ensure Permit2 events classify correctly by discovering a recent transaction on-chain.
    """
    signature = "0xc6a377bfc4eb120024a8ac08eef205be16b817020812c73223e81d1bdb9708ec"
    contract_address = "0x000000000022d473030f116ddee9f6b43ac78ba3"
    contract_checksum = Web3.to_checksum_address(contract_address)

    latest_block = w3.eth.block_number
    search_window = 200_000  # scan ~200k blocks back (~1 month on mainnet)
    chunk_size = 20_000

    tx_hash = None
    to_block = latest_block
    from web3.exceptions import TransactionNotFound

    while to_block >= max(latest_block - search_window, 0):
        from_block = max(to_block - chunk_size + 1, 0)
        logs = w3.eth.get_logs(
            {
                "address": contract_checksum,
                "topics": [signature],
                "fromBlock": from_block,
                "toBlock": to_block,
            }
        )
        if logs:
            tx_hash = logs[0]["transactionHash"].hex()
            break
        to_block = from_block - 1

    if not tx_hash:
        pytest.skip("No Permit2 events discovered in the configured search window")

    txn_data = tx_fetcher.get_transaction_data(tx_hash)
    result = _process_receipt_logs(log_processor, txn_data["receipt"])

    assert result["permit2_events"], "Permit2 events should be classified"
    permit_event = result["permit2_events"][0]
    assert permit_event.pool_manager_address == contract_checksum
