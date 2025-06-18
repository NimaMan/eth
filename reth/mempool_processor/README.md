# Mempool Processor - Real-time Ethereum Transaction Analysis

## Overview

High-performance Rust implementation for real-time Ethereum mempool monitoring and transaction analysis. Achieves sub-millisecond latency for scam detection and automated trading systems.

## Core Functionality

**Input**: Live Ethereum transactions from local Reth node mempool  
**Output**: Simulation-ready transaction data with comprehensive analysis  
**Latency**: 0.888ms average (65.3% sub-1ms) from mempool arrival to actionable data

## Measured Performance

### Primary Metric: Mempool Arrival → Simulation-Ready Data
- **Average Latency**: 0.888ms
- **Sub-1ms Success**: 65.3% of transactions
- **Success Rate**: 100% (no failed fetches)
- **Measurement Period**: 5 minutes continuous (2,976 transactions)

### Method Comparison
| Method | Average Latency | Use Case |
|--------|----------------|----------|
| **IPC** | 0.888ms | High-frequency trading |
| **WebSocket** | 1.486ms | Real-time dashboards |
| **HTTP RPC** | 1.969ms | Batch processing |

## Key Components

### 📡 **Mempool Fetcher** (`src/mempool_fetcher/`)
- **Input**: Transaction hash announcements via IPC/WebSocket
- **Output**: Complete signed transaction objects (`ethers::types::Transaction`)
- **Performance**: Multiple connection methods with measured latencies

### 🎯 **Signal Engine** (`src/signal_engine/`)
- **Input**: Simulation-ready transaction objects
- **Output**: Scam detection alerts and trading signals
- **Features**: Real-time pattern recognition, state change analysis

### ⚡ **Transaction Simulator** (`src/tx_simulator/`)
- **Input**: Signed transactions + current blockchain state
- **Output**: Predicted state changes and execution results
- **Engine**: REVM-based simulation with state diff tracking

### 🔍 **Performance Tools** (`examples/`)
- **Input**: Mempool stream or historical data
- **Output**: Detailed latency measurements and performance reports
- **Tools**: Precise timing measurement, method comparison, honest auditing

## Quick Start

### Prerequisites
- Local Reth node running with IPC enabled
- Rust 1.86.0+
- PostgreSQL database (for persistence)

### Run Performance Measurement
```bash
# 5-minute precise latency measurement
cargo run --example precise_mempool_latency_measurement

# Compare all connection methods
cargo run --example method_comparison_audit

# Detailed IPC performance audit
cargo run --example honest_performance_audit
```

### Run Signal Detection
```bash
# Real-time scam detection with IPC
cargo run --bin mempool_signal_detection_ipc_optimized

# Transaction simulation monitoring  
cargo run --bin revm_performance_monitor
```

## Architecture

```
Reth Mempool → IPC/WebSocket → Transaction Fetcher → Signal Engine → Alerts/Actions
     ↓              ↓                   ↓               ↓
  Real-time     Sub-ms          Simulation-ready    Pattern
  Updates       Latency         Transaction Data    Recognition
```

## Transaction Flow

1. **Detection**: Subscribe to `newPendingTransactions` via IPC
2. **Fetch**: Request complete transaction via `eth_getTransactionByHash`  
3. **Parse**: Deserialize to `ethers::types::Transaction` (simulation-ready)
4. **Analyze**: Run through signal detection and simulation engines
5. **Act**: Generate alerts or execute protective trades

## Documentation

- **[TRANSACTION_FETCHING_METHODS.md](TRANSACTION_FETCHING_METHODS.md)** - Complete methodology
- **[DEFINITIVE_MEMPOOL_TO_SIMULATION_MEASUREMENT.md](DEFINITIVE_MEMPOOL_TO_SIMULATION_MEASUREMENT.md)** - Measurement specification
- **[FINAL_MEASURED_PERFORMANCE_REPORT.md](FINAL_MEASURED_PERFORMANCE_REPORT.md)** - 5-minute test results

## Development

### Structure
- `src/` - Core library modules with individual READMEs
- `examples/` - Performance measurement tools
- `tools/` - Analysis and monitoring utilities
- `experimental/` - DevP2P and advanced features

### Testing
```bash
# Compile all components
cargo check

# Run specific measurement
cargo run --example [tool_name]

# Performance validation
./run_timing_analysis.sh
```

## Production Status

✅ **Live System**: Processing real mainnet transactions  
✅ **Performance Validated**: Sub-millisecond latency achieved  
✅ **Scam Detection**: Real-time protective trading active  
✅ **Measurement Tools**: Comprehensive performance auditing  

This system is production-ready for high-frequency trading and real-time blockchain analysis.