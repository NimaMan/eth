# Mempool Fetcher Examples

## Primary Performance Monitor

**`mempool_fetcher_performance_monitor.rs`** - The ONLY monitoring tool you need

### What it measures:
1. **IMMEDIATE detection**: Are we getting transactions within microseconds? (Target: <10μs)
2. **COMPLETE data**: Do we get all transaction fields in the first notification?
3. **LONG-TERM stability**: Does performance degrade over hours/days?

### Run it:
```bash
cargo run --example mempool_fetcher_performance_monitor --release
```

### Output:
- Logs to `/home/nima/code/crypto/logs/mempool/performance_YYYYMMDD_HHMMSS.csv`
- Shows real-time warnings for high latency (>50μs) or incomplete data
- Prints performance summary every 5 minutes
- Checks for degradation every hour

## Other Examples

**`nonblocking_ipc_1k_demo.rs`** - Quick test that collects 1,000 transactions

**`verify_new_transactions_only.rs`** - Validates we only get NEW transactions (not existing mempool)

## Production Stats

Our NonBlockingIpcClient achieves:
- **Average latency**: 5-7μs
- **Target**: <10μs for 95%+ of transactions
- **Data completeness**: 100% (all fields in first notification)
- **Long-term stability**: No degradation over days of running