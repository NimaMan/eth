"""
Schema alignment regression: compare Python vs PyReth processed transactions.

This test exercises the shared ProcessedTransaction schema by running a real
transaction through both the pure-Python TransactionProcessor and the Rust
PyReth TxProcessor. It casts the PyReth output into the same ProcessedTransaction
dataclass and asserts that key fields (status, fees, ERC20 transfers, internal
calls, etc.) are identical. The goal is to prevent regressions while the
Rust↔Python bridge stabilises.
"""

from typing import Iterable, Sequence, Tuple

import pytest

from eth_data.tx_processor.data_models.tx_models import ProcessedTransaction

pyreth = pytest.importorskip("pyreth")


# Mainnet transaction chosen because it exercises ERC20 transfers, internal calls,
# and non-zero bribes while being used elsewhere in the test-suite.
PARITY_TX_HASHES: Sequence[str] = (
    # KERMIT swap with multiple transfers + internal execution path
    "0xf403b3d19a6e83ddb04e7755cdc5122e1812b5b7b7df69528ce1d202f42e22b5",
)


@pytest.fixture(scope="session")
def pyreth_tx_processor():
    """Initialise the shared PyReth TxProcessor singleton once for the test session."""
    core = pyreth.PyReth()
    return core.tx_processor()


def _project_erc20(transfers: Iterable) -> Tuple[Tuple[str, str, str, int, int], ...]:
    """Project ERC20Transfer objects to comparable tuples."""
    normalised = []
    for t in transfers:
        normalised.append(
            (
                t.token_address.lower(),
                t.from_address.lower(),
                t.to_address.lower(),
                int(t.amount),
                int(getattr(t, "log_index", -1)),
            )
        )
    return tuple(sorted(normalised))


def _project_internal(
    internals: Iterable,
) -> Tuple[Tuple[str, str, int, int, int, int, str, str], ...]:
    """Project InternalTransaction objects to comparable tuples."""
    normalised = []
    for call in internals:
        normalised.append(
            (
                call.from_address.lower(),
                (call.to_address or "").lower(),
                int(call.value),
                int(call.gas),
                int(call.gas_used),
                int(call.depth),
                call.trace_type,
                (call.call_type or "") if getattr(call, "call_type", None) else "",
            )
        )
    return tuple(sorted(normalised))


def _assert_core_fields(python_tx: ProcessedTransaction, pyreth_tx: ProcessedTransaction) -> None:
    """Assert core scalar fields match between the two processed transactions."""
    assert bool(python_tx.status) == bool(pyreth_tx.status), "status mismatch"
    assert int(python_tx.value) == int(pyreth_tx.value), "value mismatch (wei)"
    assert int(python_tx.bribe_amount) == int(pyreth_tx.bribe_amount), "bribe mismatch (wei)"
    assert python_tx.tx_type == pyreth_tx.tx_type
    assert set(python_tx.actions) == set(pyreth_tx.actions)

    # Fees: compare primary gas metrics; optional fee caps may be None.
    assert int(python_tx.fees.gas_price) == int(pyreth_tx.fees.gas_price)
    assert int(python_tx.fees.gas_used) == int(pyreth_tx.fees.gas_used)
    assert int(python_tx.fees.tx_fee) == int(pyreth_tx.fees.tx_fee)

    assert python_tx.fees.protocol_type == pyreth_tx.fees.protocol_type
    assert python_tx.fees.max_fee_per_gas == pyreth_tx.fees.max_fee_per_gas
    assert python_tx.fees.max_priority_fee == pyreth_tx.fees.max_priority_fee


@pytest.mark.parametrize("tx_hash", PARITY_TX_HASHES)
def test_python_and_pyreth_processed_transaction_parity(
    tx_hash: str,
    tx_data_fetcher,
    tx_analyzer,
    pyreth_tx_processor,
):
    """
    Ensure a processed transaction produced by the Python pipeline matches the
    PyReth output when both are represented as the shared ProcessedTransaction.
    """
    tx_data = tx_data_fetcher.get_transaction_data(tx_hash)

    python_tx = tx_analyzer.process_transaction(
        tx_data["transaction"],
        tx_data["receipt"],
        tx_data["trace"],
    )

    pyreth_raw = pyreth_tx_processor.process_transaction_from_hash_with_simulation(tx_hash)
    pyreth_tx = pyreth_raw.as_python_dataclass()
    assert isinstance(pyreth_tx, ProcessedTransaction)

    _assert_core_fields(python_tx, pyreth_tx)

    assert _project_erc20(python_tx.erc20_transfers) == _project_erc20(
        pyreth_tx.erc20_transfers
    ), "ERC20 transfer parity failed"

    assert _project_internal(python_tx.internal_transactions) == _project_internal(
        pyreth_tx.internal_transactions
    ), "internal transaction parity failed"

    assert python_tx.unique_addresses == pyreth_tx.unique_addresses
    assert python_tx.erc20_contracts == pyreth_tx.erc20_contracts

    # Ensure the simulator config accepts both Python and PyReth processed transactions directly.
    config = pyreth.PoolBuySellParameters.with_denom_amount(0.01, 18, 18)
    config.set_prior_transactions([python_tx, pyreth_raw])
