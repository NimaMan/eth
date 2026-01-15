# Mempool Integration Requirements for eth_kartal 📊

## Overview

This document clearly defines what eth_kartal needs from the mempool_processor for gas optimization, replacing the current WebSocket-based approach with a more efficient API-based integration.

## Current Architecture (To Be Replaced)

eth_kartal currently uses:
- WebSocket connection to Reth node (`ws://127.0.0.1:8546`)
- Local mempool tracking with `BTreeMap<U256, Vec<PendingTransaction>>`
- Duplicated transaction monitoring logic

## Required Data from mempool_processor

### 1. **Gas Price Distribution** (Real-time)

```rust
struct GasPercentiles {
    p10: U256,   // 10th percentile
    p25: U256,   // 25th percentile  
    p50: U256,   // Median
    p75: U256,   // 75th percentile
    p90: U256,   // 90th percentile
    p95: U256,   // 95th percentile
    p99: U256,   // 99th percentile
}
```

**Usage**: Determine competitive gas prices for different urgency levels

### 2. **Position Calculation Functions**

```rust
// Given gas price → queue position
async fn get_position_for_gas_price(gas_price: U256) -> u64

// Given target position → required gas price
async fn get_gas_price_for_position(target_position: u64) -> U256
```

**Usage**: Calculate optimal gas price to achieve desired mempool position

### 3. **Congestion Metrics**

```rust
struct CongestionMetrics {
    pending_tx_count: u64,              // Total pending transactions
    arrival_rate_per_second: f64,       // Transaction arrival rate
    congestion_level: CongestionLevel,  // Low/Medium/High/Extreme
}
```

**Usage**: Adjust safety margins based on network congestion

### 4. **MEV Activity Indicators**

```rust
struct MevActivity {
    mev_tx_count: u64,           // Count of suspected MEV transactions
    avg_mev_gas_price: U256,     // Average gas price of MEV txs
    mev_percentage: f64,         // % of transactions that are MEV
}
```

**Usage**: Determine when to use Flashbots vs public mempool

## Integration Options

### Option 1: HTTP API (Recommended)

**Pros**:
- Simple integration
- No state management in eth_kartal
- Easy to scale and load balance
- Clear separation of concerns

**Cons**:
- Network latency (mitigated by <1ms response times)
- Requires HTTP client

**Implementation**:
```rust
// In eth_kartal
let gas_client = MempoolGasClient::new("http://localhost:8088");
let metrics = gas_client.get_gas_metrics().await?;
```

### Option 2: Direct Integration

**Pros**:
- Zero network overhead
- Direct access to all metrics

**Cons**:
- Tight coupling
- Requires running in same process
- Complex dependency management

**Implementation**:
```rust
// Would require significant refactoring
let collector = mempool_processor::get_global_gas_collector();
```

## API Requirements

### Performance Requirements

- **Response time**: <1ms for all queries
- **Update frequency**: Metrics updated every 100ms
- **Availability**: 99.9% uptime

### Data Freshness

- **Maximum staleness**: 200ms
- **Confidence scoring**: Based on data age and sample size

### Error Handling

```rust
enum GasMetricsError {
    NoDataAvailable,        // No transactions in buffer
    StaleData,             // Data older than threshold
    ServiceUnavailable,    // API unreachable
}
```

## Migration Path

### Phase 1: Add Gas Client (Current)
- Create `mempool_gas_client.rs` ✅
- Implement HTTP client for gas metrics
- Keep existing WebSocket as fallback

### Phase 2: Integration Testing
- Test gas optimization with new client
- Compare results with WebSocket approach
- Verify performance targets met

### Phase 3: Replace WebSocket
- Remove `mempool_tracker.rs` WebSocket code
- Update `gas_optimizer.rs` to use new client
- Remove duplicate transaction tracking

### Phase 4: Optimization
- Add caching layer (100ms TTL)
- Implement batch queries
- Add WebSocket for real-time updates (future)

## Success Criteria

1. **Performance**: Gas optimization <25ms (currently 0ms ✅)
2. **Accuracy**: Position predictions within ±5 positions
3. **Reliability**: No failed transactions due to gas pricing
4. **Simplicity**: Reduced code complexity vs WebSocket

## Example Usage in eth_kartal

```rust
// Before (WebSocket approach)
let mempool_tracker = MempoolTracker::new(ws_url);
mempool_tracker.start_monitoring().await?;
// ... complex WebSocket management ...

// After (Gas Metrics API)
let gas_client = MempoolGasClient::new(api_url);
let gas_price = gas_client.get_gas_price_for_position(10).await?;
// Simple, clean, efficient
```

## Summary

The mempool_processor now provides all required gas metrics through a high-performance, non-blocking API. eth_kartal can leverage this to:

1. **Eliminate WebSocket complexity**: No more connection management
2. **Improve performance**: <1ms queries vs 25ms+ WebSocket processing
3. **Reduce memory usage**: No duplicate mempool tracking
4. **Simplify codebase**: Clear API boundaries

The integration is designed to be drop-in replaceable with minimal changes to eth_kartal's gas optimization logic.