# Token Tracking Publishers

## Overview

The publishers module provides real-time token and pool state information from the Python blockchain processor to external consumers, primarily the Rust mempool analyzer for scam detection. This system tracks live token updates and maintains a cache of healthy pools for efficient mempool transaction analysis.

## Architecture

```
LiveBacktestEngine → TokenInfoExtractor → TokenTrackingCache → TokenInfoPublisher → ZeroMQ → Rust Mempool Processor
      (blocks)         (extract pools)     (filter scams)       (publish)                    (scam detection)
```

## Components

### 1. **TokenInfoExtractor** (`token_info_extractor.py`)
- Extracts pool and token data from blockchain updates
- Converts raw token data into structured pool information
- Tracks token creators and pool creation metadata

### 2. **TokenTrackingCache** (`token_tracking_cache.py`)
- Maintains cache of healthy (non-scam) pools
- Automatically evicts pools when tokens are marked as scams
- Filters out low-liquidity pools to reduce noise

### 3. **TokenInfoPublisher** (`token_info_publisher.py`)
- Publishes pool updates via ZeroMQ PUB/SUB (port 5557)
- Provides REQ/REP interface for data queries (port 5558)
- Handles block synchronization updates

### 4. **TradeSignalPublisher** (`trade_signal_publisher.py`)
- Publishes trading signals to eth_kartal execution engine
- Tracks signal execution status and confirmations

## Published Data Schema

### Token/Pool Updates (PUB Socket - Port 5557)

```json
{
    "type": "token_updates",
    "timestamp": 1703001234.567,
    "data": {
        "0x123...abc": {  // pool_address
            "token_symbol": "PEPE",
            "token_name": "Pepe Token",
            "token_address": "0x456...",
            "pool_address": "0x123...abc",
            "pool_type": "V2",  // V2, V3, V4
            "denom_currency": "WETH",  // WETH, USDC, USDT, etc.
            "denom_address": "0xC02aa...",
            "token_decimals": 18,
            "denom_reserve": 5.5,  // ETH or denomination token amount
            "token_reserve": 1000000.0,
            "latest_block_number": 18750123,
            "trading_enabled": true,
            
            // Creation metadata
            "creator_address": "0x789...",
            "creation_block": 18750000,
            "creation_timestamp": 1703000000,
            "creation_tx": "0xabc...",
            
            // Trading enable info
            "trading_enabled_block": 18750010,
            "trading_enabled_tx": "0xdef...",
            
            // Ownership info
            "current_owner": "0x000...",
            "ownership_renounced": true,
            "renouncement_block": 18750020,
            
            // Supply info
            "total_supply": 1000000000,
            
            // Scam detection info
            "is_scam": false,
            "scam_label": null,  // e.g. "rug_pull", "honeypot", etc.
            
            // Calculated fields
            "pool_age_blocks": 123
        }
    }
}
```

### REQ/REP Interface (Port 5558)

#### Request Types:

1. **Get Specific Pool**
```json
{
    "type": "get_pool",
    "pool_address": "0x123..."
}
```

2. **Get All Pools**
```json
{
    "type": "get_all_pools"
}
```

3. **Get Pool Statistics**
```json
{
    "type": "get_pool_stats"
}
```

4. **Get Mempool Data** (includes creator tracking)
```json
{
    "type": "get_mempool_data"
}
```

## Mempool Processor Integration

The Rust mempool processor uses this data for real-time scam detection:

### Critical Fields for Scam Detection:
- **`denom_reserve`**: Current ETH/token reserves for drain detection
- **`token_reserve`**: Token side of the pool
- **`token_address`**: Identifies the token
- **`block_number`**: For staleness checks

### Scam Detection Logic:
1. Simulates pending transaction impact on pool reserves
2. Calculates drain percentage: `(old_reserve - new_reserve) / old_reserve`
3. Triggers alerts when:
   - Drain > 60% OR remaining < 0.3 ETH = Scam Alert
   - Drain > 50% = Critical severity
   - Drain > 20% = Liquidity warning

## Filtering and Cache Management

### Pool Filtering Criteria:
1. **Scam Tokens**: Automatically evicted when token marked as scam
2. **Low Liquidity**: Pools below minimum threshold (configurable)
3. **Stale Pools**: Optional cleanup for pools not updated recently

### Minimum Liquidity Thresholds:
- **WETH pools**: 0.01 ETH minimum (default)
- **Stablecoin pools**: Different thresholds may apply
- **Other denominations**: Configurable per currency

## Performance Characteristics

- **Update Frequency**: Every block with token changes (~12 seconds)
- **Message Size**: ~500 bytes per pool update
- **Latency**: Sub-millisecond from block processing to ZMQ publish
- **Cache Size**: Typically 2000-3000 active pools

## Configuration

### Environment Variables:
- `MIN_ETH_THRESHOLD`: Minimum ETH to keep pool in cache (default: 0.01)
- `ZMQ_PUB_ENDPOINT`: Publisher endpoint (default: tcp://*:5557)
- `ZMQ_REP_ENDPOINT`: Reply endpoint (default: tcp://*:5558)

## Integration Example

### Python Side - Publishing Updates:
```python
# In LiveTokenTracker
extractor = TokenInfoExtractor(logger, use_pool_cache=True)
publisher = TokenInfoPublisher(min_eth_threshold=0.01)

# Process block updates
updated_pools = await extractor.update_token_info(updated_tokens, block_number)
await publisher.update_token_info(updated_pools)
```

### Rust Side - Consuming Updates:
```rust
// Subscribe to updates
let subscriber = ctx.socket(zmq::SUB)?;
subscriber.connect("tcp://localhost:5557")?;

// Process updates
match msg_type {
    "token_updates" => {
        for (pool_addr, pool_data) in updates {
            pool_cache.update(pool_addr, pool_data.denom_reserve);
            // Check mempool transactions against new state
        }
    }
}
```

## Future Improvements

1. **Denomination-aware Filtering**: Different thresholds for WETH vs stablecoins
2. **Pool Quality Metrics**: Track pool age, volume, creator reputation
3. **Compression**: Optional message compression for high-volume periods
4. **Historical Tracking**: Pool reserve trends for better predictions