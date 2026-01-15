"""
Regression test for PyReth processed_transaction_by_hash.

Requires a locally synced Reth dataset with block 23700867 present.
"""

import pytest

TARGET_TX_HASH = "0x134939eab652b27b4517a96416992bb3aa34b967008a60efd220839acfea1fc6"


def _require_pyreth() -> "pyreth.PyReth":
    if pyreth is None:
        pytest.skip(f"pyreth unavailable: {_pyreth_import_error}")
    try:
        return pyreth.PyReth()
    except Exception as exc:  # pragma: no cover - runtime guard
        pytest.skip(f"PyReth could not be initialised: {exc}")


@pytest.mark.integration
@pytest.mark.xfail(
    reason="processed_transaction_by_hash still fails to replay earlier block txs (nonce mismatch)",
    raises=RuntimeError,
)
def test_processed_transaction_by_hash_replays_block_prefix():
    reth = _require_pyreth()
    provider = reth.processed_tx_provider()

    processed = provider.processed_transaction_by_hash(TARGET_TX_HASH)
    assert processed.hash.lower() == TARGET_TX_HASH.lower()
