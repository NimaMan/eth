## Block Pipeline Tests

This directory contains integration/performance tests for the Python block-processing stack. They no longer depend on RabbitMQ—the live pipeline publishes block snapshots and notifications via Redis.

### Available Tests

- `test_block_processor.py` / `test_block_processor_performance.py` – end-to-end validation and profiling of `BlockProcessor`.
- `test_block_fetcher_performance.py` – stress-tests the block fetcher over HTTP.
- `test_batch_data_processor.py` – ensures the receipt/trace batching logic still behaves as expected.
- `test_block_encoding_regression.py` – legacy serialization regression test for historical blocks that once failed to encode; retained because it still exercises tricky payloads.
- `test_monitor_new_blocks.py` – optional WebSocket smoke test (skipped by default) that verifies the node can be reached and blocks can be processed live.

### Prerequisites

1. A synced Ethereum node exposing both WebSocket (`ws://127.0.0.1:8546`) and HTTP (`http://127.0.0.1:8545`) endpoints.
2. Redis available at `redis://localhost:6379/0` if you plan to watch live block notifications.

### Notes

- The former RabbitMQ consumer tests were removed; Redis Pub/Sub is now the canonical notification path.
- When running the optional live monitor tests, ensure `LiveBlockProcessor` is running so Redis receives fresh snapshots.
