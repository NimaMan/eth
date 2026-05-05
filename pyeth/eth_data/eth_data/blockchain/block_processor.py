"""Compatibility import for the Rust-backed block processor.

The old Python RPC/debug-trace implementation has been retired from this
entry point.  Existing code can continue importing ``BlockProcessor`` from
``eth_data.blockchain.block_processor`` while sharing the single PyReth/Rust
processor under the hood.
"""

from eth_data.blockchain.pyreth_block_processor import PyRethBlockProcessor


class BlockProcessor(PyRethBlockProcessor):
    """Legacy class name for the PyReth/Rust block processor."""


__all__ = ["BlockProcessor"]
