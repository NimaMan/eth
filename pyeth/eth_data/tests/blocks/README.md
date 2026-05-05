## Block Pipeline Tests

This directory contains integration/performance tests for Python consumers of the Rust/PyReth block-processing stack.

### Available Tests

- `test_block_processor.py` - end-to-end validation of `from pyreth import block_processor`.

### Prerequisites

1. A synced Ethereum node exposing both WebSocket (`ws://127.0.0.1:8546`) and HTTP (`http://127.0.0.1:8545`) endpoints.
2. Redis available at `redis://localhost:6379/0` if you plan to inspect live block snapshots from the Rust service.

### Notes

- The old Python live monitor, block fetcher, and tx fetcher tests were removed; block and tx processing are owned by Rust/PyReth.
