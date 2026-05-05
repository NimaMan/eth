import asyncio

from eth_data.tx_provider import (
    ProcessedTransactionProvider,
    ProcessedTxProvider,
    RustProcessedTransactionProvider,
)


class FakeTx:
    def __init__(self, tx_hash: str, tx_index: int = 0):
        self.hash = tx_hash
        self.tx_index = tx_index


class FakeBlock:
    transactions = [
        {"hash": "0xccc", "tx_index": 2},
        FakeTx("0xaaa", 0),
        FakeTx("0xbbb", 1),
    ]


class FakeProcessedTxProvider:
    def process_block(self, block_number: int):
        assert block_number == 42
        return FakeBlock()


class FakeRustTxProcessor:
    def process_transaction_from_hash_with_simulation(self, tx_hash: str):
        return FakeTx(tx_hash)

    def process_transaction_hash_list(self, tx_hashes):
        return [FakeTx(tx_hash, idx) for idx, tx_hash in enumerate(tx_hashes)]


class FakeTxMetaDataFetcher:
    def get_tx_hashes_and_blocks_for_address(self, address, start_block, end_block, num_blocks):
        assert address == "0xabc"
        return [("0xaaa", 42), ("0xbbb", 42)]

    def close(self):
        self.closed = True


def build_provider():
    return ProcessedTransactionProvider(
        tx_processor=FakeRustTxProcessor(),
        processed_tx_provider=FakeProcessedTxProvider(),
        tx_meta_data_fetcher=FakeTxMetaDataFetcher(),
    )


def test_provider_uses_single_rust_block_provider_and_caches_by_hash() -> None:
    provider = build_provider()

    transactions = provider.get_block_transactions(42)

    assert [tx["hash"] if isinstance(tx, dict) else tx.hash for tx in transactions] == [
        "0xaaa",
        "0xbbb",
        "0xccc",
    ]
    assert list(provider.processed_block_cache) == [42]
    assert sorted(provider.processed_tx_cache) == ["0xaaa", "0xbbb", "0xccc"]


def test_provider_gets_transaction_batches_from_rust_tx_processor() -> None:
    provider = build_provider()

    result = asyncio.run(provider.get_processed_transactions_from_tx_hashes(["aaa", "0xbbb"]))

    assert sorted(result) == ["0xaaa", "0xbbb"]
    assert [tx.hash for tx in result.values()] == ["0xaaa", "0xbbb"]


def test_legacy_provider_names_are_aliases_to_canonical_provider() -> None:
    assert ProcessedTxProvider is ProcessedTransactionProvider
    assert RustProcessedTransactionProvider is ProcessedTransactionProvider
