# Mempool Fetcher Examples

All fetcher examples use `RETH_IPC_PATH` or `IPC_PATH` when set, otherwise
they resolve `RETH_IPC_PATH` from `/home/nima/code/crypto/blockchains/eth/config.env`.

## Instant Collection Performance Measurement

**`measure_instant_fetch_performance.rs`** - Measure the new instant fetch method with configurable parameters

### Usage:
```bash
# Quick test - 1,000 transactions
cargo run --example measure_instant_fetch_performance --release -- 1000

# Standard test - 10,000 transactions (default)
cargo run --example measure_instant_fetch_performance --release

# Long test - 100,000 transactions
cargo run --example measure_instant_fetch_performance --release -- 100000

# Custom batch size
cargo run --example measure_instant_fetch_performance --release -- 10000 --batch-size 200
```

### What it measures:
- **Fetch latency**: Time to call `get_transactions_instant()` (should be <100μs)
- **Batch efficiency**: How full are the batches? Are we hitting limits?
- **Detection latency**: The `detection_ns` from each transaction
- **Throughput**: Transactions per second sustained

### Output:
- CSV file with per-fetch details
- Console summary with percentiles (P50, P95, P99)
- Warnings for performance issues

## Long-term Monitoring

**`mempool_fetcher_performance_monitor.rs`** - Production monitoring tool

### What it measures:
1. **IMMEDIATE detection**: Are we getting transactions within microseconds? (Target: <10μs)
2. **COMPLETE data**: Do we get all transaction fields in the first notification?
3. **LONG-TERM stability**: Does performance degrade over hours/days?

### Run it:
```bash
cargo run --example mempool_fetcher_performance_monitor --release
```

## Other Examples

**`verify_new_transactions_only.rs`** - Validates we only get NEW transactions (not existing mempool)

**`test_fetcher_simple.rs`** - Small connectivity smoke test that fetches up to 10 transactions

**`function_detector_example.rs`** - Runs live transactions through the function detector

## Production Stats

Our MempoolFetcherIPCClient achieves:
- **Average latency**: 5-7μs
- **Target**: <10μs for 95%+ of transactions
- **Data completeness**: 100% (all fields in first notification)
- **Long-term stability**: No degradation over days of running
