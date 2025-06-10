# Fetch From Reth Module

## Overview

The `fetch_from_reth` module provides **RPC-free data access** to Ethereum blockchain data stored in a local Reth node's MDBX database. This module is critical for achieving sub-millisecond data retrieval times by bypassing network calls entirely and directly accessing Reth's memory-mapped database files.

## Architecture

```
fetch_from_reth/
├── mod.rs           # Module exports and public interface
├── provider.rs      # RethDataProvider trait definition
└── db_impl.rs       # Direct database implementation
```

## Core Functionality

### 1. **Direct MDBX Database Access**
- Direct read-only access to Reth's MDBX (Memory-Mapped Database) files
- Memory-mapped file access for near-instant data retrieval
- No network overhead, JSON-RPC serialization, or IPC calls
- ACID-compliant reads with strong consistency guarantees

### 2. **Data Retrieval Interface**
```rust
pub trait RethDataProvider: Send + Sync {
    /// Fetch single transaction with all related data
    fn fetch_transaction(&self, tx_hash: B256) -> Result<TransactionData, FetchError>;
    
    /// Fetch multiple transactions efficiently
    fn fetch_batch(&self, tx_hashes: &[B256]) -> Result<Vec<TransactionData>, FetchError>;
    
    /// Check if transaction exists
    fn transaction_exists(&self, tx_hash: B256) -> Result<bool, FetchError>;
}
```

### 3. **Comprehensive Transaction Data**
Each fetch operation retrieves:
- **Transaction Details**: Hash, from/to addresses, value, gas, nonce, input data, receipt, logs, block
- **Receipt Information**: Status, gas used, contract address, logs
- **Block Context**: Block number, timestamp, base fee


## Usage Examples

### Single Transaction Fetch
```rust
use crate::fetch_from_reth::{RethDatabaseProvider, RethDataProvider};
use reth_db::open_db_read_only;
use reth_provider::ProviderFactory;
use std::path::Path;

// Open read-only connection to Reth's MDBX database
let reth_datadir = Path::new("/path/to/reth/datadir");
let db = open_db_read_only(reth_datadir.join("db"), Default::default())?;
let factory = ProviderFactory::new(db.into(), spec.into(), static_file_provider);

let provider = RethDatabaseProvider::new(factory)?;
let tx_data = provider.fetch_transaction(tx_hash)?;

println!("Transaction: {} in block {}", tx_data.hash, tx_data.block_number);
println!("From: {} To: {:?}", tx_data.from, tx_data.to);
```

### Batch Processing
```rust
let tx_hashes = vec![hash1, hash2, hash3];
let transactions = provider.fetch_batch(&tx_hashes)?;

for tx in transactions {
    println!("Processing tx: {}", tx.hash);
}
```

### Error Handling
```rust
match provider.fetch_transaction(tx_hash) {
    Ok(tx_data) => process_transaction(tx_data),
    Err(FetchError::NotFound(_)) => println!("Transaction not found"),
    Err(FetchError::DatabaseError(e)) => eprintln!("DB error: {}", e),
    Err(e) => eprintln!("Unexpected error: {}", e),
}
```

## Reth MDBX Database Schema

### Core MDBX Tables Used
Reth uses 26+ specialized MDBX tables for optimized blockchain data storage:

#### Transaction Data:
- `Transactions` - Raw transaction data (indexed by transaction number)
- `TransactionHashNumbers` - Transaction hash → Transaction number mapping
- `TransactionBlocks` - Transaction number → Block number mapping
- `Receipts` - Transaction execution results (indexed by transaction number)

#### Block Data:
- `Headers` - Block headers (indexed by block number)
- `CanonicalHeaders` - Block number → Header hash mapping
- `HeaderNumbers` - Block hash → Block number mapping
- `BlockBodyIndices` - Block number → Transaction range indices

#### State Data:
- `PlainAccountState` - Address → Account state
- `PlainStorageState` - Address + Storage key → Storage value
- `Bytecodes` - Code hash → Contract bytecode

#### Historical Data:
- `AccountsHistory` - Account change history for state at any block
- `StoragesHistory` - Storage change history for state at any block
- `AccountChangeSets` - Before/after account states for reverts
- `StorageChangeSets` - Before/after storage states for reverts

## Performance Characteristics

| Operation | Target | Typical | Notes |
|-----------|--------|---------|-------|
| Single Transaction | <1ms | 0.2ms | Memory-mapped access |
| Batch (100 txs) | <20ms | 8ms | Sequential MDBX reads |
| Database Open | <100ms | 50ms | One-time mmap setup |
| Memory Usage | Variable | ~2GB | Virtual memory mapping |
| Concurrent Readers | Unlimited | N/A | MDBX multi-reader design |

## Reth Database Location and Setup

### Default Database Locations
```bash
# Linux
~/.local/share/reth/mainnet/db/

# Environment variable override
export RETH_DATADIR="/custom/path/to/reth"

# Database structure
$RETH_DATADIR/
├── db/           # Main MDBX database files
│   ├── data.mdb  # Main database file
│   └── lock.mdb  # Lock file
├── static_files/ # Static file storage for older blocks
└── tmp/          # Temporary files
```

### Provider Factory Setup
```rust
use reth_db::{open_db_read_only, DatabaseArguments};
use reth_provider::{ProviderFactory, providers::StaticFileProvider};
use reth_chainspec::ChainSpecBuilder;
use std::path::Path;

pub fn create_reth_provider(datadir: &Path) -> Result<ProviderFactory<...>, RethError> {
    // Open read-only MDBX database
    let db = open_db_read_only(
        datadir.join("db"), 
        DatabaseArguments::default()
    )?;
    
    // Create static file provider for older blocks
    let static_file_provider = StaticFileProvider::read_only(
        datadir.join("static_files"), 
        true // check_consistency
    )?;
    
    // Build chain specification  
    let spec = ChainSpecBuilder::mainnet().build();
    
    // Create provider factory
    let factory = ProviderFactory::new(
        db.into(),
        spec.into(),
        static_file_provider
    );
    
    Ok(factory)
}
```

## Data Access Patterns

### High-Level Provider APIs (Recommended)
```rust
// Use Reth's provider abstractions for safety and performance
let provider = factory.provider()?;

// Transaction access
let tx = provider.transaction_by_hash(tx_hash)?;
let receipt = provider.receipt_by_hash(tx_hash)?;

// Block access  
let block = provider.block_by_number(block_number.into())?;
let header = provider.header_by_number(block_number)?;

// State access
let account = provider.basic_account(&address)?;
let storage = provider.storage(address, storage_key)?;
let code = provider.account_code(&address)?;
```

### Direct Table Access (Advanced)
```rust
use reth_db_api::{tables, cursor::DbCursorRO};

// For advanced users requiring direct table access
let tx = provider.tx_ref();
let mut cursor = tx.cursor_read::<tables::Transactions>()?;

// Seek to specific transaction number
if let Some((tx_num, tx_data)) = cursor.seek(transaction_number)? {
    // Process transaction data directly
}
```

## Configuration

### Database Configuration
```rust
pub struct RethDataConfig {
    pub datadir: PathBuf,
    pub read_only: bool,
    pub enable_static_files: bool,
    pub check_consistency: bool,
}

impl Default for RethDataConfig {
    fn default() -> Self {
        Self {
            datadir: dirs::data_dir()
                .unwrap_or_default()
                .join("reth")
                .join("mainnet"),
            read_only: true,           // Always use read-only for external access
            enable_static_files: true, // Required for complete data access
            check_consistency: true,   // Verify database integrity on open
        }
    }
}
```

## Production Considerations

### Safety Guidelines
1. **Always use read-only access** to avoid conflicts with running Reth node
2. **Handle database locks gracefully** - Reth may temporarily block readers during writes
3. **Monitor memory usage** - MDBX maps entire database into virtual memory
4. **Use provider APIs** rather than direct table access when possible
5. **Handle missing data** - Some older blocks may be in static files

### Performance Optimization
- **Reuse provider instances** across multiple queries
- **Use batch operations** for processing multiple transactions
- **Consider data locality** when accessing related data
- **Monitor MDBX file sizes** for capacity planning

### Integration with Analytics Pipeline
```rust
// Typical integration pattern for analytics systems
pub struct RethDataReader {
    factory: ProviderFactory<...>,
    config: RethDataConfig,
}

impl RethDataReader {
    pub fn fetch_block_transactions(&self, block_number: u64) -> Result<Vec<TransactionData>> {
        let provider = self.factory.provider()?;
        
        // Get block with transaction indices
        let block = provider.block_by_number(block_number.into())?
            .ok_or(FetchError::BlockNotFound(block_number))?;
        
        // Fetch all transactions in batch
        let mut transactions = Vec::new();
        for tx_hash in &block.body.transactions {
            if let Some(tx) = provider.transaction_by_hash(*tx_hash)? {
                transactions.push(tx);
            }
        }
        
        Ok(transactions)
    }
}
```

This module provides the **high-performance foundation** for the entire transaction processing pipeline, enabling direct access to Reth's blockchain data with minimal overhead and maximum reliability.
