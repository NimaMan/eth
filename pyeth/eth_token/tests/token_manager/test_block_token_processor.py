from eth_token.token_manager.block_token_processor import BlockTokenProcessor
from eth_token.token_manager.block_token_processor import HistoricalBlockTokenProcessor


def test_metadata_nonce_too_high_is_retryable() -> None:
    processor = BlockTokenProcessor.__new__(BlockTokenProcessor)

    assert processor._is_retryable_metadata_error(
        RuntimeError("transaction validation error: nonce 1 too high, expected 0")
    )


def test_token_processor_accepts_pyreth_block_shape() -> None:
    class FakeTx:
        def __init__(self, tx_index: int) -> None:
            self.tx_index = tx_index

        def to_dict(self):
            return {
                "hash": f"0x{self.tx_index}",
                "tx_index": self.tx_index,
                "from_address": "0x0000000000000000000000000000000000000001",
                "unique_addresses": [],
                "erc20_contracts": [],
            }

    class FakeBlock:
        number = 123
        hash = "0xblock"
        parent_hash = "0xparent"
        timestamp = 456
        gas_used = 789
        gas_limit = 1000
        base_fee_per_gas = "42"
        transactions = [FakeTx(2), FakeTx(0), {"hash": "0x1", "tx_index": 1}]

    transactions = BlockTokenProcessor._get_block_transactions(FakeBlock())

    assert [tx["tx_index"] if isinstance(tx, dict) else tx.tx_index for tx in transactions] == [0, 1, 2]


def test_historical_processor_uses_pyreth_adapter_when_provider_is_supplied() -> None:
    class FakeTokenProcessor:
        processed_blocks = {}

    class FakeProvider:
        pass

    processor = HistoricalBlockTokenProcessor(
        block_token_processor=FakeTokenProcessor(),
        processed_tx_provider=FakeProvider(),
    )

    assert processor.block_processor is not None
