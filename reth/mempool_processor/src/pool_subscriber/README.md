# Pool Subscriber Module

## Overview

The `pool_subscriber` module serves as a real-time data bridge between the Python blockchain processing system and the Rust mempool processor. It subscribes to pool ETH reserve updates published by the Python component and maintains a thread-safe cache for high-performance access.

## Purpose

This module enables the Rust-based scam detection engine to access up-to-date pool liquidity information without making blockchain queries. It receives pre-processed pool data from Python, which has already parsed confirmed blocks and calculated pool states.

## Architecture

```
Python Side                         Rust Side (pool_subscriber)
-----------                         -----------------------------
Live Block Processor                PoolSubscriber
    ↓                                   |
Token Processor                         ├─ Connects to ZMQ endpoints
    ↓                                   ├─ Subscribes to updates
Pool Level Publisher  ----ZMQ---->      ├─ Maintains pool cache
    |                                   |
    ├─ PUB (5557) -----------→    SUB socket (real-time updates)
    └─ REP (5558) ←----------    REQ socket (initial data load)
                                        |
                                        ↓
                                  PoolStateCache
                                        |
                                        ↓
                                  Scam Detection Engine
```

## Input

### 1. Real-time Updates (PUB/SUB - Port 5557)

**Format**: JSON messages published on each block with pool changes
```json
{
  "type": "pool_updates",
  "timestamp": 1234567890.123,
  "data": {
    "0xPoolAddress1": {
      "eth_reserve": 123.456,
      "token_address": "0xTokenAddress",
      "block_number": 12345678,
      "update_time": 1234567890.123
    },
    "0xPoolAddress2": {
      "eth_reserve": 78.901,
      "token_address": "0xAnotherToken",
      "block_number": 12345678,
      "update_time": 1234567890.123
    }
  }
}
```

### 2. Initial Data Request (REQ/REP - Port 5558)

**Request**:
```json
{
  "type": "get_all_pools"
}
```

**Response**:
```json
{
  "status": "success",
  "count": 1234,
  "data": {
    "0xPoolAddress1": { /* same format as PUB updates */ },
    "0xPoolAddress2": { /* same format as PUB updates */ }
  }
}
```

## Output

### PoolStateCache API

The module provides a thread-safe cache accessible to other components:

```rust
// Get the cache instance
let cache = pool_subscriber.get_pool_cache();

// Get specific pool
if let Some(pool_state) = cache.get_pool("0xPoolAddress") {
    println!("Pool ETH reserve: {}", pool_state.eth_reserve);
}

// Get all pools
let all_pools = cache.get_all_pools();

// Get pool count
let count = cache.get_pool_count();
```

## Data Structures

### PoolUpdate
Represents an individual pool update from Python:
- `eth_reserve: f64` - Current ETH liquidity in the pool
- `token_address: String` - Address of the token paired with ETH
- `block_number: u64` - Block when this data was observed
- `update_time: f64` - Unix timestamp of the update

### PoolUpdatesMessage
Complete message format from Python publisher:
- `message_type: String` - Always "pool_updates"
- `timestamp: f64` - Message creation time
- `data: HashMap<String, PoolUpdate>` - Pool updates keyed by pool address

### PoolState
Cached pool state with staleness tracking:
- All fields from `PoolUpdate`
- `received_at: Instant` - When we received this update (for staleness checks)
- Methods: `is_stale()`, `age()`

## Key Features

1. **Address Normalization**: All addresses are stored in EIP-55 checksum format for consistency with Python
2. **Pool Limit Management**: Python side maintains a maximum of 2K pools, removing low-liquidity pools when capacity is reached
3. **Thread-Safe Access**: Uses `Arc<RwLock<HashMap>>` for concurrent read/write access
4. **Connection Resilience**: Handles ZMQ connection failures gracefully
5. **No Blockchain Queries**: Relies entirely on Python-processed data for performance

## Usage Example

```rust
use mempool_processor::pool_subscriber::PoolSubscriber;

// Create subscriber with 0.1 ETH threshold
let mut subscriber = PoolSubscriber::new(0.1);

// Or with custom endpoint
let mut subscriber = PoolSubscriber::with_endpoint(0.1, "tcp://localhost:5557");

// Get the cache for use in other components
let pool_cache = subscriber.get_pool_cache();

// Start listening (in async context)
subscriber.start_listening().await?;

// In another component, check pool liquidity
if let Some(pool) = pool_cache.get_pool("0xPoolAddress") {
    if pool.eth_reserve < 1.0 {
        // Low liquidity pool - potential scam risk
    }
}
```

## Implementation Notes

1. **Performance**: Uses direct Python values without blockchain verification for speed
2. **Memory**: Python side limits to 2K pools max, removing low-liquidity pools (< 0.01 ETH) when at capacity
3. **Latency**: Sub-millisecond access to cached data, ~10ms for ZMQ updates
4. **Error Handling**: Logs errors but continues operation on partial failures
5. **Pool Filtering**: Filtering by ETH reserves is handled on Python side to maintain consistent pool set

## Dependencies

- **ZeroMQ**: For PUB/SUB and REQ/REP communication
- **Python Pool Publisher**: Must be running for data updates
- **Local Network**: Assumes Python and Rust components on same host (localhost)

## Monitoring

Key metrics to monitor:
- Pool count in cache
- Update frequency (should match block time ~12s)
- Cache hit rate
- ZMQ connection status
- Memory usage

## Future Enhancements

1. Support for remote Python publishers (non-localhost)
2. Pool data persistence across restarts
3. Historical pool data queries
4. WebSocket alternative to ZMQ