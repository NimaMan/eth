# WebSocket Method

## Overview

The WebSocket method connects to Reth node via WebSocket protocol for real-time transaction streaming. This is currently the production method as it provides a good balance between performance and reliability.

## Performance Characteristics

### Expected Performance

| Metric | Value | Notes |
|--------|-------|-------|
| **Average latency** | 1-50ms | Varies with network conditions |
| **Typical latency** | 15-25ms | Most transactions |
| **Mempool coverage** | 100% | Full visibility |
| **Consistency** | Good | More predictable than IPC |
| **Production ready** | Yes | Currently used in production |

### From README Claims

- Average: 28.3ms (measured in production)
- P50: 23.1ms
- P95: 47.2ms
- P99: 68.4ms
- 89.2% under 50ms
- 67.8% under 25ms

## Usage

```rust
use mempool_processor::mempool_fetcher::websocket::WebSocketClient;

// Connect to WebSocket endpoint
let client = WebSocketClient::new(
    "ws://localhost:8546",
    "http://localhost:8545"
)?;

// Start monitoring
client.start_monitoring().await?;

// Get transactions
loop {
    let transactions = client.get_transactions(100).await?;
    for tx in transactions {
        println!("Transaction {} detected in {:.2}ms", tx.hash, tx.latency_ms);
    }
}

// Check statistics
let stats = client.get_stats().await;
println!("Average latency: {:.2}ms", stats.avg_latency_ms);

// Get percentiles
let (p50, p95, p99) = client.get_percentiles().await?;
println!("P50: {:.2}ms, P95: {:.2}ms, P99: {:.2}ms", p50, p95, p99);
```

## Requirements

- Reth node with WebSocket enabled (port 8546)
- Network connectivity to Reth node
- HTTP endpoint for fetching full transaction data

## When to Use

✅ **Use WebSocket when:**
- Need full mempool coverage (100%)
- Running remotely from Reth node
- Need consistent, predictable latency
- Building production systems

❌ **Don't use WebSocket when:**
- Need absolute minimum latency (<1ms)
- Running on same machine as Reth (use IPC instead)
- Network connectivity is unreliable

## Advantages

1. **Full Coverage**: Sees 100% of mempool transactions
2. **Remote Access**: Works over network connections
3. **Consistent**: More predictable than IPC subscriptions
4. **Production Ready**: Battle-tested in production
5. **Standard Protocol**: Works with any Ethereum node

## Limitations

- Higher latency than IPC for simple queries
- Network overhead adds 10-50ms
- Requires stable network connection
- JSON serialization overhead

## Initial Capture Strategy

Unlike RPC polling which only sees 7% of mempool, WebSocket captures everything:

1. **Subscribe** to `newPendingTransactions`
2. **Stream** all new transactions as they arrive
3. **Fetch** full transaction data via HTTP if needed
4. **Track** timing for each transaction

This ensures 100% mempool visibility with reasonable latency.