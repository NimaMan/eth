# Token Tracking Module

Real-time cache system for token, pool, and creator state management with Python-Rust synchronization via ZMQ.

## Current Architecture

### Data Flow
```
Python Token Tracker (postgresql + chain state)
    ↓ [ZMQ PUB on port 5557 - real-time updates]
    ↓ [ZMQ REP on port 5558 - initial bulk load]
TokenTrackingSubscriber 
    ↓ [Deserialize with field aliasing]
TokenTrackingCache
    ├── tokens: HashMap<String, TokenInfo>      # 368 tokens
    ├── pools: PoolStateCache                   # 379 pools  
    ├── all_creators: HashSet<String>           # 289 creators
    └── all_pools: HashSet<String>              # Pool addresses
```

### Critical Usage Points

#### 1. Transaction Routing (Hot Path - 12k tx/sec)
```rust
// tx_router.rs - Determines if transaction is from token creator
if cache.is_creator(&from_addr).await {  // O(1) HashSet lookup
    // Route as CreatorTransaction for simulation
    if let Some(token) = cache.get_token_for_creator(&from_addr).await {
        // Categorize transaction type
    }
}
```

#### 2. Signal Detection
```rust
// signal_manager.rs - Process simulation results
let pools = token_cache.get_pools_for_token(&token).await;  // O(n) - loads ALL pools!
for (pool_addr, pool_state) in pools {
    // Generate signals per pool
    if pool_state.trading_enabled {
        // Generate TAX_SIGNAL
    }
}
```

#### 3. Liquidity Detection
```rust
// liquidity_detector.rs - Check pool reserves
if let Some(pool) = cache.pools.get_pool(&pool_addr).await {
    if pool.eth_reserve < threshold {
        // Generate LIQUIDITY_REMOVAL signal
    }
}
```

## Current Problems

### Performance Issues
1. **Excessive Cloning**: `get_token()` returns cloned TokenInfo (>1KB per call)
2. **O(n) Searches**: `get_pools_for_token()` loads ALL 100K pools then filters
3. **Lock Contention**: Repeated lock acquisition in loops
4. **Memory Waste**: No limit on tokens HashMap (could grow unbounded)

### Data Consistency Issues
1. **Field Mismatches**: Python sends `denom_reserve`, Rust expects `eth_reserve`
2. **Missing Fields**: `token_address` not provided in nested pools
3. **Type Confusion**: Two different `TokenInfo` types (cache.rs vs types.rs)

### Measured Impact
- Without cache: 0 simulations, 0 signals in 12 minutes
- With cache: 2-5 simulations/sec, proper signal generation
- Cache load time: ~3 seconds for 368 tokens from Python

## Field Mapping (Python → Rust)

### Pool Fields
```
Python                  → Rust (with serde aliases)
denom_reserve          → eth_reserve
latest_block_number    → last_updated_block  
last_update_time       → last_updated_time
pool_address           → (nested, no token_address field)
```

### Token Fields
```
Python sends complete TokenInfo with:
- creator_address, tax_setter_addresses
- current_buy_tax, current_sell_tax (0-100 range)
- pools: HashMap<pool_address, PoolInfo>
- tax_history: Vec<TaxChange>
```

## Cache Statistics (Production)
- **Tokens**: 368 active tokens
- **Pools**: 379 pools (368 above 0.1 ETH threshold)
- **Creators**: 289 unique addresses (creators + owners + tax setters)
- **Memory Usage**: ~50MB for full cache
- **Update Frequency**: Block-level updates from Python (~12 sec)

## API Usage Examples

### Check if address is creator (FAST - O(1))
```rust
if token_cache.is_creator(&address).await {
    // This is a token creator/owner/tax setter
}
```

### Get token information (SLOW - clones entire struct)
```rust
if let Some(token) = token_cache.get_token(&token_address).await {
    // Access token.creator_address, token.buy_tax, etc
}
```

### Get pools for token (VERY SLOW - O(n))
```rust
let pools = token_cache.get_pools_for_token(&token_address).await;
// Returns Vec<(String, PoolState)> - all pools for this token
```

### Token metadata helpers
```rust
use alloy_primitives::Address;
use mempool_processor::token_tracking::token_parameter_extraction::fetch_token_metadata;
use reth_chain_query::provider::RethQueryProvider;

async fn describe_token(provider: &RethQueryProvider, token: Address) {
    if let Ok(meta) = fetch_token_metadata(provider, token, None).await {
        tracing::info!(
            "Token {} ({}): decimals={} total_supply={}",
            meta.name, meta.symbol, meta.decimals, meta.total_supply
        );
    }
}
```

## Configuration

### ZMQ Endpoints
- **SUB**: tcp://localhost:5557 (real-time updates from Python)
- **REQ**: tcp://localhost:5558 (initial bulk load request)

### Cache Limits
- **Pools**: 100,000 max (LRU eviction)
- **Tokens**: Unlimited (PROBLEM - should be bounded)
- **ETH Threshold**: 0.1 ETH (pools below this are ignored)

## Testing

### Verify Cache Population
```bash
# Run the CSV logger to check cache contents
cargo run --example token_pool_csv_logger

# Output: token_pool_data_YYYYMMDD_HHMMSS.csv
# Should show 368 tokens, 379 pools
```

### Check Signal Detection
```bash
# Monitor signal generation
tail -f mempool_processor/logs/signal_detector_*/signals/tax_signals.log
```

## Known Issues

1. **Startup Dependency**: Must wait for Python publisher to be running
2. **Field Evolution**: Python schema changes break Rust deserialization
3. **No Backpressure**: Can't handle Python sending faster than processing
4. **Memory Growth**: Token HashMap has no eviction policy

## Future Improvements

Planned refactors focus on cutting copy costs, tightening query paths, and putting a hard ceiling on memory usage.

### Core Principles Under Review
- Zero-copy read paths (return `Arc` handles or iterators instead of cloning `TokenInfo`).
- Indexed O(1) lookups for creators, pools, and token relationships.
- Explicit collection limits with LRU-style eviction to cap memory.
- Single canonical type per concept (token, pool, creator) to avoid serde alias sprawl.
- Data locality: keep frequently accessed fields together to stay cache-friendly.

### Proposed Data Model
```rust
pub struct TokenTrackingCache {
    // Primary storage (bounded LRU caches)
    tokens: Arc<RwLock<LruCache<Address, Arc<Token>>>>,
    pools: Arc<RwLock<LruCache<Address, Arc<Pool>>>>,

    // Relationship indexes for O(1) lookups
    creator_to_tokens: Arc<RwLock<HashMap<Address, HashSet<Address>>>>,
    token_to_pools: Arc<RwLock<HashMap<Address, HashSet<Address>>>>,
    pool_to_token: Arc<RwLock<HashMap<Address, Address>>>,

    // Fast filters for hot-path checks
    active_creators: Arc<RwLock<HashSet<Address>>>,
    active_pools: Arc<RwLock<HashSet<Address>>>,
    high_liquidity_pools: Arc<RwLock<HashSet<Address>>>,

    config: CacheConfig,
}
```

Tokens and pools themselves would be richer typed structs (ownership, taxes, liquidity metadata, etc.) kept behind `Arc` so readers never clone >1KB payloads just to inspect a field.

### Update & Read Patterns (Target State)
```
Python publisher → TokenTrackingSubscriber → cache.batch_update()
    └─ single write lock updates storage + indexes + stats

Hot path routing:
TxRouter::classify() → cache.is_creator(from) → cache.get_token_for_creator(from)

Signal detection:
SimulationResult → SignalManager → cache.get_pools_for_token(token) (indexed set)
```

Batch updates would continue to hold the write lock once, refreshing both primary storage and derived indexes atomically while emitting lightweight metrics (block number, counts).

### Memory & Performance Targets
- Tokens: 10k max entries, Pools: 100k max entries (LRU with scam-first eviction).
- `is_creator()` in <1µs, `get_token()` in <5µs, `get_pools_for_token()` in <10µs.

Work on this redesign lives behind feature branches; keep the current API stable until the new cache is production-ready.
