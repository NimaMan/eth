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
2. **Lock Contention**: Repeated per-entry locking when expanding creator/pool views
3. **Staging Gap**: Newly created tokens with <0.1 ETH liquidity are dropped before we can simulate their first liquidity add (TradingEnabled delay)
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
    if let Ok(Some(meta)) = fetch_token_metadata(provider, token, None).await {
        tracing::info!(
            "Token {} ({}): decimals={} total_supply={}",
            meta.name, meta.symbol, meta.decimals, meta.total_supply
        );
    } else {
        tracing::info!("{token:?} is not an ERC-20 bytecode deployment");
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
- **Denom Thresholds**: Pools must clear the per-denomination liquidity thresholds defined in `eth_token/erc20_token/config/scam_thresholds.py` (fallback: 0.1 ETH when no mapping exists).

### Scam Handling
- Python flags pools with `pool.is_scam=true`; Rust now removes those pools from the active/high-liquidity indexes while keeping the token entry so trading-enabled detection still triggers on future liquidity.
- We maintain a `scam_pools` set in `TokenTrackingCache` so the router and simulators skip scam pools without evicting fresh deployments.
- `token.total_liquidity` excludes scam pools and only counts pools that clear the configured denomination thresholds, preventing zero-liquidity tokens from being dropped when other pools are flagged.


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

## Proposed Cache Redesign

### Goals
1. **Zero-delay signal readiness** – keep creator transactions routable as soon as a deployment is observed even if the pool has not accumulated liquidity yet.
2. **Pool-specific scam isolation** – drop rugged pools while keeping healthy pools and trading history for the same token.
3. **Predictable memory footprint** – bounded caches with explicit eviction policy.
4. **One-pass lookups** – eliminate repeated locking/cloning on the hot path (routing + simulation).

### Entities & Lifecycle
| Entity | States | Notes |
|--------|--------|-------|
| `TokenEntry` | `Pending`, `Active` | `Pending`: deployment seen but no viable pool yet (zero-liquidity stage). `Active`: at least one pool promoted (≥ threshold) or manually forced. |
| `PoolEntry` | `Pending`, `Active`, `Scam`, `Evicted` | `Pending`: exists but below liquidity threshold. `Active`: routed to simulators. `Scam`: marked by Python, excluded from routing. `Evicted`: removed during cache pressure. |

### Indexes
- `token_map`: token → `TokenEntry`
- `pool_map`: pool → `PoolEntry`
- `token_to_pools`: token → `HashSet<pool>` (includes pending & scam metadata)
- `creator_to_tokens`: creator/owner/tax-setter → `HashSet<token>`
- `scam_pools`: fast filter so routing/simulation ignores rugged pools without touching the token
- `pending_tokens`: queue for staging zero-liquidity deployments with TTL/backoff

All updates keep the indexes in sync inside `batch_update`.

### Update Flow
1. **Initial load** – hydrate the cache from Python’s REQ endpoint (baseline state).
2. **Realtime update** – per block diff:
   - Promote token from `Pending` → `Active` once any pool crosses the liquidity threshold or the simulator explicitly promotes it.
   - Record zero-liquidity pools in `Pending` instead of discarding them so the first add-liquidity helper can still be simulated.
   - When Python flags `pool.is_scam`, move the pool to the scam set, drop it from `active_pools` / `high_liquidity_pools`, and keep the token entry intact.
3. **Eviction policy** – preferentially evict (1) scam pools already surfaced to the user, (2) stale pending entries past TTL, (3) least-recently-used active pools/tokens.

### Hot-Path APIs
- `is_creator(addr)` → O(1) check (bitset / HashSet).
- `get_token_for_creator(addr)` → cheap lookup returning `Arc<TokenEntry>` without cloning.
- `get_pools_for_token(token)` → iterator over *active, non-scam* pools (pending/scam pools accessible via explicit APIs).
- `schedule_pool_promotion(pool)` → simulator hook to promote a pending pool after a successful buy/sell probe.

### Scam Handling
- Python remains the source of truth for scam detection.
- Rust stores scam pools in `scam_pools`, ensuring they are excluded from routing yet the token itself stays live.
- `token.total_liquidity` only counts non-scam pools so fresh deployments are not evicted when an older pool rugs.

### Next Steps
1. Implement pending-token/pool staging with TTL.
2. Rework cache APIs to expose iterators instead of cloning vectors.
3. Add simulator → cache promotion hook.
4. Add metrics covering token/pool state transitions (pending → active → scam).

### Token & Pool Status Alignment

Python currently emits `TokenStatusEnum` with the following lifecycle: `CREATION` → `PAIR_CREATION` → `TRADING_ENABLED`, plus terminal states `INACTIVE_SCAM` / `INACTIVE_OTHER`. Rust should mirror this with a strongly-typed enum (e.g. `TokenState::{Creation, PendingLiquidity, ActiveTrading, ScamInactive, OtherInactive}`) so both runtimes speak the same vocabulary.

For pools we will introduce a dedicated `PoolState` aligned to Python’s pool events:
- `Discovered`: contract deployed, zero liquidity.
- `LiquidityDeposited`: liquidity added but trading not yet verified.
- `Active`: buy/sell simulation succeeded (trading enabled).
- `Scam`: flagged by Python (`pool.is_scam`) or local detection (drain > threshold).
- `Evicted`: removed from cache after TTL / eviction policy.

State transitions originate from Python updates; Rust can promote a pool to `Active` after a successful simulator probe (`schedule_pool_promotion`). When Rust detects a scam event (liquidity drain), it applies `PoolState::Scam` and persists the information back to Python’s writer so the warehouse stays consistent.

This shared status model ensures:
1. **Deterministic routing** – `CreatorTransaction` detection uses `TokenState` to decide whether to schedule buy/sell simulations.
2. **Consistent pruning** – only pools in `PoolState::Scam` are removed from the routing indexes; tokens remain available for future liquidity recovery.
3. **Unified telemetry** – both Python logs and Rust metrics can report token/pool state transitions using the same enum values.

Implementation outline:
- Define `TokenState` and `PoolState` enums in Rust mirroring `TokenStatusEnum` and the proposed pool lifecycle.
- Update ZMQ payloads to include explicit `pool_state` alongside existing fields.
- Adjust Python’s `TokenStatusWriter` to persist per-pool state and to publish state changes promptly (creation, liquidity add, scam).
- Modify Rust’s `TokenTrackingCache` to store these enums and expose them through lightweight getters for routing (`get_active_pools`, `get_pending_pools`, etc.).

Coordinating the enum definitions between the repos (shared schema or protobuf) prevents future drift and simplifies cross-language analytics.
