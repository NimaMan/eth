# Mempool Fetcher Module

Ultra-fast Ethereum mempool transaction detection with full transaction data via IPC.

## Overview

This module provides direct IPC (Inter-Process Communication) connection to a local Ethereum node (Reth) to receive full transaction data as soon as transactions enter the mempool. No RPC fallback needed - we get complete transaction data in the first notification.

## Components

### 1. **UltraFastClient** (`ultra_fast_client.rs`) - PRODUCTION 🚀
- **Latency**: 2-7 microseconds (μs)
- **Method**: Non-blocking socket reads with streaming JSON parser
- **Throughput**: 150-703 transactions/second sustained
- **Buffer**: 64KB read buffer, 1MB pending buffer
- **Usage**: Primary production client for signal detection

Key features:
- Uses `try_read()` for non-blocking I/O
- Processes JSON lines as they arrive
- Tracks sub-10μs, sub-100μs, and sub-1ms transactions
- Zero dependencies on RPC endpoints

### 2. **FullTransactionIpcClient** (`full_transaction_ipc_client.rs`) - LEGACY
- **Latency**: ~1ms average
- **Method**: Traditional async/await with buffered reader
- **Usage**: Backup implementation, used by standalone monitoring tools

Key features:
- More traditional async implementation
- Includes reconnection logic
- Detailed performance statistics
- Used by `mempool_full_tx_client` binary

### 3. **Types** (`types.rs`)
Common types shared across the codebase:
- `TransactionView`: Simplified transaction representation
- `MempoolTransaction`: Transaction with metadata (status, first seen time)
- `TransactionStatus`: Pending/Confirmed/Failed states

## Performance Comparison

| Client | Avg Latency | P99 Latency | Method |
|--------|-------------|-------------|---------|
| UltraFastClient | 5μs | 18μs | Non-blocking `try_read` |
| FullTransactionIpcClient | 1ms | 5ms | Async buffered reader |
| WebSocket (removed) | 28ms | 100ms | HTTP/WebSocket |
| RPC polling (removed) | 100ms+ | 500ms+ | HTTP RPC |

## Usage

### Production Usage (Signal Detection)
```rust
use mempool_processor::mempool_fetcher::{UltraFastClient, UltraFastTransaction};

let client = UltraFastClient::new(Some("/tmp/reth.ipc"))?;
client.start().await?;

while let Some(tx) = client.get_transaction().await? {
    // Process transaction with 2-7μs latency
    println!("Detected: {} in {}ns", tx.hash, tx.detection_ns);
}
```

### Monitoring/Testing Usage
```rust
use mempool_processor::FullTransactionIpcClient;

let client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
client.start_monitoring().await?;

let stats = client.get_stats().await;
println!("Processed {} transactions", stats.total_transactions);
```

## Technical Details

### IPC Subscription
Both clients use Ethereum JSON-RPC subscription:
```json
{
  "jsonrpc": "2.0",
  "method": "eth_subscribe",
  "params": ["newPendingTransactions", true],
  "id": 1
}
```

The `true` parameter requests full transaction objects instead of just hashes.

### Socket Optimization (UltraFastClient)
- Non-blocking mode: `socket.set_nonblocking(true)`
- Large buffers: 64KB read, 1MB pending
- Minimal allocations: Reuses buffers
- Direct parsing: No intermediate string conversions

### Data Flow
1. Transaction enters Reth mempool
2. Reth writes notification to IPC socket
3. UltraFastClient reads with `try_read()` (2-7μs)
4. JSON parsed and transaction extracted
5. Transaction sent via mpsc channel
6. Signal detection processes transaction

## Why Two Clients?

1. **UltraFastClient**: Optimized for absolute minimum latency in production
2. **FullTransactionIpcClient**: More features (reconnection, detailed stats) for monitoring

In production, always use UltraFastClient for signal detection.

## Removed Components

We've removed several redundant components:
- ❌ WebSocket client (28ms latency)
- ❌ DevP2P implementation (incomplete)
- ❌ RPC fallback mechanisms
- ❌ Processor module (duplicate functionality)

## Performance Verification

Run the benchmark to verify latency:
```bash
cargo run --example measure_ipc_latency --release
```

Expected results:
- Minimum: 1-2μs
- Average: 5μs
- P99: 18μs
- 100% under 100μs