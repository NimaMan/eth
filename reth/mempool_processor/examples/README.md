# Mempool Processor Examples

## Mempool Fetcher Examples
- **test_fetcher_simple.rs** - Quick connectivity test for IPC client, fetches 10 transactions and shows detection latency
- **mempool_fetcher_performance_monitor.rs** - Continuous performance monitoring with real-time stats (TPS, latency percentiles, queue depth)
- **measure_instant_fetch_performance.rs** - Compares instant vs standard fetch methods with timing analysis
- **verify_new_transactions_only.rs** - Validates no duplicate transactions are received, shows duplicate detection statistics
- **function_detector_example.rs** - Demonstrates function signature detection from transaction calldata

## TX Router Examples  
- **tx_router_example.rs** - Complete pipeline demo: connects to Python publisher, builds token cache, processes 1000 transactions through function detector and router, logs detailed classification results

## Simulator Examples
- **database_lock_prevention_demo.rs** - Demonstrates MempoolSimulator prevents database locks during pool buy/sell simulations
- **live_mempool_transaction_simulation.rs** - Live mempool transaction simulation using MempoolFetcherIPCClient and MempoolSimulator with automatic nonce retry
- **token_tax_calculation_with_external_processor.rs** - Tax calculation using external tx_processor with known tokens (AITAI, 0xT, FLOKI)
- **simulation_pipeline.rs** - Full signal detection pipeline: fetches transactions, simulates, detects signals (tax, trading, liquidity)

## Token Parameter Extraction Examples
- **simulate_buy_sell_taxes.rs** - Calculates buy/sell taxes using transaction simulation
- **analyze_token_with_detectors.rs** - Comprehensive token analysis using multiple detection methods
- **get_token_info_via_rpc.rs** - Fetches token metadata via RPC calls

### reth_simulation/
Examples for Direct Reth simulation (20-40x faster than RPC):
- `basic_mempool_simulation.rs` - Basic mempool simulation
- `benchmark_1k_mempool.rs` - High-throughput benchmark 
- `state_change_extraction.rs` - State change extraction demo
- `working_direct_reth_simulation.rs` - Comprehensive example

### tx_simulation/
Examples for the new unified TxSimulator API:
- `test_state_changes_nonblocking.rs` - MempoolFetcherIPCClient + TxSimulator demo

## Examples with Input/Output

### tx_simulation/test_state_changes_nonblocking.rs

**Input**: Live mempool transactions via IPC
**Usage**: `cargo run --example test_state_changes_nonblocking --release`

**Output**:
```
🚀 TxSimulator API Demo with MempoolFetcherIPCClient
=================================================

✅ DirectTxSimulator initialized in 19.597037ms
   🎯 Direct Reth database access for ultra-fast simulation
📡 Connecting to mempool via MempoolFetcherIPCClient...
✅ MempoolFetcherIPCClient started - Sub-10μs detection!

📊 Processing transaction #1
   Hash: 0x07f0ef9defbd97
   Detection latency: 0.005ms
   ✅ Simulation successful in 0.694ms
   📈 Block: 22875496
   🎯 Addresses affected: 2
     1. Address: 0x3a50964Cc58801D4436929befEFb570B1E01c108
        ETH change: 0.01895746 ETH
     2. Address: 0xCfC0F98f30742B6d880f90155d4EbB885e55aB33
        ETH change: -0.01895746 ETH

📊 Processing transaction #8
   Hash: 0x20030887b8d7e9
   Detection latency: 0.004ms
   ✅ Simulation successful in 0.739ms
   📈 Block: 22875496
   🎯 Addresses affected: 2
     1. Address: 0x7c5830Cb1a1eFe898dcD5Cf401165Cb952508cCc
        Token changes: 1
          USDC: -1000.00000000
     2. Address: 0xc1C52c4350A7A646003E2471835b6E8F469EEF9c
        Token changes: 1
          USDC: 1000.00000000

📊 DEMO SUMMARY:
   Total processed: 10
   Successful simulations: 7 (70.0%)
   Transactions with state changes: 6 (60.0%)

✅ Demo completed successfully!
💡 This example demonstrates:
   - MempoolFetcherIPCClient for microsecond detection
   - TxSimulator with automatic latest block handling
   - Nonce retry logic built-in
   - Clean, unified API
```

### reth_simulation/basic_mempool_simulation.rs

**Input**: Live mempool transactions via IPC
**Usage**: `cargo run --example basic_mempool_simulation --release`

**Output**:
```
🚀 Basic Mempool Transaction Simulation
======================================

✅ Direct Reth simulator initialized in 20.023ms
📡 Connecting to mempool via IPC...
✅ Mempool monitoring started

Processing 1000 transactions...
Transaction 1/1000: 0x1b2c3d4e5f...
  ✅ Simulated in 345.123µs
     Gas used: 21000
     Success: true

Transaction 2/1000: 0x2c3d4e5f67...
  ❌ Simulation failed: nonce 1234 too high, expected 1232

Transaction 38/1000: 0x58b6f7c99b...
  ✅ Simulated in 690.998µs
     Gas used: 46019
     Success: true

📊 FINAL STATISTICS:
   Total processed: 1000
   Successful simulations: 672 (67.2%)
   Failed simulations: 328 (32.8%)
   Average simulation time: 445.67µs
   Total runtime: 23.4 seconds
```

### mempool_fetcher/measure_instant_fetch_performance.rs

**Input**: Live mempool via IPC
**Usage**: `cargo run --example measure_instant_fetch_performance --release -- 100`

**Output**:
```
🚀 Measuring Instant Fetch Performance
   Target: 100 transactions
   Batch size: 100

Client started, beginning measurement...
0x2ed34dfec8b17555243873df63bea183566e6c06efab12e7cb51fb4da6935230 4μs
0x4cf9239235731f0b6d0014e78f63645727bcbf4f3c8033c0364dc917efa8f8ed 4μs
0x287354b81764d657d7df86f861bb648b60f2677e0f0e35770b0380edd90fbf04 3μs
0x543956df92d4c319a3d2dbf7bfe605f123f4bad6ea84a10616b05fb4aa659753 4μs
0x941ebcd2159927b23308adab5ededd184561d832e56b35a5584ae0cf1bf4d5f9 5μs
...

📊 PERFORMANCE SUMMARY:
   Transactions collected: 100
   Average detection latency: 3.8μs
   P50 latency: 3μs
   P95 latency: 5μs
   P99 latency: 6μs
   Total time: 2.4 seconds
   Throughput: 41.7 tx/sec
```

## Prerequisites

- Local Ethereum node running with:
  - IPC socket at `/tmp/reth.ipc`
  - HTTP RPC on `http://127.0.0.1:8545`
  - Debug API enabled for trace methods
- Reth database directory accessible at `/home/nima/.local/share/reth/mainnet`

## Basic Usage

```bash
# Run any example
cargo run --example <example_name> --release

# Examples with custom IPC path
IPC_PATH=/custom/path/reth.ipc cargo run --example <example_name> --release

# Examples with arguments
cargo run --example measure_instant_fetch_performance --release -- 1000
```

## Common Environment Variables

- `IPC_PATH`: IPC socket path (default: `/tmp/reth.ipc`)
- `ETH_RPC_URL`: HTTP RPC endpoint (default: `http://127.0.0.1:8545`)
- `RETH_DATADIR`: Reth database directory (default: `/home/nima/.local/share/reth/mainnet`)
- `RUST_LOG`: Logging level (e.g., `info`, `debug`)

## Performance Characteristics

Based on actual runs:
- **IPC Detection**: 2-7μs (sub-10μs consistently)
- **Transaction Simulation**: 200-1000μs (Direct Reth)
- **State Change Extraction**: Included in simulation time
- **Success Rate**: ~70% (normal for mempool transactions)
- **Throughput**: 30-50 tx/sec sustained processing