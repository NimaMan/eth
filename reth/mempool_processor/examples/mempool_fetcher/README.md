# Mempool Fetcher Examples

This folder contains examples and monitoring tools for the mempool fetcher module.

## Production Monitoring (NonBlockingIpcClient - 2-7μs latency)

### Core Monitoring Tools
- **`mempool_queue_monitor.rs`** - Real-time queue dynamics monitoring
  - Tracks mempool size, arrival rates, detection rates
  - Calculates latency percentiles (P50, P95, P99)
  - Logs to CSV for analysis

- **`mempool_health_monitor.rs`** - Advanced health monitoring
  - Tracks transaction detection vs block inclusion timing
  - Identifies missed transactions
  - Measures system coverage and performance

- **`long_term_health_test.rs`** - Extended performance tracking
  - Runs for hours/days to identify degradation
  - Logs detailed metrics to CSV
  - Tracks memory usage and stability

### Validation & Testing
- **`verify_new_transactions_only.rs`** - Critical behavior validation
  - Confirms we only receive NEW transactions
  - Validates subscription semantics
  - Essential for understanding system behavior

- **`nonblocking_ipc_1k_demo.rs`** - Quick performance demo
  - Collects 1,000 transactions
  - Shows latency distribution
  - Good for quick system checks

### Alternative Approaches
- **`precise_mempool_latency_measurement.rs`** - Different methodology
  - Uses subscription + individual fetches
  - Provides complementary insights
  - Different measurement approach

## Legacy Examples (FullTransactionIpcClient - ~1ms latency)

These examples test the legacy client, kept for reference:
- **`test_new_full_tx_ipc.rs`** - Legacy client performance test
- **`validate_full_tx_data.rs`** - Legacy client data validation

## Running Examples

```bash
# Monitor queue dynamics
cargo run --example mempool_queue_monitor --release

# Run health monitoring
cargo run --example mempool_health_monitor --release

# Validate behavior
cargo run --example verify_new_transactions_only --release

# Quick performance check
cargo run --example nonblocking_ipc_1k_demo --release
```

## Key Metrics

Our production NonBlockingIpcClient achieves:
- **Average latency**: 2-7μs
- **P99 latency**: <20μs
- **Throughput**: 150-700 tx/s
- **Coverage**: 100% of new transactions entering mempool