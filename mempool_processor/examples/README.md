# Mempool Processor Examples

All Rust examples under this directory are registered in `Cargo.toml` and are
covered by:

```bash
cargo check --examples
```

## Mempool Fetcher

- `test_fetcher_simple`: quick IPC connectivity test; fetches up to 10 pending transactions.
- `measure_instant_fetch_performance`: bounded performance run with CSV and hash logs.
- `mempool_fetcher_performance_monitor`: long-running latency/completeness monitor.
- `verify_new_transactions_only`: compares the current `txpool_content` snapshot with `newPendingTransactions` subscription output.
- `function_detector_example`: runs live mempool transactions through the function detector.

```bash
cargo run --example test_fetcher_simple
cargo run --example measure_instant_fetch_performance --release -- 1000 --batch-size 100
cargo run --example mempool_fetcher_performance_monitor --release
cargo run --example verify_new_transactions_only --release
cargo run --example function_detector_example --release
```

## Router And Pipeline

- `tx_router_example`: token cache subscriber plus function detector plus transaction router.
- `full_pipeline_signal_detection`: live fetch, route, simulate, and log signal-pipeline results.
- `replay_trading_enabled_signal`: replay a known trading-enabled sequence through the detector path.

```bash
cargo run --example tx_router_example --release
cargo run --example full_pipeline_signal_detection --release -- --target-count 1000
cargo run --example replay_trading_enabled_signal --release -- --help
```

## Simulator Replays

- `replay_contract_creation`: replay a contract-creation transaction from local Reth data.
- `replay_liquidity_removal`: replay and inspect a liquidity-removal transaction.
- `replay_trading_status`: replay creator helper transactions, then run a buy/approve/sell probe.

```bash
cargo run --example replay_contract_creation --release -- --help
cargo run --example replay_liquidity_removal --release -- --help
cargo run --example replay_trading_status --release -- --help
```

## Writers And Diagnostics

- `test_unified_writer`: verify unified signal writer behavior.
- `signal_subscriber/signal_subscriber.py`: Python ZMQ subscriber for published signals.

```bash
cargo run --example test_unified_writer --release
python examples/signal_subscriber/signal_subscriber.py
```

## Live Dependencies

Live examples expect local infrastructure:

- Shared config: `/home/nima/code/crypto/blockchains/eth/config.env`
- Reth IPC socket: default `/home/nima/storage/samsung8tb/ethereum/reth/reth.ipc`
- HTTP RPC: default `http://127.0.0.1:8545`
- Reth data directory: default `/home/nima/storage/samsung8tb/ethereum/reth`
- Token server: default `http://127.0.0.1:8765`

Useful overrides:

```bash
RETH_IPC_PATH=/custom/reth.ipc cargo run --example test_fetcher_simple
RETH_HTTP_RPC=http://127.0.0.1:8545 cargo run --example verify_new_transactions_only
RETH_DATADIR=/path/to/reth cargo run --example full_pipeline_signal_detection
MEMPOOL_LIVE_TOKEN_SERVER_URL=http://127.0.0.1:8765 cargo run --example full_pipeline_signal_detection
```
