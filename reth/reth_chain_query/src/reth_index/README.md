# RethIndex: High-Performance Indexing Layer for Reth

## Purpose

RethIndex is a complementary MDBX database that provides fast entity-centric (address-based) indexing for Reth. While Reth stores blockchain data optimized for node operation (sequential by block/transaction number), RethIndex provides the missing reverse indexes and pre-computed aggregations needed for analytical queries.

**Key Value**: Get information that would take extremely long to retrieve from Reth in milliseconds.

## Core Problem Statement

Reth stores blockchain data optimized for node operation:
- Transactions indexed by txumber
- Blocks indexed by BlockNumber  
- State indexed by Address

But cannot efficiently answer analytical queries like:
- "Which transactions did address X participate in?"
- "What is the trading history of address X?"
- "What are the aggregated metrics for address X?"

RethIndex provides these missing indexes and aggregations.

## Reth Database Tables Reference

Understanding reth's native tables is crucial for designing our analytics database efficiently. We want to avoid duplicating data and leverage reth's existing indexes.

### Block and Header Tables

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **CanonicalHeaders** | `BlockNumber` → `HeaderHash` | Maps block number to its canonical header hash |
| **HeaderNumbers** | `BlockHash` → `BlockNumber` | Reverse lookup: hash to block number |
| **Headers** | `BlockNumber` → `Header` | Full header data for each block |
| **HeaderTerminalDifficulties** | `BlockNumber` → `CompactU256` | Total difficulty at each block |
| **BlockBodyIndices** | `BlockNumber` → `StoredBlockBodyIndices` | Transaction range for each block: `{first_tx_num, tx_count}` |
| **BlockOmmers** | `BlockNumber` → `StoredBlockOmmers` | Uncle blocks data |
| **BlockWithdrawals** | `BlockNumber` → `StoredBlockWithdrawals` | Validator withdrawals post-merge |

### Transaction Tables

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **Transactions** | `txumber` → `TransactionSigned` | Full transaction data by sequential ID |
| **TransactionHashNumbers** | `TxHash` → `txumber` | Maps transaction hash to its number |
| **TransactionBlocks** | `txumber` → `BlockNumber` | Maps transaction to its block (key is the highest transaction ID in the block) |
| **TransactionSenders** | `txumber` → `Address` | Cached sender addresses for fast lookup |
| **Receipts** | `txumber` → `Receipt` | Transaction receipts including logs |

### State Tables

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **PlainAccountState** | `Address` → `Account` | Current account state (balance, nonce, etc.) |
| **PlainStorageState** | `(Address, StorageKey)` → `StorageEntry` | Current storage values |
| **Bytecodes** | `CodeHash` → `Bytecode` | Smart contract bytecode by hash |

### History Tables (for archive nodes)

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **AccountsHistory** | `ShardedKey<Address>` → `BlockNumberList` | Blocks where account changed |
| **StoragesHistory** | `StorageShardedKey` → `BlockNumberList` | Blocks where storage changed |
| **AccountChangeSets** | `(BlockNumber, Address)` → `AccountBeforeTx` | Account state before changes |
| **StorageChangeSets** | `(BlockNumber, Address, Key)` → `StorageEntry` | Storage state before changes |

### Merkle Trie Tables

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **HashedAccounts** | `Keccak256(Address)` → `Account` | Hashed addresses for merkle tree |
| **HashedStorages** | `(HashedAddress, HashedKey)` → `StorageEntry` | Hashed storage for merkle tree |
| **AccountsTrie** | `StoredNibbles` → `BranchNodeCompact` | Account merkle patricia trie |
| **StoragesTrie** | `(HashedAddress, Nibbles)` → `StorageTrieEntry` | Storage merkle patricia tries |

### Sync and Meta Tables

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **StageCheckpoints** | `StageId` → `StageCheckpoint` | Sync progress for each stage |
| **StageCheckpointProgresses** | `StageId` → `Vec<u8>` | Additional stage progress data |
| **PruneCheckpoints** | `PruneSegment` → `PruneCheckpoint` | Pruning progress tracking |
| **ChainState** | `ChainStateKey` → `BlockNumber` | Finalized/safe block tracking |
| **VersionHistory** | `Timestamp` → `ClientVersion` | Database version history |

### Key Insights for Our Design

1. **Transaction Numbering**: Reth uses sequential `txumber` (u64) as primary key, not hashes
2. **Block-Transaction Mapping**: `BlockBodyIndices` gives us transaction ranges per block
3. **No Address Index**: Reth has no reverse index from address to transactions (our main value-add)
4. **Efficient Lookups**: Can convert `TxHash → txumber` and `txumber → Block` easily
5. **State vs History**: Plain tables for current state, ChangeSets for historical

### Critical Tables for Integration

For our analytics database, we'll frequently need to query:
- `TransactionHashNumbers`: Convert hash from ProcessedTransaction to txumber
- `BlockBodyIndices`: Get transaction ranges for block-based queries
- `Transactions`: Fetch full transaction details when needed
- `TransactionBlocks`: Map transactions to blocks

## Conceptual Framework: How Tables Relate

### Reth's Data Organization Layers

Reth organizes blockchain data in distinct layers, each optimized for node operation but not analytics:

#### Layer 1: Block Structure
```
Block N
├── Headers table: Block metadata (timestamp, hash, parent)
├── BlockBodyIndices: Transaction range {first_tx_num: 1000, tx_count: 50}
├── BlockOmmers: Uncle blocks (rare)
└── BlockWithdrawals: Validator withdrawals (post-merge)
```

#### Layer 2: Transaction Data
```
Transaction Numbers 1000-1049 (Block N)
├── Transactions table: Full transaction data by txumber
├── TransactionHashNumbers: Hash → txumber lookup
├── TransactionSenders: txumber → From address (cached)
├── Receipts table: Logs, gas used, status
└── TransactionBlocks: txumber → BlockNumber (reverse lookup)
```

#### Layer 3: World State
```
Current State (at Block N)
├── PlainAccountState: Address → {balance, nonce, code_hash}
├── PlainStorageState: (Address, Key) → Value
└── Bytecodes: CodeHash → Contract bytecode
```

#### Layer 4: Historical State (Archive Nodes Only)
```
Historical Changes
├── AccountsHistory: Address → [block1, block2, ...] (when changed)
├── StoragesHistory: (Address, Key) → [block1, block2, ...]
├── AccountChangeSets: What accounts looked like before each block
└── StorageChangeSets: What storage looked like before each block
```

### What Reth Cannot Answer Efficiently

Despite having all blockchain data, reth's indexing structure cannot answer:

1. **Address-Centric Questions**:
   - "Which transactions did address X participate in?" 
   - Requires scanning ALL transactions (impossible at scale)

2. **Trading Analysis**:
   - "What is the trading history of address X?"
   - "What's the PnL for address X in token Y?"
   - Requires parsing all events across all transactions

3. **Aggregated Metrics**:
   - "How much gas has address X spent total?"
   - "What's the total volume for address X?"
   - Requires summing across many transactions

4. **Entity Recognition**:
   - "Is this address a DEX, CEX, or MEV bot?"
   - Requires behavioral pattern analysis

### How Our Analytics Tables Fill the Gaps

#### address_to_txs: The Missing Reverse Index
```
Reth Flow (Impossible):
Address X → ??? → [transactions]  ❌

Our Flow (O(1) lookup):
Address X → [tx_num1, tx_num2, ...] → Query reth for details ✅
```

**Integration Pattern**:
1. Get txumbers from our index: `analytics_db[address] = [1000, 1001, 1005]`
2. Fetch details from reth: `reth_db.Transactions[1000], reth_db.Transactions[1001]...`

#### trades: Aggregated Trading Data
```
Reth Flow (Expensive):
Address X → Scan all transactions → Parse all events → Aggregate swaps ❌

Our Flow (Pre-computed):
(Address X, Token Y) → {total_spent: 5.2 ETH, total_received: 1000 USDC, ...} ✅
```

**Integration Pattern**:
1. ProcessedTransaction identifies swap with block_number and tx_index
2. Calculate tx_number directly: `block.first_tx_num + tx_index` (via BlockBodyIndices)
3. Update our aggregated trade data
4. Store tx_number for traceability

**Note**: We can avoid TransactionHashNumbers lookup entirely by calculating txumber from block data!

#### address_metrics: Pre-computed Analytics
```
Reth Flow (Very Expensive):
Address X → Find all txs → Sum gas → Calculate volume → Detect patterns ❌

Our Flow (Instant):
Address X → {total_gas: 2.1 ETH, volume: 50K USD, entity_type: "MEV"} ✅
```

#### tokens & pools: Cached Metadata
```
Reth Flow (RPC calls required):
Token address → Call name(), symbol(), decimals() → Cache manually ❌

Our Flow (Local cache):
Token address → {name: "USDC", symbol: "USDC", decimals: 6} ✅
```

### Complete Query Flow Examples

#### Example 1: "Get recent trades for address 0x742d35Cc6639C0532fEa33b19B8B32ff59c3c7c4"

```
1. Address Query (Analytics):
   analytics_db.address_to_txs[0x742d...] = [1000, 1001, 1005, 2000]

2. Recent Filter (Analytics):
   latest(10) = [1001, 1005, 2000]

3. Transaction Details (Reth):
   reth_db.Transactions[1001] = {from: 0x742d..., to: Uniswap, ...}
   reth_db.Transactions[1005] = {from: 0x742d..., to: SushiSwap, ...}

4. Trade Data (Analytics):
   analytics_db.trades[(0x742d, WETH)] = {total_spent: 5.2 ETH, profit: +$200}
```

#### Example 2: "Find all PEPE traders with >$10K volume"

```
1. Token Query (Analytics):
   analytics_db.tokens[PEPE] = {symbol: "PEPE", decimals: 18}

2. Volume Filter (Analytics):
   Find all keys in trades table matching (*,PEPE,*) where total_volume > $10K

3. Address Details (Combined):
   For each address: get metrics from address_metrics + recent txs from address_to_txs
```

#### Example 3: "Block range query: transactions 20M-20.1M for address X"

```
1. Block to txumber Range (Reth):
   reth_db.BlockBodyIndices[20000000] = {first_tx_num: 500000000, tx_count: 200}
   reth_db.BlockBodyIndices[20100000] = {first_tx_num: 502000000, tx_count: 150}
   Range: 500000000..502000150

2. Address Transactions (Analytics):
   analytics_db.address_to_txs[0x742d...] = [499999000, 500050000, 501000000, 502500000]

3. Range Filter (In-memory):
   Filter list to [500050000, 501000000]  // Only txs in block range

4. Transaction Details (Reth):
   reth_db.Transactions[500050000] = {...}
   reth_db.Transactions[501000000] = {...}
```

### Key Architectural Insights

1. **Complementary Design**: Reth provides raw data, we provide indexes and aggregations
2. **No Data Duplication**: We store only txumbers (8 bytes) not full transactions (200+ bytes)  
3. **Hybrid Queries**: Most queries touch both databases for complete picture
4. **Write-Once, Read-Many**: Our tables are append-only during sync, read-heavy in production
5. **Atomic Consistency**: Both databases updated in same block processing loop

### Data Flow Integration

```
New Block Processing:
1. Reth processes block → Updates all reth tables
2. Our processor reads ProcessedTransactions
3. For each ProcessedTransaction:
   a. Calculate txumber = BlockBodyIndices[block_num].first_tx_num + tx_index
   b. Update address_to_txs[addr] += tx_number
   c. If swap: Update trades aggregation
   d. Update address_metrics counters
   e. Cache new tokens/pools discovered
4. Commit analytics transaction

Optimization: No need to query TransactionHashNumbers!
We calculate txumber directly from block data + tx index.
```

This design ensures our analytics database is always a few milliseconds behind reth but provides 10-100x faster queries for address-centric and aggregated data.

## Architecture Overview

Single MDBX environment with multiple named databases (tables), each serving a specific analytics purpose. Located alongside reth's database for optimal performance.

```
/home/nima/.local/share/reth/mainnet/
├── db/                        # Reth's blockchain database
└── reth_index/                # RethIndex database
    ├── data.mdb              # Main data file (~100GB)
    ├── lock.mdb              # Lock file for consistency
    └── [named databases within]
```

## Tables and Their Purposes

### 1. Address Transaction Index (`address_to_txs`)

**Purpose**: Enable fast lookup of all transactions involving a specific address

**Schema**:
- **Key**: `Address` (20 bytes)
- **Value**: `Vec<txumber>` (list of u64, sorted by block order)

**Queries Enabled**:
- "Get all transactions for address X" → O(1) lookup
- "Get transaction count for address X" → O(1) lookup
- "Get transactions in block range for address X" → O(1) lookup + filter

**Size Estimate**: ~70GB
- 87M addresses × average 100 transactions × 8 bytes per txumber

**Update Pattern**: Append-only (new transactions always added to end)

### 2. Trades Table (`trades`)

**Purpose**: Track aggregated trading activity per address-token pair

**Schema**:
- **Key**: `(Address, TokenAddress, Currency)` (20 + 20 + 1 = 41 bytes)
- **Value**: `TradeData` struct
  ```rust
  struct TradeData {
      entry_block: u64,        // First trade block
      latest_block: u64,       // Most recent trade block
      total_spent: U256,       // Total currency spent
      total_received: U256,    // Total tokens received
      realized_profit: i128,   // Realized PnL
      unrealized_profit: i128, // Unrealized PnL
      num_buys: u32,          // Buy transaction count
      num_sells: u32,         // Sell transaction count
      token_balance: U256,    // Current token balance
      gas_spent: U256,        // Total gas spent
  }
  ```

**Queries Enabled**:
- "Get trading history for address X in token Y"
- "Get PnL for address X across all tokens"
- "Get top traders for token Y"

**Size Estimate**: ~15GB

**Update Pattern**: Read-modify-write (aggregate on each trade)

### 3. Address Metrics Table (`address_metrics`)

**Purpose**: Store pre-computed metrics for fast address analysis

**Schema**:
- **Key**: `Address` (20 bytes)
- **Value**: `AddressMetrics` struct
  ```rust
  struct AddressMetrics {
      first_seen_block: u64,
      last_seen_block: u64,
      total_transactions: u64,
      total_volume_usd: f64,
      total_gas_spent: U256,
      total_profit: f64,
      scam_interactions: u32,
      is_contract: bool,
      entity_type: Option<String>,  // "DEX", "CEX", "MEV", etc.
  }
  ```

**Queries Enabled**:
- "Get address overview"
- "Find addresses by activity level"
- "Identify high-value addresses"

**Size Estimate**: ~10GB

**Update Pattern**: Read-modify-write (update on each transaction)

### 4. Tokens Table (`tokens`)

**Purpose**: Cache token metadata to avoid repeated RPC calls

**Schema**:
- **Key**: `TokenAddress` (20 bytes)
- **Value**: `TokenMetadata` struct
  ```rust
  struct TokenMetadata {
      name: String,
      symbol: String,
      decimals: u8,
      total_supply: U256,
      creator_address: Address,
      creation_block: u64,
      creation_tx: TxHash,
      is_scam: bool,
      scam_reason: Option<String>,
  }
  ```

**Queries Enabled**:
- "Get token information"
- "Find tokens by creator"
- "List all scam tokens"

**Size Estimate**: ~1GB

**Update Pattern**: Write-once (immutable after creation)

### 5. Pools Table (`pools`)

**Purpose**: Track DEX liquidity pools for trading analysis

**Schema**:
- **Key**: `PoolAddress` (20 bytes)
- **Value**: `PoolData` struct
  ```rust
  struct PoolData {
      token0: Address,
      token1: Address,
      pool_type: PoolType,      // V2, V3, V4
      fee_tier: u32,            // 100, 500, 3000, 10000
      creation_block: u64,
      creation_tx: TxHash,
      is_active: bool,
      total_volume: U256,
      last_activity_block: u64,
  }
  ```

**Queries Enabled**:
- "Get pool configuration"
- "Find pools for token X"
- "Get active pools by volume"

**Size Estimate**: ~1GB

**Update Pattern**: Write-once + activity updates

### 6. Mempool Arrivals Table (`mempool_arrivals`)

**Purpose**: Track when transactions first appear in mempool for timing analytics

**Schema**:
- **Key**: `TxHash` (32 bytes)
- **Value**: `u64` (Unix timestamp when first observed)

**Queries Enabled**:
- "How long did transaction X wait in mempool?"
- "Average mempool wait time for address X"
- "MEV bot detection via timing patterns"

**Size Estimate**: ~320KB (assuming ~10K pending txs on average)
- 10,000 txs × (32 bytes key + 8 bytes timestamp) = 400KB

**Update Pattern**: 
- Write on first mempool observation
- Delete after transaction inclusion (or after timeout)

**Integration with mempool_processor**:
```rust
// When mempool_processor sees a new transaction
mempool_processor.on_new_tx(tx_hash) -> mempool_arrivals[tx_hash] = timestamp

// When block arrives with the transaction
reth_index.on_tx_included(tx_hash, block_timestamp) -> {
    wait_time = block_timestamp - mempool_arrivals[tx_hash]
    update address_metrics with wait_time
    delete mempool_arrivals[tx_hash]
}
```

## Key Design Decisions

### Why MDBX?

1. **Same technology as reth**: Consistency in tooling and operations
2. **Memory-mapped I/O**: Direct memory access, no serialization overhead
3. **No separate service**: Embedded database, no network latency
4. **ACID transactions**: Atomic updates across multiple tables
5. **Proven reliability**: Battle-tested in reth for blockchain data

### Why These Specific Tables?

1. **address_to_txs**: Essential reverse index that reth cannot provide efficiently
2. **trades**: Aggregated data that would require scanning all transactions otherwise
3. **address_metrics**: Pre-computed values for instant dashboard queries
4. **tokens**: Frequently accessed metadata, avoid repeated RPC calls
5. **pools**: Critical for DEX analysis and trade routing

### Why No Address IDs?

Considered using sequential IDs (u32/u64) instead of addresses (20 bytes):

**Pros of IDs**:
- Save ~12 bytes per reference
- Slightly faster integer comparisons

**Cons of IDs**:
- Added complexity of bidirectional mapping
- Extra lookup step for every query
- Harder debugging (can't see addresses directly)
- Marginal space savings (70GB → 40GB)

**Decision**: Use direct addresses for simplicity and maintainability

### Why txumber Instead of TxHash?

- **Space**: 8 bytes vs 32 bytes (75% reduction)
- **Reth native**: txumber is reth's primary index
- **Sequential**: Natural ordering for range queries
- **Performance**: Integer comparison faster than hash comparison

## Data Flow

### Write Path

```
1. New block arrives in reth
   ↓
2. Block processor extracts ProcessedTransactions
   ↓
3. AnalyticsWriter opens write transaction
   ↓
4. For each ProcessedTransaction:
   a. Extract unique_addresses
   b. Update address_to_txs for each address
   c. If swap: update trades table
   d. Update address_metrics
   e. Add new tokens/pools if discovered
   ↓
5. Commit transaction atomically
```

### Read Path

```
1. Query request (e.g., "get txs for address X")
   ↓
2. Open read transaction
   ↓
3. Direct lookup in appropriate table
   ↓
4. Return data (may join with reth for details)
```

## Performance Characteristics

### Write Performance
- **Target**: 1000 blocks/second during initial sync
- **Bottleneck**: Disk I/O for large batch writes
- **Optimization**: Batch commits every N blocks

### Read Performance
- **Address lookup**: <1ms (single key lookup)
- **Trade query**: <1ms (single key lookup)
- **Range queries**: Linear in result size
- **Concurrent reads**: Unlimited (MVCC)

### Storage Efficiency
- **Total size**: ~100GB (vs 726GB PostgreSQL)
- **Growth rate**: ~50GB/year at current activity
- **Compression**: MDBX native compression available

## Integration Points

### Input Sources

1. **Primary**: ProcessedTransaction from tx_processor
   - Contains all participant addresses
   - Includes decoded events (swaps, transfers)
   - Has transaction classification

2. **Secondary**: Direct reth database queries
   - For backfilling historical data
   - For verification and consistency checks

### Output Consumers

1. **API Services**: REST/GraphQL endpoints
2. **Analytics Scripts**: Python/Rust analysis tools
3. **Real-time Monitors**: Alert systems
4. **Research Queries**: Ad-hoc analysis

## Operations and Maintenance

### Initial Build

```bash
# Build from genesis
cargo run --example build_analytics_db -- \
  --start-block 0 \
  --end-block latest \
  --batch-size 1000
```

### Incremental Updates

```bash
# Run continuously, processing new blocks
cargo run --bin analytics_indexer -- \
  --follow-head \
  --reth-db /path/to/reth/db \
  --analytics-db /path/to/analytics
```

### Verification

```bash
# Verify consistency with reth
cargo run --example verify_analytics_db -- \
  --sample-size 1000 \
  --check-addresses \
  --check-trades
```

### Backup Strategy

1. **Hot backup**: Copy MDBX files while running (safe with MVCC)
2. **Cold backup**: Stop writer, copy files, restart
3. **Incremental**: Track last_processed_block, replay from there

### Monitoring

Key metrics to track:
- Last processed block
- Write transaction latency
- Database file size
- Number of addresses indexed
- Query response times

## Future Extensions

### Potential New Tables

1. **contract_creations**: Track all deployed contracts
2. **nft_ownership**: Current NFT ownership state
3. **mev_transactions**: MEV bundle detection
4. **gas_analytics**: Gas price trends per address

### Potential Optimizations

1. **Bloom filters**: For existence checks before lookups
2. **Compression**: Custom compression for transaction lists
3. **Sharding**: Split by address range for parallel processing
4. **Caching layer**: Redis/memory cache for hot addresses

## Migration from PostgreSQL

### Advantages Over Current PostgreSQL Schema

1. **Space**: 100GB vs 726GB (86% reduction)
2. **Performance**: 10-100x faster for simple lookups
3. **Simplicity**: No SQL, no ORM, direct access
4. **Consistency**: Same transaction boundaries as reth
5. **Operations**: No separate database service

### Migration Plan

1. Run both systems in parallel initially
2. Verify data consistency
3. Migrate consumers one by one
4. Decommission PostgreSQL when confident

## Conclusion

This analytics database design provides:
- Essential indexes not available in reth
- Pre-computed aggregations for fast queries
- Efficient storage with MDBX
- Simple, maintainable architecture
- Clear extension path for future needs

Total storage: ~100GB (vs 726GB PostgreSQL)
Query performance: <1ms for point lookups
Write performance: 1000 blocks/second
Maintenance: Minimal (embedded database)