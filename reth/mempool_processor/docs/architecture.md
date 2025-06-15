# Mempool Processor

## Overview

The Mempool Processor is a high-performance Rust service that monitors Ethereum's mempool in real-time to detect scam transactions and protect users from malicious activities. It processes pending and queued transactions, simulates their execution using REVM, and identifies potential threats before they are mined.

## Architecture

### Core Components

1. **Transaction Fetcher** (`src/mempool_processor/fetcher.rs`)
   - WebSocket subscription for real-time transaction monitoring
   - HTTP fallback for reliability
   - Concurrent transaction fetching with semaphore-based rate limiting

2. **Pool Tracker** (`src/mempool_processor/pools.rs`)
   - Tracks Uniswap V2/V3/V4 liquidity pools
   - Monitors reserve changes in real-time
   - Detects potential rug pulls (>90% liquidity removal)
   - Maintains pool state cache with configurable thresholds

3. **Transaction Processor** (`src/mempool_processor/processor.rs`)
   - Filters transactions based on configurable criteria
   - Integrates with REVM for state simulation
   - Generates alerts for suspicious activities
   - Supports multiple alert reasons (high value, watched addresses, state changes)

4. **State Diff Tracker** (`src/tx_simulator/state_diff.rs`)
   - Simulates transactions using REVM
   - Extracts ETH balance changes
   - Traces internal transactions
   - Provides Python-compatible output format

5. **Scam Detection Engine** (`src/scam_detection/engine.rs`)
   - Analyzes simulated transaction effects
   - Compares against current pool states
   - Dynamic threshold calculation based on pool size
   - Generates scam alerts with detailed information

## Functionality

### Real-Time Monitoring
- Subscribes to pending transactions via WebSocket
- Processes up to 100 concurrent transactions
- No artificial delays - optimized for low latency
- Automatic fallback to polling if WebSocket fails

### Pool Tracking
- Registers and monitors liquidity pools
- Tracks reserve changes from swap events
- Detects large withdrawals and potential rug pulls
- Maintains statistics (TVL, pool count by type)

### Scam Detection
- **High Value Transfers**: Alerts on transactions above threshold
- **Pool Drainage**: Detects when pools drop below safety threshold
- **Large Withdrawals**: Identifies >50% liquidity removals
- **State Analysis**: Simulates transactions to predict effects

### Performance Features
- Concurrent processing with controlled parallelism
- Efficient memory management with bounded caches
- Optimized for <10ms processing per transaction
- Supports 100+ TPS throughput

## Code Structure

```
mempool_processor/
├── src/
│   ├── main.rs                    # Entry point with WebSocket subscription
│   ├── lib.rs                     # Library exports
│   ├── mempool_processor/
│   │   ├── mod.rs                 # Module definitions
│   │   ├── fetcher.rs             # Transaction fetching logic
│   │   ├── pools.rs               # Pool tracking implementation
│   │   ├── processor.rs           # Transaction processing
│   │   ├── streaming_fetcher.rs   # WebSocket streaming
│   │   └── types.rs               # Common data structures
│   ├── tx_simulator/
│   │   ├── simulator.rs           # REVM integration
│   │   ├── state_diff.rs          # State change tracking
│   │   └── conversions.rs         # Type conversions
│   └── scam_detection/
│       ├── engine.rs              # Scam detection logic
│       ├── service.rs             # Detection service
│       └── types.rs               # Scam-specific types
├── examples/
│   ├── mempool_monitor.rs         # Basic monitoring example
│   └── pool_rug_detector.rs       # Pool monitoring example
├── tools/
│   └── python/
│       └── core/                  # Performance analysis tools
└── tests/                         # Integration tests
```

## Usage Examples

### Basic Mempool Monitoring
```rust
cargo run --example mempool_monitor
```

This example shows:
- Connecting to local Ethereum node
- Filtering high-value transactions (>1 ETH)
- Real-time alert generation
- Pool interaction detection

### Pool Rug Pull Detection
```rust
cargo run --example pool_rug_detector
```

Demonstrates:
- Pool registration and tracking
- Simulating liquidity changes
- Detecting rug pulls
- Generating alerts

### Production Deployment
```bash
# Build optimized binary
cargo build --release

# Run with environment configuration
RUST_LOG=info \
WS_RPC_URL=ws://localhost:8546 \
HTTP_RPC_URL=http://localhost:8545 \
THRESHOLD_WEI=1000000000000000000 \
./target/release/mempool_processor
```

## Production Considerations

### Reliability
- Automatic WebSocket reconnection
- Fallback to HTTP polling
- Comprehensive error handling
- No panics in critical paths

### Performance
- Zero artificial delays
- Concurrent processing (100 parallel fetches)
- Efficient memory usage with bounded caches
- Optimized for local node connectivity (<10ms latency)

### Security
- Input validation on all external data
- Bounds checking for numeric operations
- Stale data rejection
- No unbounded memory growth

### Monitoring
- Detailed logging at multiple levels
- Performance metrics collection
- Transaction processing statistics
- Pool tracking statistics

## Dependencies

### Core Dependencies
- `ethers` - Ethereum client library
- `tokio` - Async runtime
- `revm` - EVM implementation for simulation
- `zmq` - Inter-process communication

### Supporting Libraries
- `tracing` - Structured logging
- `serde` - Serialization
- `clap` - CLI argument parsing
- `eyre` - Error handling

## Testing

Run all tests:
```bash
cargo test
```

Run specific module tests:
```bash
cargo test pools::tests
cargo test scam_detection::tests
```

Performance testing:
```bash
cd tools/python/core
./run_timing_analysis.sh
```

## Future Enhancements

1. **Multi-Protocol Support**
   - Add support for more DEX protocols
   - Cross-chain monitoring capabilities

2. **Advanced Detection**
   - Machine learning for pattern recognition
   - Behavioral analysis of addresses

3. **Integration Features**
   - REST API for external services
   - Webhook notifications
   - Database persistence layer

4. **Performance Optimization**
   - GPU acceleration for simulation
   - Distributed processing support

## Maintenance

- Keep dependencies updated: `cargo update`
- Run security audits: `cargo audit`
- Monitor memory usage in production
- Archive old logs and data regularly

This system is designed for production use with real blockchain data, providing critical protection against scams and malicious activities in the Ethereum ecosystem.