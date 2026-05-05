from eth_token.token_manager.block_token_processor import BlockTokenProcessor


def test_metadata_nonce_too_high_is_retryable() -> None:
    processor = BlockTokenProcessor.__new__(BlockTokenProcessor)

    assert processor._is_retryable_metadata_error(
        RuntimeError("transaction validation error: nonce 1 too high, expected 0")
    )
