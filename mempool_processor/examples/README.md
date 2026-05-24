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

### Trading-Enabled Replay Fixtures

These examples replay real recent mempool `trading_enabled` signals from local
Reth data. Use the block immediately before the tx mined so the simulator
replays the pending tx against the same parent-style state.

```bash
RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth \
cargo run --example replay_trading_enabled_signal --release -- \
  --datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --reth-index /home/nima/storage/samsung8tb/ethereum/reth/reth_index \
  --block 25163949 \
  --token 0x27a251c9a2669De495399764aD1e13c530405150 \
  --pool 0x520a40B187BAa890fbdCF44091D530caAB4813F0 \
  --hashes 0x6e771f8376501abca08f53120335ef3f798cb690e32ae9ccbfd42c3c47cbee2c

RETH_DATADIR=/home/nima/storage/samsung8tb/ethereum/reth \
cargo run --example replay_trading_enabled_signal --release -- \
  --datadir /home/nima/storage/samsung8tb/ethereum/reth \
  --reth-index /home/nima/storage/samsung8tb/ethereum/reth/reth_index \
  --block 25163941 \
  --token 0xb94aEa2e3729c03AcF7Af859d96A6b8487d49191 \
  --pool 0x62B00495457A7FA8a19Fe058F409506371E04e99 \
  --hashes 0xf813f4a3e419afd49df209c55907217d2c094625d7d3a84a90cfe9ca3841f8ab
```

Both commands should emit one `TradingEnabled` signal with `can_buy=true`,
`can_sell=true`, and 0%/0% tax.

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
