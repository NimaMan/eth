# Ethers-rs Method Reference for Mempool Processing

This document provides the correct method names for working with the ethers-rs crate v2.0 for mempool and transaction operations.

## Transaction Fetching Methods

### HTTP Provider Methods
```rust
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::H256;

// Create provider
let provider = Provider::<Http>::try_from("http://localhost:8545")?;

// Fetch single transaction by hash
let tx_hash: H256 = "0x...".parse()?;
let transaction = provider.get_transaction(tx_hash).await?;

// Note: There is NO method called `pending_transaction_hashes()` in ethers v2.0
// For pending transactions, use WebSocket subscriptions
```

### WebSocket Subscription for Pending Transactions
```rust
// Subscribe to pending transactions via WebSocket
let subscribe_request = json!({
    "jsonrpc": "2.0",
    "method": "eth_subscribe",
    "params": ["newPendingTransactions"],
    "id": 1
});
```

### IPC Methods (Using Our Custom Wrappers)
```rust
use mempool_processor::mempool_fetcher::ipc_socket::FullTxIpcClient;

// Create IPC client
let ipc_client = FullTxIpcClient::new(Some("/tmp/reth.ipc"))?;

// Start monitoring (replaces subscribe_to_mempool)
ipc_client.start_monitoring().await?;

// Get transactions (replaces get_next_transaction)
let transactions = ipc_client.get_full_transactions(max_count).await?;
```

## Common Errors and Fixes

### Error: "no method named `pending_transaction_hashes`"
**Incorrect:**
```rust
let hashes = provider.pending_transaction_hashes().await?;
```

**Correct:**
Use WebSocket subscriptions to `newPendingTransactions` instead.

### Error: "no method named `get_next_transaction`"
**Incorrect:**
```rust
let tx = ipc_client.get_next_transaction().await?;
```

**Correct:**
```rust
let transactions = ipc_client.get_full_transactions(1).await?;
if let Some(tx) = transactions.into_iter().next() {
    // process tx
}
```

### Error: "no method named `subscribe_to_mempool`"
**Incorrect:**
```rust
ipc_client.subscribe_to_mempool().await?;
```

**Correct:**
```rust
ipc_client.start_monitoring().await?;
```

## Database Logger Methods
```rust
use mempool_processor::database::DbLogger;

// Available methods (no `log_scam_signal` method exists):
db_logger.write_mempool_scam_prediction(...).await?;
db_logger.write_mempool_scam_prediction_with_tx(...).await?;
```

## Working Examples

See these files for correct usage patterns:
- `src/bin/mempool_signal_detection_ipc_optimized.rs` - Production ready
- `examples/mempool_fetch_timing_analysis.rs` - HTTP provider usage
- `src/mempool_fetcher/websocket/client.rs` - WebSocket subscriptions