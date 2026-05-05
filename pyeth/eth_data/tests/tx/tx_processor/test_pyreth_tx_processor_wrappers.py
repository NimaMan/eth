import asyncio

from eth_data.tx_processor.pyreth_tx_processor import PyRethTransactionProcessor
from eth_data.tx_processor.tx_batch_processor import TransactionBatchProcessor
from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher


class FakeTx:
    def __init__(self, tx_hash: str, tx_index: int = 0):
        self.hash = tx_hash
        self.tx_index = tx_index


class FakeRustTxProcessor:
    def process_transaction_from_hash_with_simulation(self, tx_hash: str):
        return FakeTx(tx_hash)

    def load_transaction_from_hash_db_only(self, tx_hash: str):
        return FakeTx(tx_hash)

    def process_transaction_hash_list(self, tx_hashes):
        return [FakeTx(tx_hash, idx) for idx, tx_hash in enumerate(tx_hashes)]


class FakeBlock:
    transactions = [
        FakeTx("0xbbb", 1),
        FakeTx("0xaaa", 0),
    ]


class FakeProvider:
    def process_block(self, block_number: int):
        assert block_number == 42
        return FakeBlock()


def test_transaction_processor_delegates_hash_processing_to_rust() -> None:
    processor = PyRethTransactionProcessor(
        tx_processor=FakeRustTxProcessor(),
        processed_tx_provider=FakeProvider(),
    )

    result = processor.process_transaction({"hash": "aaa"})

    assert result.hash == "0xaaa"


def test_transaction_batch_processor_uses_rust_block_provider() -> None:
    processor = TransactionBatchProcessor(
        tx_processor=FakeRustTxProcessor(),
        processed_tx_provider=FakeProvider(),
    )

    result = asyncio.run(processor.process_block_transactions(42))

    assert [tx.hash for tx in result] == ["0xaaa", "0xbbb"]


def test_transaction_data_fetcher_returns_hash_only_payload() -> None:
    payload = TransactionDataFetcher().get_transaction_data("0xabc")

    assert payload["transaction"] == {"hash": "0xabc"}
    assert payload["receipt"] == {"transactionHash": "0xabc"}
    assert payload["trace"] is None
