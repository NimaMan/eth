# IPC Socket Method

## Overview

The IPC (Inter-Process Communication) socket method connects to a local Reth node via Unix domain socket at `/tmp/reth.ipc`. This provides lower latency than HTTP/WebSocket connections by avoiding network stack overhead.

## Performance Characteristics

### Measured Performance (from audit_ipc_measurement.rs)

| Metric | Value | Notes |
|--------|-------|-------|
| **Round-trip latency** | ~20μs average | Simple queries like eth_blockNumber |
| **Subscription notifications** | 96ms average | High variance due to OS buffering |
| **P50 (median)** | 40ms | Half of transactions detected within 40ms |
| **P95** | 337ms | 95% detected within 337ms |
| **P99** | 600ms | Outliers due to OS scheduling |
| **<1ms detection** | 11.4% | Only 11% achieve sub-millisecond |

### Why the High Variance?

1. **OS Buffering**: Unix sockets buffer data, causing batch delivery
2. **Thread Scheduling**: OS scheduling adds unpredictable delays
3. **Notification Bundling**: Multiple transactions may arrive together
4. **Measurement Artifacts**: Some 0μs readings indicate buffering

## Usage

```rust
use mempool_processor::mempool_fetcher::ipc_socket::IpcClient;

// Connect to default socket (/tmp/reth.ipc)
let client = IpcClient::new(None)?;

// Start monitoring
client.start_monitoring().await?;

// Get transactions
loop {
    let transactions = client.get_transactions(100).await?;
    for tx in transactions {
        println!("Transaction {} detected in {}μs", tx.hash, tx.latency_us);
    }
}

// Check statistics
let stats = client.get_stats().await;
println!("Average latency: {}μs", stats.avg_latency_us);
```

## Requirements

- Local Reth node with IPC enabled
- Unix domain socket at `/tmp/reth.ipc`
- Proper file permissions for socket access

## When to Use

✅ **Use IPC when:**
- Running on same machine as Reth node
- Need lowest possible latency for simple queries
- Can tolerate high variance in subscription notifications

❌ **Don't use IPC when:**
- Need consistent sub-millisecond detection
- Running on different machine than Reth
- Need predictable latency guarantees

## Comparison with Other Methods

| Method | Average Latency | Consistency | Use Case |
|--------|----------------|-------------|----------|
| **IPC Socket** | 96ms (subscriptions) | High variance | Local node access |
| **WebSocket** | 1-50ms | More consistent | Remote access OK |
| **DevP2P** | <10ms target | Good | Direct protocol |
| **Direct Reth** | <1ms target | Excellent | Custom builds |

## Limitations

- Only works with local Reth node
- High variance in notification delivery
- OS-level buffering affects real-time performance
- Not suitable for consistent sub-millisecond requirements