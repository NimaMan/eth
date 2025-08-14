# Token Tracking Cache Design Document

## Overview
The token tracking cache is a critical component that maintains real-time state of all tokens, pools, and creators in the Ethereum ecosystem. It serves as the central data store for making routing and signal detection decisions.

## Current Problems
1. **Redundant Cloning**: Excessive cloning of large data structures
2. **Inefficient Lookups**: O(n) searches through all tokens/pools
3. **Memory Waste**: No limits on token storage
4. **Type Confusion**: Multiple conflicting `TokenInfo` types
5. **Poor Data Locality**: Related data spread across multiple HashMaps

## Proposed New Architecture

### Core Principles
1. **Zero-Copy Access**: Return references or Arc-wrapped data
2. **Indexed Lookups**: O(1) access for all common queries
3. **Memory Bounded**: Limits on all collections with LRU eviction
4. **Single Source of Truth**: One canonical data structure per concept
5. **Data Locality**: Related data stored together

### Data Structure Design

```rust
pub struct TokenTrackingCache {
    // Primary storage - single source of truth
    tokens: Arc<RwLock<LruCache<Address, Arc<Token>>>>,
    pools: Arc<RwLock<LruCache<Address, Arc<Pool>>>>,
    
    // Indexed lookups for O(1) access
    creator_to_tokens: Arc<RwLock<HashMap<Address, HashSet<Address>>>>,
    token_to_pools: Arc<RwLock<HashMap<Address, HashSet<Address>>>>,
    pool_to_token: Arc<RwLock<HashMap<Address, Address>>>,
    
    // Pre-computed sets for fast filtering
    active_creators: Arc<RwLock<HashSet<Address>>>,
    active_pools: Arc<RwLock<HashSet<Address>>>,
    high_liquidity_pools: Arc<RwLock<HashSet<Address>>>,
    
    // Configuration
    config: CacheConfig,
}

pub struct Token {
    // Identity
    pub address: Address,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: U256,
    
    // Ownership
    pub creator: Address,
    pub current_owner: Address,
    pub tax_setters: Vec<Address>,
    pub ownership_renounced: bool,
    
    // Tax state
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tax_history: Vec<TaxChange>,
    pub pending_tax_changes: Vec<PendingTaxChange>,
    
    // Metadata
    pub creation_block: u64,
    pub creation_txn: H256,
    pub latest_activity_block: u64,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    
    // Cached computations
    pub primary_pool: Option<Address>,
    pub total_liquidity: f64,
}

pub struct Pool {
    // Identity
    pub address: Address,
    pub token_address: Address,
    pub pool_type: PoolType,
    
    // Reserves
    pub token_reserve: f64,
    pub eth_reserve: f64,
    pub denom_currency: String,
    pub denom_address: Address,
    
    // Trading state
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<u64>,
    pub trading_enabled_txn: Option<H256>,
    
    // Metadata
    pub fee_tier: Option<u32>,
    pub pool_id: Option<String>,
    pub last_updated_block: u64,
    pub last_updated_time: f64,
    pub is_scam: bool,
}
```

## Data Flow

### 1. Data Ingestion (Python → Rust)
```
Python Token Tracker
    ↓ [ZMQ PUB on port 5557]
TokenTrackingSubscriber
    ↓ [Deserialize & Transform]
TokenTrackingCache::batch_update()
    ↓ [Update indexes]
Pre-computed Sets & Indexes
```

### 2. Read Access Patterns

#### Transaction Routing (Hot Path)
```
MempoolTransaction arrives
    ↓
TxRouter::classify()
    ↓
cache.is_creator(from_address) → O(1) HashSet lookup
    ↓
cache.get_token_for_creator() → O(1) index lookup
    ↓
Route to simulation
```

#### Signal Detection
```
SimulationResult arrives
    ↓
SignalManager::process_simulation_result()
    ↓
cache.get_pools_for_token() → O(1) index lookup
    ↓
Generate signals for each pool
```

### 3. Update Flow
```
Python publishes update
    ↓
TokenTrackingSubscriber receives
    ↓
batch_update() acquires write lock once
    ↓
Updates primary storage
    ↓
Updates all indexes atomically
    ↓
Publishes update metrics
```

## API Design

### Zero-Copy Reads
```rust
// Returns Arc<Token> - no cloning
pub async fn get_token(&self, address: &Address) -> Option<Arc<Token>>

// Returns iterator - no allocation
pub async fn iter_creator_tokens(&self, creator: &Address) -> impl Iterator<Item = Arc<Token>>

// Returns reference to HashSet - no cloning
pub async fn creator_addresses(&self) -> RwLockReadGuard<HashSet<Address>>
```

### Efficient Queries
```rust
// O(1) lookups via indexes
pub async fn is_creator(&self, address: &Address) -> bool
pub async fn is_pool(&self, address: &Address) -> bool
pub async fn get_token_pools(&self, token: &Address) -> Vec<Arc<Pool>>
pub async fn get_primary_pool(&self, token: &Address) -> Option<Arc<Pool>>
```

### Batch Operations
```rust
// Single lock acquisition for multiple updates
pub async fn batch_update(&self, updates: TokenUpdates) -> UpdateResult
```

## Memory Management

### Limits
- Tokens: 10,000 max (LRU eviction)
- Pools: 100,000 max (LRU eviction)
- Indexes: Automatically pruned with primary storage

### Eviction Strategy
1. Scam tokens evicted first
2. Low activity tokens evicted next
3. High value tokens preserved

## Performance Targets
- `is_creator()`: < 1μs
- `get_token()`: < 5μs
- `get_pools_for_token()`: < 10μs
- Batch update (1000 items): < 100ms
- Memory usage: < 1GB for 10K tokens + 100K pools

## Migration Plan
1. Implement new cache structure alongside old
2. Update all read paths to use new API
3. Switch update path to new cache
4. Remove old cache implementation
5. Performance validation

## Testing Strategy
- Unit tests for each cache operation
- Concurrent access stress tests
- Memory limit validation
- Performance benchmarks
- Integration tests with signal detector