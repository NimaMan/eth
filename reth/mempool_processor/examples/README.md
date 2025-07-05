# Mempool Processor Examples

This directory contains examples demonstrating various features and capabilities of the mempool processor. Each example is self-contained and demonstrates specific functionality.

## Examples Overview

### Mempool Fetching Examples

#### **mempool_fetcher/nonblocking_ipc_1k_demo.rs** ⚡
**Purpose**: Demonstrates NonBlockingIpcClient by collecting 1,000 transactions with microsecond latency.
- **Latency**: 2-7μs detection time
- **Output**: Timestamps logged to `/home/nima/code/crypto/logs/mempool/nonblocking_ipc_1k_demo.log`
- **Usage**: `cargo run --example nonblocking_ipc_1k_demo --release`

#### **test_new_full_tx_ipc.rs**
**Purpose**: Tests FullTransactionIpcClient performance over 10 seconds.
- **Latency**: ~1ms average
- **Features**: Reconnection logic, detailed statistics
- **Usage**: `cargo run --example test_new_full_tx_ipc`

#### **validate_full_tx_data.rs**
**Purpose**: Validates completeness of full transaction data from IPC.
- **Functionality**: Checks all transaction fields are present
- **Usage**: `cargo run --example validate_full_tx_data`

### 1. **actual_simulation_measurement.rs**
**Purpose**: Measures actual transaction simulation performance using real mempool data.
- **Input**: Connects to local Ethereum node (WebSocket + HTTP RPC)
- **Functionality**: 
  - Fetches transactions from mempool
  - Simulates them using REVM
  - Measures end-to-end latency and throughput
- **Output**: Performance metrics including TPS, latency percentiles, and success rates

### 2. **detect_pool_creation.rs**
**Purpose**: Detects new liquidity pool creation events from mempool transactions.
- **Input**: Transaction hash as command line argument
- **Functionality**:
  - Analyzes transaction logs for pool creation events
  - Identifies DEX factory contracts (Uniswap V2, V3, Sushiswap)
  - Extracts pool parameters (tokens, initial reserves)
- **Output**: Pool creation details including token addresses, factory, and initial liquidity

### 3. **detect_pool_state_changes.rs**
**Purpose**: Analyzes how a transaction affects liquidity pool states.
- **Input**: Transaction hash as command line argument
- **Functionality**:
  - Detects pool interactions (swaps, liquidity add/remove)
  - Calculates reserve changes and price impacts
  - Identifies all affected pools in complex transactions
- **Output**: Detailed state changes for each affected pool

### 4. **fast_simulation_with_state_changes.rs**
**Purpose**: Demonstrates fast transaction simulation using debug_traceCall.
- **Input**: Connects to local Ethereum node
- **Functionality**:
  - Uses RPC-based simulation (faster than REVM)
  - Extracts state changes from debug traces
  - Processes multiple transactions concurrently
- **Output**: State changes per address with timing metrics

### 5. **measure_mempool_performance.rs**
**Purpose**: Benchmarks mempool fetching performance.
- **Input**: WebSocket and HTTP RPC endpoints
- **Functionality**:
  - Measures WebSocket subscription latency
  - Tracks transaction arrival rates
  - Monitors queue depths and processing backlogs
- **Output**: Detailed performance statistics and bottleneck analysis

### 6. **monitor_pool_state_changes.rs** 
**Purpose**: Monitors mempool for transactions affecting known liquidity pools.
- **Input**: 
  - Python pool publisher (ZMQ on ports 5557/5558)
  - Mempool WebSocket subscription
- **Functionality**:
  - Subscribes to pool updates from Python
  - Filters mempool transactions involving known pools
  - Calculates comprehensive state changes including internal calls
- **Output**: Log file with all pool-related transactions and their state changes

### 7. **quick_simulation_test.rs**
**Purpose**: Quick test of transaction simulation capabilities.
- **Input**: Connects to local Ethereum node
- **Functionality**:
  - Fetches a few recent transactions
  - Simulates them with both REVM and debug_traceCall
  - Compares results and timing
- **Output**: Side-by-side comparison of simulation methods

### 8. **realtime_simulation_pipeline.rs**
**Purpose**: Demonstrates complete real-time mempool processing pipeline.
- **Input**: WebSocket mempool subscription
- **Functionality**:
  - Streaming mempool ingestion
  - Queue-based processing architecture
  - Concurrent simulation pipeline
  - Performance monitoring
- **Output**: Real-time statistics every 30 seconds

### 9. **simple_latency_test.rs**
**Purpose**: Measures basic mempool arrival latency.
- **Input**: WebSocket connection
- **Functionality**:
  - Tracks time from transaction broadcast to local arrival
  - Measures WebSocket notification delays
  - Calculates latency distributions
- **Output**: Latency statistics and percentiles

### 10. **test_optimized_ipc.rs**
**Purpose**: Tests optimized IPC connection to local Ethereum node.
- **Input**: IPC socket path
- **Functionality**:
  - Compares IPC vs HTTP performance
  - Measures request/response latencies
  - Tests concurrent request handling
- **Output**: Performance comparison between connection methods

### 11. **test_pool_subscriber.rs**
**Purpose**: Tests the pool subscriber component.
- **Input**: Python pool publisher on ZMQ
- **Functionality**:
  - Requests initial pool data via REQ/REP
  - Subscribes to real-time updates via PUB/SUB
  - Validates pool cache functionality
- **Output**: Pool subscription statistics and cache status

### 12. **test_tx_simulator_performance.rs**
**Purpose**: Comprehensive performance test of transaction simulators.
- **Input**: Recent block transactions
- **Functionality**:
  - Benchmarks REVM simulator
  - Benchmarks debug_traceCall simulator
  - Tests various transaction types
  - Measures state diff calculation overhead
- **Output**: Detailed performance breakdown by component

## Running Examples

### Prerequisites
- Local Ethereum node running with:
  - HTTP RPC on `http://127.0.0.1:8545`
  - WebSocket on `ws://127.0.0.1:8546`
  - Debug API enabled for trace methods
- For pool-related examples:
  - Python pool publisher running on ZMQ ports 5557/5558

### Basic Usage

```bash
# Run any example
cargo run --example <example_name>

# Examples requiring arguments
cargo run --example detect_pool_creation -- <TRANSACTION_HASH>
cargo run --example detect_pool_state_changes -- <TRANSACTION_HASH>

# Examples with optional environment variables
ETH_RPC_URL=http://localhost:8545 cargo run --example measure_mempool_performance
```

### Common Environment Variables
- `ETH_RPC_URL`: HTTP RPC endpoint (default: `http://127.0.0.1:8545`)
- `ETH_WS_URL`: WebSocket endpoint (default: `ws://127.0.0.1:8546`)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`)

## Output Locations

Most examples output to console. Some create log files:
- `monitor_pool_state_changes`: Creates logs in `/home/nima/code/crypto/logs/`
- Performance examples may create CSV files for analysis

## Troubleshooting

### "No pools received"
Ensure Python pool publisher is running:
```bash
python src/pool_subscriber/tests/python_publisher_test.py --ports 5557,5558
```

### "Connection refused"
Check that your Ethereum node is running and accessible:
```bash
curl http://127.0.0.1:8545 -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

### "Method not found: debug_traceCall"
Ensure your node has debug API enabled. For Reth:
```bash
reth node --http.api debug,eth,net,web3
```