# Ethereum Mempool Processor

A high-performance Rust application that monitors Ethereum's mempool for suspicious transactions and liquidity removal events.

## Overview

The Ethereum Mempool Processor is designed to:

1. Capture pending transactions from the Ethereum mempool in real-time
2. Filter transactions based on configurable criteria (value, gas price, addresses)
3. Simulate transactions using REVM to extract state changes
4. Track address balances and detect suspicious transfers
5. Identify potential scams and liquidity removal events
6. Alert on high-value or suspicious transactions via ZeroMQ

## Components

### Core Modules

- **Fetcher**: Retrieves transactions from the mempool using WebSocket or polling
- **Processor**: Filters transactions and generates alerts
- **Simulator**: Tracks address balances and simulates simple transactions
- **State Diff Tracker**: Uses REVM to simulate transactions and extract per-address balance changes
- **Alerts**: Publishes alerts via ZeroMQ in FlatBuffers format

## Project Structure

```
mempool_processor/
├── src/                              # Core Rust source code
│   ├── bin/                          # Main service binaries
│   │   ├── mempool_processor.rs      # Main mempool monitoring service
│   │   ├── mempool_tracker.rs        # Coverage analysis binary
│   │   ├── scam_detection_service.rs # Scam detection service
│   │   └── metrics_api_server.rs     # Real-time metrics API
│   └── [modules]/                    # Library modules (fetcher, processor, simulator)
├── examples/                         # Example implementations
│   ├── mempool_coverage_analysis/    # 53.2% coverage analysis (production-tested)
│   └── historical_simulation.rs      # REVM historical transaction simulation
├── tools/                           # Analysis and development tools
│   ├── performance/                 # Performance testing tools
│   ├── validation/                  # Transaction validation (90+ test cases)
│   └── python/                      # Python analysis utilities
├── tests/                           # Integration tests
└── docs/                           # Technical documentation
```

## Documentation

- **[README.md](README.md)** - Main project overview and usage
- **[README_PRODUCTION.md](README_PRODUCTION.md)** - Production deployment guide
- **[docs/TECHNICAL_ANALYSIS.md](docs/TECHNICAL_ANALYSIS.md)** - Detailed performance analysis and system architecture
- **[docs/EVM.md](docs/EVM.md)** - Queue theory analysis and transaction processing patterns
- **[examples/README.md](examples/README.md)** - Example implementations and usage
- **[tools/README.md](tools/README.md)** - Analysis tools and utilities
- **[tests/README.md](tests/README.md)** - Testing framework and validation tools

## Usage

### Main Services

#### Mempool Coverage Analysis
```bash
# Run 1-hour production analysis (53.2% coverage demonstrated)
cd examples/mempool_coverage_analysis
./run_hourly_analysis.sh
```

#### Main Mempool Processor
```bash
cargo run --bin mempool_processor -- \
  --ws-rpc-url ws://127.0.0.1:8546 \
  --http-rpc-url http://127.0.0.1:8545 \
  --threshold 100000000000000000 \
  --verbose
```

#### Scam Detection Service
```bash
cargo run --bin scam_detection_service -- \
  --http-rpc-url http://127.0.0.1:8545 \
  --ws-rpc-url ws://127.0.0.1:8546
```

## State Diff Tracking

The state diff tracker is used to:

1. Simulate transaction execution without broadcasting
2. Extract balance changes for all addresses involved 
3. Calculate ETH value impact for each address
4. Cache simulation results using LMDB
5. Detect suspicious outflows that may indicate scams

### How It Works

1. Transaction simulation uses REVM (Rust Ethereum Virtual Machine)
2. Initial state is loaded from the Ethereum node for relevant accounts
3. The transaction is executed locally
4. State changes (balance differences) are extracted
5. Changes are cached locally for fast access

## Building

```bash
# Build all binaries
cargo build --release

# Run only the test state diff utility
cargo run --bin test_state_diff
```

## Features

- **Real-time Transaction Monitoring**:
  - WebSocket subscription for instant notification of pending transactions
  - Fallback to polling-based approach when WebSocket is unavailable
  - Efficient handling of high transaction volumes

- **Transaction Simulation**:
  - Simulate transaction execution without broadcasting
  - Track high-value addresses (> 1 ETH balance)
  - Analyze transaction effects on account balances

- **Alert Generation**:
  - Monitor for suspicious transaction patterns
  - Track high-value transfers involving significant addresses
  - ZeroMQ integration for real-time alerts to external systems

## Architecture

The codebase is organized into modular components:

- **Fetcher**: Retrieves transactions from the Ethereum mempool
- **Processor**: Filters transactions based on criteria
- **Simulator**: Simulates transaction execution and analyzes effects
- **Alerts**: Publishes alerts via ZeroMQ to other systems

## Tools

### Mempool Processor

Main application that monitors the Ethereum mempool and generates alerts for interesting transactions:

```
cargo run --bin mempool_processor -- --http-rpc-url "http://localhost:8545" --ws-rpc-url "ws://localhost:8546"
```

### Transaction Simulator

Simulates mempool transactions to detect high-value transactions and track valuable addresses:

```
cargo run --bin simulate_tx -- --http-rpc-url "http://localhost:8545" --ws-rpc-url "ws://localhost:8546" --min-eth-balance 1.0
```

### Test Mempool

Test utility to generate mock transactions and verify the processing pipeline:

```
cargo run --bin test_mempool -- --tx-count 100 --high-value-count 10
```

## Getting Started

### Prerequisites

- Rust 1.65+
- Access to an Ethereum node with WebSocket and HTTP RPC endpoints
- Optional: ZeroMQ if using the alert subscriber

### Installation

1. Clone the repository
2. Build the project: `cargo build --release`
3. Configure your Ethereum RPC endpoints
4. Run one of the tools above

### Configuration

Both the main processor and simulator accept the following parameters:

- `--http-rpc-url`: HTTP endpoint for RPC requests
- `--ws-rpc-url`: WebSocket endpoint for real-time subscriptions
- `--min-eth-balance` (simulator only): Minimum ETH balance for tracked addresses
- `--verbose`: Enable more detailed logging

## Roadmap

- Enhanced transaction simulation with contract execution
- DEX liquidity pool monitoring
- Machine learning-based anomaly detection
- Integration with blockchain analytics platforms

## License

This project is licensed under [MIT License](LICENSE).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 