# Simulator Examples

This directory contains examples demonstrating the simulation capabilities of the mempool processor. Each example focuses on a specific aspect of transaction simulation using Direct Reth for high-performance analysis.

## Examples Overview

### 1. basic_mempool_simulation.rs

**Purpose**: Demonstrates the fundamental process of fetching mempool transactions and simulating them using Direct Reth.

**Inputs**:
- IPC connection to Reth node at `/tmp/reth.ipc`
- Reth database at `/home/nima/.local/share/reth/mainnet`
- RPC endpoint at `http://127.0.0.1:8545` for fetching raw transaction data

**Outputs**:
- Console output showing simulation results for each transaction
- Log file at `/home/nima/code/crypto/logs/mempool/dev/basic_sim_1k_[timestamp].log`
- Statistics including:
  - Success/failure rates
  - Gas usage
  - Simulation timing
  - Detection latency from IPC

**How it works**:
1. Connects to mempool via IPC to receive new transactions
2. Fetches raw transaction data via RPC
3. Simulates each transaction using Direct Reth (local DB access)
4. Processes 1000 transactions and reports performance metrics
5. Compares Direct Reth performance vs traditional RPC methods

### 2. benchmark_direct_reth_simulation.rs

**Purpose**: Comprehensive performance benchmark comparing Direct Reth simulation against traditional RPC methods.

**Inputs**:
- `--count`: Number of transactions to process (default: 1000)
- `--batch-size`: Batch size for fetching transactions (default: 50)
- `--reth-db`: Path to Reth database (default: `/home/nima/.local/share/reth/mainnet`)
- `--ipc-path`: IPC socket path (default: `/tmp/reth.ipc`)
- `--rpc-url`: RPC endpoint (default: `http://127.0.0.1:8545`)

**Outputs**:
- Real-time console output with batch processing statistics
- Detailed log file at `/home/nima/code/crypto/logs/mempool/dev/benchmark_direct_reth_[count]tx_[timestamp].log`
- Performance metrics:
  - Overall throughput (tx/sec)
  - Simulation time percentiles (P50, P95, P99)
  - Success/revert/failure breakdown
  - Average gas usage
  - Detection latency
  - Theoretical maximum throughput

**How it works**:
1. Processes transactions in configurable batches
2. Measures end-to-end latency for each transaction
3. Calculates detailed performance statistics
4. Generates comparison showing Direct Reth is typically 20-40x faster than RPC

### 3. state_change_extraction.rs

**Purpose**: Demonstrates extraction of detailed state changes from mempool transactions using debug_traceCall functionality.

**Inputs**:
- IPC connection to Reth node
- Reth database for Direct Reth access
- Target of 3 transactions to analyze

**Outputs**:
- Detailed state change information for each transaction:
  - ETH balance changes for all affected addresses
  - Token balance changes with proper decimal formatting
  - Storage slot modifications
  - Gas consumption details
- Human-readable formatting with token symbols where available

**How it works**:
1. Fetches individual transactions from mempool
2. Simulates with state change tracking enabled
3. Extracts and formats all address state modifications
4. Displays ETH transfers, token transfers, and storage changes
5. Provides insights into what each transaction actually does on-chain

### 4. test_buy_sell_simulator.rs

**Purpose**: Tests the buy/sell simulation functionality for detecting honeypots and calculating token taxes.

**Inputs**:
- Hardcoded test tokens:
  - AITAI token: `0x4ff734a3b6711bCb24a96a2Df86e9450d3cDFF28` (normal token)
  - 0xT token: `0x57Cb93206C3BaCa53C0BCeEd829Eb858A5885FDa` (known honeypot)
- Simulated buy/sell amounts (0.01 ETH worth)
- Latest block number from RPC

**Outputs**:
- For each token:
  - Buy simulation results with state changes
  - Sell simulation results (if not honeypot)
  - Calculated buy and sell tax percentages
  - Token balance changes
  - Gas usage for each operation
  - Honeypot detection status
- Formatted output showing all affected addresses and balances

**How it works**:
1. Creates a sequential buy/sell simulator with test wallet configuration
2. Simulates buying tokens with 0.01 ETH
3. Attempts to sell tokens back
4. Analyzes state changes to calculate actual taxes
5. Detects honeypots when sell transactions fail
6. Demonstrates the full tax calculation pipeline

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