"""
Regression test for access list preservation during prior transaction replay.

This ensures that the pool buy/sell simulator can successfully re-simulate
transaction 0x134939… when its access list is preserved. The test is skipped
automatically if PyReth cannot connect to the local Reth dataset.
"""

from __future__ import annotations

import pytest


try:
    from pyreth import (  # type: ignore
        PoolBuySellParameters,
        block_processor,
        pool_buy_sell_simulator,
    )
except Exception as exc:  # pragma: no cover - environment specific
    PoolBuySellParameters = None  # type: ignore[assignment]  # pragma: no cover
    block_processor = None  # type: ignore[assignment]  # pragma: no cover
    pool_buy_sell_simulator = None  # type: ignore[assignment]  # pragma: no cover
    _pyreth_import_error = exc  # pragma: no cover
else:
    _pyreth_import_error = None


TX_HASH = "0x134939eab652b27b4517a96416992bb3aa34b967008a60efd220839acfea1fc6"
TOKEN_ADDRESS = "0x83410f3E63fC34AF02d6550F2C3aBB5492D71422"
POOL_ADDRESS = "0x0895Dd4a9aAB7A4457F34a6268A0EC95Fe7B2f16"
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"


def _require_pyreth() -> None:
    if block_processor is None:
        pytest.skip(f"pyreth unavailable: {_pyreth_import_error}")
    try:
        block_processor()
    except Exception as exc:  # pragma: no cover - runtime guard
        pytest.skip(f"PyReth could not be initialised: {exc}")


@pytest.mark.integration
def test_prior_replay_preserves_access_list():
    """Prior transaction replay succeeds when access list is retained."""
    _require_pyreth()
    simulator = pool_buy_sell_simulator()
    provider = block_processor()

    prior = provider.processed_transaction_by_hash(TX_HASH)

    params = PoolBuySellParameters.with_denom_amount(0.05, 18, 18)
    params.block_number = int(prior.block_number) - 1
    params.token_decimals = 18
    params.denom_decimals = 18
    params.denom_address = WETH_ADDRESS
    params.set_prior_transactions([prior])

    result = simulator.check_uniswap_v2_pool(TOKEN_ADDRESS, POOL_ADDRESS, params)

    assert result.prior_transactions, "expected prior replay information"
    assert result.prior_transactions[0].status is True, "prior replay should succeed"
    assert not result.error_message, f"expected no error, got {result.error_message!r}"
