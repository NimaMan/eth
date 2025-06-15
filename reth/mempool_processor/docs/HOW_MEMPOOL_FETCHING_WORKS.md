# How Mempool Fetching Works

## Overview

When you start the mempool processor, it faces two distinct challenges:
1. **Initial Mempool**: Capturing the ~20,000 transactions already in the mempool
2. **New Transactions**: Detecting new transactions as they arrive in real-time

## Current Implementation

### 1. WebSocket/IPC Subscription (What We Use)

**Initial Mempool:**
- ❌ **Cannot capture existing transactions**
- Subscriptions to `newPendingTransactions` only show NEW transactions
- The ~20,000 existing transactions are invisible to subscriptions

**New Transactions:**
- ✅ **100% coverage of new transactions**
- Real-time push notifications
- ~30ms latency from arrival at Reth to our detection

**Code Flow:**
```rust
// 1. Connect to WebSocket/IPC
let client = WebSocketClient::new("ws://localhost:8546");

// 2. Subscribe to newPendingTransactions
client.subscribe("newPendingTransactions");

// 3. Receive only NEW transactions
while let tx = client.next_transaction() {
    // This only gets transactions that arrive AFTER subscription
    process(tx);
}
```

### 2. HTTP RPC `txpool_content` (Fallback)

**Initial Mempool:**
- ⚠️ **Only captures ~7% of mempool**
- Can get snapshot immediately
- Misses 93% of transactions due to RPC limitations

**New Transactions:**
- ❌ **Not suitable for real-time**
- Must poll repeatedly (high overhead)
- High latency, misses transactions

**Code Example from main.rs:**
```rust
async fn get_mempool_transactions(provider: &Provider<Http>) -> Result<HashMap<H256, TransactionView>> {
    // Gets txpool_content - but only ~1,400 of 20,000 transactions
    let response: Value = provider.request("txpool_content", ()).await?;
    
    // Process pending (typically ~50-100 transactions)
    if let Some(pending) = response.get("pending") {
        // Only gets a small subset
    }
    
    // Process queued (rest of the 7%)
    if let Some(queued) = response.get("queued") {
        // Still missing 93% of mempool
    }
}
```

## The Initial Mempool Problem

When you start the process:

1. **Mempool State**: 20,000+ transactions already exist
2. **What We Can See**:
   - Via WebSocket/IPC: 0 existing transactions (only new ones)
   - Via RPC: ~1,400 transactions (7% of total)
3. **What We Miss**: 18,600+ transactions (93%)

## Current Workarounds

### Option 1: Accept the Gap (Current Approach)
- Start fresh, only monitor new transactions
- Build up picture over time
- Miss initial 20,000 transactions

### Option 2: Hybrid Approach
```rust
// 1. Get what we can from RPC (7%)
let initial = get_mempool_transactions().await?;
process_batch(initial); // Only ~1,400 transactions

// 2. Switch to WebSocket for new transactions
websocket.start_monitoring().await?;
// Now we get 100% of NEW transactions
```

### Option 3: Use Different Methods (Not Implemented)
- **DevP2P**: Can sync full mempool from peers
- **Direct Reth**: Direct memory access to full pool
- **Custom RPC**: Modify Reth to expose full mempool

## Performance Characteristics

| Stage | Method | Coverage | Latency |
|-------|--------|----------|---------|
| **Initial Load** | RPC | 7% | Instant |
| **Initial Load** | WebSocket/IPC | 0% | N/A |
| **New Transactions** | WebSocket/IPC | 100% | ~30ms |
| **New Transactions** | RPC Polling | Variable | 500ms+ |

## Practical Impact

For most use cases:
1. **Missing initial mempool is acceptable** - Old transactions likely to be mined soon
2. **Real-time coverage is critical** - Must catch new transactions immediately
3. **7% RPC coverage is misleading** - Often doesn't include the transactions you care about

## Future Solutions

To achieve 100% initial mempool capture would require:
1. **Direct Reth Integration** - Run inside Reth process
2. **Custom RPC Methods** - Modify Reth to expose full mempool
3. **DevP2P Implementation** - Sync mempool from peer nodes
4. **Database Snapshot** - Read from Reth's mempool database directly

Currently, we prioritize real-time detection of new transactions over initial mempool capture.