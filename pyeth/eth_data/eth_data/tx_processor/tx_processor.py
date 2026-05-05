"""Compatibility import for the Rust-backed transaction processor."""

from eth_data.tx_processor.pyreth_tx_processor import PyRethTransactionProcessor


class TransactionProcessor(PyRethTransactionProcessor):
    """Legacy class name for PyReth/Rust transaction processing."""


__all__ = ["TransactionProcessor"]
