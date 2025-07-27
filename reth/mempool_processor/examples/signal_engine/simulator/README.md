# Direct Reth Mempool Transaction Simulation Examples

This directory contains working examples of simulating mempool transactions using Direct Reth integration, which bypasses RPC for 20-40x performance improvement.

## Prerequisites

- Running Reth node with IPC enabled at `/tmp/reth.ipc`
- Reth database at `/home/nima/.local/share/reth/mainnet`
- HTTP RPC endpoint at `http://127.0.0.1:8545` (for raw transaction fetching)

## Examples

### 1. `basic_mempool_simulation.rs`
Basic example showing how to:
- Connect to mempool via IPC
- Fetch pending transactions
- Simulate them using Direct Reth
- Log results with timing metrics

```bash
cargo run --release --example basic_mempool_simulation
```

Output:
- Processes 10 transactions
- Shows gas usage and success/failure status
- Logs to `/home/nima/code/crypto/logs/mempool/basic_sim_*.log`

### 2. `state_change_extraction.rs`
Demonstrates state change extraction:
- Simulates transactions and extracts state changes
- Shows which addresses were affected
- Displays storage slot modifications
- Identifies contract deployments

```bash
cargo run --release --example state_change_extraction
```

Output:
- Detailed state changes for 3 transactions
- Pre/post state comparison
- Storage slot analysis

### 3. `benchmark_1k_mempool.rs`
High-throughput benchmark:
- Processes 1000 mempool transactions
- Measures performance metrics
- Compares with RPC baseline

```bash
cargo run --release --example benchmark_1k_mempool
```

Output:
- Throughput metrics (tx/sec)
- Latency percentiles (P50, P95, P99)
- Success/failure/revert rates
- Logs to `/home/nima/code/crypto/logs/mempool/benchmark_1k_*.log`

### 4. `test_buy_sell_simulator.rs`
Honeypot detection and tax calculation:
- Simulates buy -> approve -> sell sequences
- Detects honeypot tokens (can buy but can't sell)
- Calculates buy and sell taxes from state changes
- Tests with known tokens (AITAI and 0xT)

```bash
cargo run --release --example test_buy_sell_simulator
```

Output:
- Honeypot detection results
- Tax percentages for buy/sell
- Performance metrics (8-15ms per simulation)

## Performance Metrics

Typical performance observed:
- **Simulation time**: 100-300µs per transaction
- **State extraction**: 40-120µs additional
- **Throughput**: 1000-5000 tx/sec (vs 10-50 tx/sec with RPC)
- **Detection latency**: 50µs-100ms (mempool to detection)

## Key Components

### Direct Reth Simulator
- Uses `reth_signed_tx_simulator::RethSignedTxSimulator`
- Directly accesses Reth's MDBX database
- No JSON serialization overhead
- Native REVM execution

### Mempool Fetcher
- Uses `mempool_processor::FullTransactionIpcClient`
- Connects via IPC for low-latency transaction streaming
- Tracks detection latencies with nanosecond precision

### State Changes Format
Compatible with `debug_traceCall` prestateTracer:
```json
{
  "pre": {
    "0xAddress": { "balance": "0x...", "nonce": "0x..." }
  },
  "post": {
    "0xAddress": { 
      "balance": "0x...", 
      "storage": { "0xSlot": "0xValue" }
    }
  }
}
```

## Old Examples

The previous mock implementations have been moved to `old_examples/` directory. These used placeholder implementations and should not be used.

## Troubleshooting

1. **"Reth database path does not exist"**
   - Ensure Reth is synced and database exists at the expected path
   - Update the path in the examples if your Reth data is elsewhere

2. **"Failed to connect to IPC"**
   - Verify Reth is running with IPC enabled
   - Check the socket exists: `ls -la /tmp/reth.ipc`

3. **"Failed to get raw transaction"**
   - Ensure HTTP RPC is enabled on port 8545
   - Some mempool transactions may be replaced before we can fetch them

4. **High failure rate**
   - Normal for mempool transactions (nonce gaps, replaced transactions)
   - Transactions may depend on others not yet included