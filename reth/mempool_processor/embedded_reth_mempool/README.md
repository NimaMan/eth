# Embedded Reth Mempool Listener

Ultra-low latency Ethereum mempool transaction detection using embedded Reth networking stack.

## Overview

This project implements an architectural approach to reduce Ethereum mempool transaction detection latency by embedding Reth's networking components directly into your application, eliminating IPC overhead.

## Performance

| Method | Latency | Notes |
|--------|---------|--------|
| **Embedded Reth** | **Theoretical improvement** | Eliminates IPC overhead |
| Unix IPC | 150-300 µs | Current mempool_processor |
| WebSocket | 1500-3000 µs | Standard RPC |

**Note**: Performance improvements are architectural - this implementation eliminates IPC/JSON-RPC overhead through direct memory access, but specific latency numbers require real-world testing and comparison.

## Architecture

```
┌────────────────────────────┐
│ Your Application           │
├────────────────────────────┤
│ Embedded Reth Components:  │
│ - Network Manager          │
│ - Transaction Pool         │
│ - P2P Protocol            │
└────────────────────────────┘
        │
        │ Direct memory access
        │ Zero copy, zero IPC
        ▼
   Arc<TransactionSigned>
```

## Quick Start

```rust
use embedded_reth_mempool::{EmbeddedRethConfig, EmbeddedRethListener};
use futures::StreamExt;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Create listener
    let config = EmbeddedRethConfig::default();
    let listener = EmbeddedRethListener::new(config).await?;
    
    // Start processing
    listener.start_processing().await?;
    
    // Subscribe to transactions
    let mut tx_stream = listener.subscribe();
    
    // Process transactions with improved latency  
    while let Some(tx) = tx_stream.next().await {
        println!("TX: {} | Processing time: {}μs", tx.hash_short(), tx.latency_us());
    }
    
    Ok(())
}
```

## Running Examples

### Simple Usage
```bash
cargo run --example simple_usage
```

### Integration Demo
```bash
cargo run --example integration_demo
```

### Working Binaries
```bash
# Simple demonstration
cargo run --bin embedded_demo

# Precise timing measurement
cargo run --bin precise_timing_measurement

# Transaction verification
cargo run --bin verify_signed_tx
```

## Integration with mempool_processor

This module is designed to be integrated back into the main mempool_processor project once proven stable. The integration points are:

1. **Transaction Format**: `EmbeddedTransaction` can be converted to mempool_processor's `TransactionView` format
2. **Metrics**: Compatible with existing performance tracking
3. **Configuration**: Can be added as a new fetcher method alongside IPC and WebSocket

### Integration Example

```rust
// In your mempool_processor signal detection:
use embedded_reth_mempool::{EmbeddedRethListener, EmbeddedRethConfig, MempoolTransaction};

// Create embedded listener
let config = EmbeddedRethConfig::with_port(30313);
let listener = EmbeddedRethListener::new(config).await?;
listener.start_processing().await?;

// Use alongside existing methods
let method = match args.fetch_method {
    "embedded" => {
        let mut tx_stream = listener.subscribe();
        while let Some(tx) = tx_stream.next().await {
            let mempool_tx = MempoolTransaction::from(&tx);
            signal_detector.process(mempool_tx).await?;
        }
    }
    "ipc" => { /* existing IPC logic */ }
    "websocket" => { /* existing WebSocket logic */ }
};
```

## Technical Details

### Why It's Faster

1. **Zero IPC Overhead**: No Unix sockets, no TCP, no JSON-RPC
2. **Direct Memory Access**: Transactions are passed as `Arc<TransactionSigned>`
3. **No Serialization**: No RLP encoding/decoding, no JSON parsing
4. **Same Process**: No context switches, no kernel scheduling

### Limitations

1. **Same Machine**: Must run on the same machine as your application
2. **Resource Usage**: Includes full P2P networking stack
3. **Port Requirements**: Needs ports 30303 (TCP) and 30304 (UDP) open

## Development

### Running Tests
```bash
cargo test
```

### Building for Production
```bash
cargo build --release
```

### Benchmarking
```bash
cargo bench
```

## Future Enhancements

1. **Mempool Entry Timestamps**: Get actual mempool entry time for accurate latency
2. **Custom Pool Implementation**: Further optimize for specific use cases
3. **Selective Transaction Filtering**: Filter at the pool level to reduce overhead
4. **Integration Tests**: Test with real mempool_processor signal detection

## License

Same as mempool_processor project.