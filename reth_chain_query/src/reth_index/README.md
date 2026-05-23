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

This section explains the broader indexing design. The current active
implementation is smaller: `RethIndexDB` opens `address_to_blocks` and
`mempool_tx_arrival_times`. References below to `trades`, `address_metrics`,
`tokens`, or `pools` are design examples, not currently active RethIndex DBIs.

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

#### address_to_blocks: The Candidate Block Reverse Index
```
Reth Flow (Impossible):
Address X → ??? → [processed blocks]  ❌

Our Flow (O(1) lookup):
Address X → [block1, block2, ...] → load/replay candidate blocks ✅
```

**Integration Pattern**:
1. Get candidate block numbers from our index: `analytics_db[address] = [25029968, 25029969, 25030001]`
2. Load processed blocks from cache, or replay the blocks from Reth if cache is missing
3. Filter the processed transactions inside those blocks for the exact address/pool/token condition

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
   analytics_db.address_to_blocks[0x742d...] = [25029968, 25029969, 25030001, 25030120]

2. Recent Filter (Analytics):
   latest(10) = [25029969, 25030001, 25030120]

3. Processed Block Details:
   load/replay blocks, then filter ProcessedTransactions whose participants include 0x742d...

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
   For each address: get metrics from address_metrics + candidate blocks from address_to_blocks
```

#### Example 3: "Block range query: transactions 20M-20.1M for address X"

```
1. Range Filter (Analytics):
   query address_to_blocks[address] and keep values in 20000000..20100000

2. Candidate Blocks (Analytics):
   analytics_db.address_to_blocks[0x742d...] = [19999900, 20005000, 20070000, 20125000]

3. Range Filter (In-memory):
   Filter list to [20005000, 20070000]  // Only candidate blocks in block range

4. Transaction Details:
   load/replay the candidate blocks and filter matching ProcessedTransactions
```

### Key Architectural Insights

1. **Complementary Design**: Reth provides raw data, we provide indexes and aggregations
2. **No Data Duplication**: We store only block numbers (8 bytes) not full transactions (200+ bytes)  
3. **Hybrid Queries**: Most queries touch both databases for complete picture
4. **Write-Once, Read-Many**: Our tables are append-only during sync, read-heavy in production
5. **Atomic Consistency**: Both databases updated in same block processing loop

### Data Flow Integration

```
New Block Processing:
1. Reth processes block → Updates all reth tables
2. Our processor reads ProcessedTransactions
3. For each ProcessedTransaction:
   a. Add transaction participants to the current block's per-block address set
   b. Update address_to_blocks[addr] += block_number once per address per block
   c. If swap: Update trades aggregation
   d. Update address_metrics counters
   e. Cache new tokens/pools discovered
4. Commit analytics transaction

Optimization: No need to query TransactionHashNumbers for the address index.
When a caller needs exact transactions, it replays the indexed candidate blocks.
```

This design ensures our analytics database is always a few milliseconds behind reth but provides 10-100x faster queries for address-centric and aggregated data.

## Architecture Overview

Single MDBX environment with multiple named databases (tables), each serving a specific analytics purpose. Located alongside reth's database for optimal performance.

```
/home/nima/storage/samsung8tb/ethereum/reth/
├── db/                        # Reth's blockchain database
└── reth_index/                # RethIndex database
    ├── data.mdb              # Main data file (~100GB)
    ├── lock.mdb              # Lock file for consistency
    └── [named databases within]
```

## Tables and Their Purposes

`RethIndexDB` currently opens two named MDBX databases. Other model/table files
exist in `src/reth_index/tables/` as older or planned contracts, but they are
not active until `RethIndexDB` creates a DBI for them and a writer/reader path
uses them.

### 1. Address Block Index (`address_to_blocks`)

**Purpose**: enable fast lookup of candidate processed blocks involving a
specific address.

**Schema**:
- **Key**: `Address` (20 bytes)
- **Value**: duplicate `block_number` values (u64, big-endian, sorted by MDBX
  dupsort)
- **Flags**: `DUP_SORT | DUP_FIXED`

**Write path**:
- `tx_processor::ProcessedBlockReplayStoreWriter` receives a full
  `ProcessedBlock`.
- `AddressBlockParticipationWriter` unions all transaction-level address
  participations for that block.
- It writes one `(address, block_number)` duplicate value per address.

**Read contract**:
- The table returns candidate blocks only.
- Callers must load/replay those blocks and filter exact transactions in block
  order.
- Replays are idempotent; duplicate `(address, block_number)` writes are skipped.

### 2. Mempool Transaction Arrival Times (`mempool_tx_arrival_times`)

**Purpose**: store first-seen local mempool arrival time for mined transactions.

**Schema**:
- **Key**: `tx_number` / Reth txumber (u64, big-endian)
- **Value**: `first_seen_ms` epoch milliseconds (u64, big-endian)
- **Flags**: `INTEGER_KEY`

**Write path**:
- `mempool_processor` keeps an in-memory `tx_hash -> first_seen_ms` map.
- After inclusion, `MempoolArrivalWriter` resolves `tx_hash -> tx_number` using
  Reth `TransactionHashNumbers`.
- Resolved rows are batch-written to this MDBX table.

**Read contract**:
- To query by hash, resolve the tx hash through Reth first, then look up the
  txumber in `mempool_tx_arrival_times`.
- Pre-inclusion hashes are not durably stored; a process restart before mining
  can lose in-flight arrival observations by design.

### Planned / Dormant Table Model Files

These table model modules still exist but are not currently opened by
`RethIndexDB`:

- `tokens`
- `pools`
- `trades`
- `address_metrics`

Treat them as design scaffolding until there is an active DBI, writer, reader,
and owner README entry.

## Key Design Decisions

### Why MDBX?

1. **Same technology as reth**: Consistency in tooling and operations
2. **Memory-mapped I/O**: Direct memory access, no serialization overhead
3. **No separate service**: Embedded database, no network latency
4. **ACID transactions**: Atomic updates across multiple tables
5. **Proven reliability**: Battle-tested in reth for blockchain data

### Why These Specific Tables?

1. **address_to_blocks**: Candidate-block reverse index that Reth cannot provide for processed transaction participation.
2. **mempool_tx_arrival_times**: Compact timing index keyed by Reth txumber, so arrival observations compose with Reth's native transaction numbering.

The older `trades`, `address_metrics`, `tokens`, and `pools` table-model files
are dormant design scaffolding. Do not describe them as active storage until
`RethIndexDB` opens those DBIs and a writer keeps them populated.

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
3. ProcessedBlockReplayStoreWriter writes the replay store
   ↓
4. For each processed block:
   a. Extract unique_addresses from every transaction
   b. Deduplicate by address for the block
   c. Update address_to_blocks for each address once
   ↓
5. Commit the MDBX write transaction
```

The mempool arrival path is separate: `mempool_processor` records first-seen
hashes in memory, resolves mined hashes to txumbers through Reth, then batch
writes `mempool_tx_arrival_times`.

### Read Path

```
1. Query request (e.g., "get candidate blocks for address X")
   ↓
2. Open read transaction
   ↓
3. Direct lookup in address_to_blocks
   ↓
4. Load/replay candidate processed blocks and filter exact transactions
```

## Performance Characteristics

The figures below are sizing targets/design estimates. Re-measure against the
active table set before using them for capacity planning.

### Write Performance
- **Target**: 1000 blocks/second during initial sync
- **Bottleneck**: Disk I/O for large batch writes
- **Optimization**: Batch commits every N blocks

### Read Performance
- **Address lookup**: <1ms (single key lookup)
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

1. **eth_chain_server activity APIs**: token/address activity-block lookup.
2. **Token/network analytics**: seed candidate processed blocks before exact filtering.
3. **Mempool timing examples**: arrival coverage/list/delete/write smoke tools.
4. **Research queries**: ad-hoc block-candidate lookup and replay workflows.

## Operations and Maintenance

### Initial Build / Incremental Updates

Build or refresh the active address index through the processed-block replay
store:

```bash
cargo run -p tx_processor --release --example refresh_processed_block_disk_cache -- \
  --blocks 100000
```

This fills missing `.pblock.zst` files and writes `address_to_blocks` unless
`--skip-address-block-index` is set. Existing cache hits are not rewritten.

### Verification

```bash
cargo run -p reth_chain_query --example tx_arrival_index_smoke
cargo run -p reth_chain_query --example list_tx_arrivals -- <reth_index_dir>
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
4. **Caching layer**: in-memory cache for hot addresses

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
