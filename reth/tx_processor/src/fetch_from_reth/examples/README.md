# Fetch From Reth - Comprehensive Data Access Guide

This module provides direct, high-performance access to all data stored in Reth's MDBX database, bypassing RPC entirely for maximum speed and efficiency.

## 🎯 Overview

The `fetch_from_reth` module gives you **complete access** to every piece of data stored in a Reth node's database:

- **Transactions**: Complete transaction data with receipts and metadata
- **Blocks**: Full block data including headers, bodies, and statistics  
- **Account States**: Balances, nonces, contract code, storage slots
- **Historical Data**: Account states and storage at any block height
- **Receipts**: Transaction receipts with event logs and gas usage
- **State Changes**: Track how accounts and storage change over time
- **Chain Metadata**: Block hashes, numbers, chain progression

## 📊 Performance Benefits

- **Sub-millisecond queries**: Direct MDBX access vs 100-500ms RPC calls
- **Batch operations**: Fetch multiple items efficiently
- **Caching**: Built-in LRU cache with TTL support
- **Memory-mapped files**: Zero-copy data access where possible
- **Concurrent access**: Thread-safe operations

## 🏗️ API Categories

### 1. Transaction Data Access

| Method | Input | Output | Description |
|--------|-------|--------|-------------|
| `fetch_transaction(tx_hash)` | `B256` (tx hash) | `TransactionData` | Complete transaction with receipt |
| `fetch_batch(tx_hashes)` | `&[B256]` | `Vec<TransactionData>` | Multiple transactions efficiently |
| `transaction_exists(tx_hash)` | `B256` | `bool` | Check existence without fetching |
| `fetch_transactions_by_block(block_id)` | `BlockId` | `Vec<TransactionData>` | All transactions in a block |
| `fetch_transaction_by_block_and_index(block, index)` | `u64, u64` | `TransactionData` | Transaction by position |

**TransactionData Fields:**
```rust
pub struct TransactionData {
    pub hash: B256,                    // Transaction hash
    pub from: Address,                 // Sender address
    pub to: Option<Address>,           // Recipient (None for contract creation)
    pub value: U256,                   // ETH amount in wei
    pub gas_limit: u64,                // Gas limit
    pub gas_used: u64,                 // Actual gas consumed
    pub gas_price: U256,               // Gas price in wei
    pub nonce: u64,                    // Sender nonce
    pub block_number: u64,             // Block containing transaction
    pub block_hash: B256,              // Block hash
    pub transaction_index: u64,        // Index within block
    pub input: Bytes,                  // Transaction data/input
    pub receipt_status: bool,          // Success/failure
    pub contract_address: Option<Address>, // Created contract address
    pub logs: Vec<Log>,                // Event logs
}
```

### 2. Block Data Access

| Method | Input | Output | Description |
|--------|-------|--------|-------------|
| `fetch_block(block_id)` | `BlockId` | `BlockData` | Complete block information |
| `fetch_blocks_batch(block_ids)` | `&[BlockId]` | `Vec<BlockData>` | Multiple blocks efficiently |
| `fetch_blocks_range(start, end)` | `u64, u64` | `Vec<BlockData>` | Range of consecutive blocks |
| `block_exists(block_id)` | `BlockId` | `bool` | Check if block exists |
| `latest_block_number()` | - | `u64` | Current blockchain height |
| `chain_info()` | - | `ChainInfo` | Overall chain state |

**BlockId Options:**
```rust
pub enum BlockId {
    Number(u64),        // Block by number
    Hash(B256),         // Block by hash
    Latest,             // Latest block
    Earliest,           // Genesis block
    Pending,            // Pending block
    Safe,               // Safe block
    Finalized,          // Finalized block
}
```

**BlockData Fields:**
```rust
pub struct BlockData {
    pub hash: B256,                    // Block hash
    pub number: u64,                   // Block number
    pub parent_hash: B256,             // Previous block hash
    pub state_root: B256,              // State merkle root
    pub transactions_root: B256,       // Transactions merkle root
    pub receipts_root: B256,           // Receipts merkle root
    pub timestamp: u64,                // Unix timestamp
    pub gas_limit: u64,                // Block gas limit
    pub gas_used: u64,                 // Total gas used
    pub difficulty: U256,              // Mining difficulty
    pub total_difficulty: Option<U256>, // Cumulative difficulty
    pub miner: Address,                // Block author/miner
    pub extra_data: Bytes,             // Extra block data
    pub transaction_count: usize,      // Number of transactions
}
```

### 3. Account State Access

| Method | Input | Output | Description |
|--------|-------|--------|-------------|
| `fetch_account(address)` | `Address` | `AccountData` | Complete account state |
| `fetch_account_at_block(address, block)` | `Address, BlockId` | `AccountData` | Historical account state |
| `fetch_accounts_batch(addresses)` | `&[Address]` | `Vec<AccountData>` | Multiple accounts |
| `fetch_balance(address)` | `Address` | `U256` | Account balance only |
| `fetch_balance_at_block(address, block)` | `Address, BlockId` | `U256` | Historical balance |
| `fetch_nonce(address)` | `Address` | `u64` | Account nonce |
| `fetch_code(address)` | `Address` | `Option<Bytes>` | Contract bytecode |

**AccountData Fields:**
```rust
pub struct AccountData {
    pub address: Address,              // Account address
    pub balance: U256,                 // Balance in wei
    pub nonce: u64,                    // Transaction nonce
    pub code_hash: B256,               // Hash of contract code
    pub code_size: Option<usize>,      // Size of contract code
    pub code: Option<Bytes>,           // Actual contract bytecode
    pub storage_root: B256,            // Storage merkle root
}
```

### 4. Storage Data Access

| Method | Input | Output | Description |
|--------|-------|--------|-------------|
| `fetch_storage(address, slot)` | `Address, B256` | `B256` | Storage slot value |
| `fetch_storage_at_block(address, slot, block)` | `Address, B256, BlockId` | `B256` | Historical storage |
| `fetch_storage_batch(address, slots)` | `Address, &[B256]` | `Vec<StorageData>` | Multiple slots |
| `fetch_storage_changes(address, start, end)` | `Address, u64, u64` | `Vec<StateChange>` | Storage evolution |

**StorageData Fields:**
```rust
pub struct StorageData {
    pub address: Address,              // Contract address
    pub key: B256,                     // Storage slot key
    pub value: B256,                   // Storage value
}
```

### 5. Receipt Data Access

| Method | Input | Output | Description |
|--------|-------|--------|-------------|
| `fetch_receipt(tx_hash)` | `B256` | `ReceiptData` | Transaction receipt |
| `fetch_receipts_batch(tx_hashes)` | `&[B256]` | `Vec<ReceiptData>` | Multiple receipts |
| `fetch_receipts_by_block(block_id)` | `BlockId` | `Vec<ReceiptData>` | All receipts in block |

**ReceiptData Fields:**
```rust
pub struct ReceiptData {
    pub transaction_hash: B256,        // Transaction hash
    pub transaction_index: u64,        // Index in block
    pub block_hash: B256,              // Block hash
    pub block_number: u64,             // Block number
    pub gas_used: u64,                 // Gas consumed
    pub cumulative_gas_used: u64,      // Total gas in block up to this tx
    pub status: bool,                  // Success/failure
    pub contract_address: Option<Address>, // Created contract
    pub logs: Vec<Log>,                // Event logs
    pub logs_bloom: Bloom,             // Bloom filter
}
```

### 6. Historical and State Change Access

| Method | Input | Output | Description |
|--------|-------|--------|-------------|
| `fetch_changed_accounts(start, end)` | `u64, u64` | `Vec<Address>` | Accounts modified in range |
| `fetch_account_state_changes(address, start, end)` | `Address, u64, u64` | `Vec<StateChange>` | Account evolution |
| `fetch_multiple_account_state_changes(addresses, start, end)` | `&[Address], u64, u64` | `Vec<StateChange>` | Multiple account changes |

**StateChange Fields:**
```rust
pub struct StateChange {
    pub block_number: u64,             // Block where change occurred
    pub address: Address,              // Changed address
    pub previous_state: Option<AccountData>, // Previous state
    pub new_state: AccountData,        // New state
    pub storage_changes: BTreeMap<B256, (Option<B256>, B256)>, // Storage diffs
}
```

### 7. Utility and Lookup Methods

| Method | Input | Output | Description |
|--------|-------|--------|-------------|
| `block_hash_to_number(hash)` | `B256` | `Option<u64>` | Hash → block number |
| `block_number_to_hash(number)` | `u64` | `Option<B256>` | Number → block hash |
| `transaction_hash_to_number(hash)` | `B256` | `Option<u64>` | Tx hash → internal ID |
| `fetch_transaction_sender(hash)` | `B256` | `Address` | Recover sender address |

## 🚀 Quick Start Examples

### Basic Setup
```rust
use revm_tx_simulator_lib::fetch_from_reth::*;

// Create provider with default configuration
let datadir = std::path::PathBuf::from("/path/to/reth/data");
let provider = RethDatabaseProvider::new(datadir)?;

// Or with custom cache configuration
let config = RethDataConfig::new(&datadir);
let cache_config = CacheConfig::high_performance();
let provider = RethDatabaseProvider::with_cache_config(config, cache_config)?;
```

### Fetch Transaction Data
```rust
use alloy_primitives::B256;

// Get transaction by hash
let tx_hash = B256::from_str("0x1234...")?;
let tx_data = provider.fetch_transaction(tx_hash)?;

println!("Transaction from: {:?}", tx_data.from);
println!("Transaction to: {:?}", tx_data.to);
println!("Value: {} ETH", wei_to_eth(tx_data.value));
println!("Gas used: {}", tx_data.gas_used);
println!("Success: {}", tx_data.receipt_status);
```

### Fetch Account State
```rust
use alloy_primitives::Address;

// Get current account state
let address = Address::from_str("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?;
let account = provider.fetch_account(address)?;

println!("Balance: {} ETH", wei_to_eth(account.balance));
println!("Nonce: {}", account.nonce);

if account.code_size.unwrap_or(0) > 0 {
    println!("Contract with {} bytes of code", account.code_size.unwrap());
} else {
    println!("Externally Owned Account (EOA)");
}
```

### Fetch Historical State
```rust
// Get account state at specific block
let block_100k = BlockId::Number(100_000);
let historical_account = provider.fetch_account_at_block(address, block_100k)?;

println!("Balance at block 100k: {} ETH", wei_to_eth(historical_account.balance));
```

### Fetch Block Data
```rust
// Get latest block
let latest_block = provider.fetch_block(BlockId::Latest)?;

println!("Block {}: {}", latest_block.number, hex::encode(&latest_block.hash.as_slice()[..4]));
println!("Timestamp: {}", latest_block.timestamp);
println!("Gas used: {}/{} ({:.1}%)", 
         latest_block.gas_used, 
         latest_block.gas_limit,
         (latest_block.gas_used as f64 / latest_block.gas_limit as f64) * 100.0);
```

### Batch Operations
```rust
// Fetch multiple transactions efficiently
let tx_hashes = vec![hash1, hash2, hash3];
let transactions = provider.fetch_batch(&tx_hashes)?;

// Fetch multiple accounts
let addresses = vec![addr1, addr2, addr3];
let accounts = provider.fetch_accounts_batch(&addresses)?;
```

## 📝 Available Examples

Run these examples to see the full capabilities:

### Address Data Access Example
```bash
cargo run --bin fetch_from_reth_address_data_access
```
**Demonstrates:**
- Complete account state queries
- Historical state access
- Contract storage reading
- Balance and nonce tracking
- Batch operations for addresses

### Block Data Access Example  
```bash
cargo run --bin fetch_from_reth_block_data_access
```
**Demonstrates:**
- Block header and metadata access
- Transaction enumeration within blocks
- Receipt analysis
- Block range operations
- Chain state validation

### Advanced Data Access Example
```bash
cargo run --bin fetch_from_reth_advanced_data_access
```
**Demonstrates:**
- Historical state evolution tracking
- Cross-address state correlations
- Transaction flow analysis
- Storage change monitoring
- Performance optimization techniques

### Handle Version Mismatch Example
```bash
cargo run --bin fetch_from_reth_handle_version_mismatch
```
**Demonstrates:**
- Detecting MDBX version mismatch errors
- Using different compatibility modes
- Fallback strategies for database access
- Troubleshooting guide for common issues
- Safe handling of version incompatibilities

## 🛠️ Configuration Options

### Database Configuration
```rust
let config = RethDataConfig::new(&datadir)
    .with_read_only(true)           // Read-only access (recommended)
    .with_static_files(true)        // Include static file data
    .with_metrics(true);            // Enable performance metrics
```

### Cache Configuration
```rust
let cache_config = CacheConfig::default()
    .with_max_size(10_000)          // Maximum cached items
    .with_ttl_seconds(300)          // 5-minute TTL
    .with_stats(true);              // Enable cache statistics

// Or use presets
let cache_config = CacheConfig::high_performance(); // Optimized for speed
let cache_config = CacheConfig::memory_conservative(); // Optimized for memory
```

## ⚡ Performance Tips

1. **Use Batch Operations**: Always prefer `fetch_batch()` over multiple `fetch_transaction()` calls
2. **Enable Caching**: Use appropriate cache configuration for your use case
3. **Specific Queries**: Use `fetch_balance()` instead of `fetch_account()` if you only need balance
4. **Block Ranges**: Use `fetch_blocks_range()` for consecutive blocks
5. **Read-Only Mode**: Always use read-only database access for safety and performance

## 🔧 Error Handling

All methods return `FetchResult<T>` which is `Result<T, FetchError>`:

```rust
match provider.fetch_transaction(tx_hash) {
    Ok(tx_data) => {
        // Process transaction data
        println!("Transaction found: {:?}", tx_data);
    }
    Err(FetchError::NotFound(msg)) => {
        println!("Transaction not found: {}", msg);
    }
    Err(FetchError::DatabaseError(msg)) => {
        println!("Database error: {}", msg);
    }
    Err(e) => {
        println!("Other error: {}", e);
    }
}
```

## 🎯 Use Cases

### Blockchain Analytics
- Track address activity and fund flows
- Analyze transaction patterns
- Monitor contract interactions
- Calculate gas usage statistics

### MEV Research  
- Identify arbitrage opportunities
- Track sandwich attacks
- Analyze liquidation events
- Monitor DEX trading patterns

### DeFi Monitoring
- Track pool state changes
- Monitor token transfers
- Analyze yield farming activities
- Calculate impermanent loss

### Security Analysis
- Detect unusual transaction patterns
- Monitor contract upgrades
- Track suspicious activities
- Analyze attack vectors

### Historical Research
- Reconstruct past states
- Analyze protocol evolution
- Study market events
- Performance benchmarking

---

This module provides the **complete foundation** for any blockchain data analysis, offering direct, high-performance access to every piece of data stored in your Reth node.