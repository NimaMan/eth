# Fetch From Reth Module

## Overview

The `fetch_from_reth` module provides **RPC-free data access** to Ethereum blockchain data stored in a local Reth node's MDBX database. This module is critical for achieving sub-millisecond data retrieval times by bypassing network calls entirely and directly accessing Reth's memory-mapped database files.

## Architecture

```
fetch_from_reth/
├── fetch_from_reth.md          # This file - detailed architecture documentation
├── mod.rs                      # Module exports and public interface
├── provider.rs                 # RethDataProvider trait definition
├── cache.rs                    # Transaction caching implementation
├── db_impl.rs                  # Direct database implementation
├── simple_db.rs               # Simplified database access
├── fetch_from_reth_examples/   # Co-located examples
│   ├── fetch_from_reth_examples.md # Examples overview
│   ├── basic_usage.rs         # Getting started tutorial
│   └── performance_optimization.rs # Advanced optimization techniques
└── fetch_from_reth_tests/     # Co-located tests
    ├── fetch_from_reth_tests.md # Testing documentation
    ├── mod.rs                 # Test module exports
    ├── unit_tests.rs          # Unit tests with mocks
    ├── integration_tests.rs   # Real-world data patterns
    ├── performance_tests.rs   # Throughput and latency tests
    └── stress_tests.rs        # High-volume processing tests
```

## Quick Start

### Basic Usage
```bash
# Run the basic usage example
cargo run --example fetch_from_reth_basic_usage

# Run performance optimization example
cargo run --example fetch_from_reth_performance_optimization
```

### Running Tests
```bash
# Run all component tests
cargo test fetch_from_reth

# Run specific test categories
cargo test fetch_from_reth::tests::unit_tests
cargo test fetch_from_reth::tests::performance_tests
cargo test fetch_from_reth::tests::integration_tests

# Run stress tests (with --ignored flag)
cargo test fetch_from_reth::tests::stress_tests -- --ignored
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
- **Transaction Details**: Hash, from/to addresses, value, gas, nonce, input data
- **Receipt Information**: Status, gas used, contract address, logs
- **Block Context**: Block number, timestamp, base fee
- **Transaction Metadata**: Position in block, transaction type

### 4. **Key Imports and Dependencies**
```rust
// Core Reth imports for database access
use reth_ethereum::{
    chainspec::ChainSpecBuilder,
    node::{EthereumNode, api::NodeTypesWithDBAdapter},
    provider::{
        providers::{ReadOnlyConfig, BlockchainProvider, StaticFileProvider},
        db::{open_db_read_only, DatabaseArguments, ClientVersion, DatabaseEnv},
        AccountReader, BlockReader, HeaderProvider, ReceiptProvider, 
        TransactionsProvider, StateProviderFactory, ProviderFactory,
    },
    TransactionSigned,
};

// Primitive types from Alloy
use alloy_primitives::{Address, B256, BlockNumber, U256};

// Standard library
use std::{path::Path, sync::Arc};

// Error handling
use eyre::Result;
```


## Usage Examples

### Single Transaction Fetch
```rust
use crate::fetch_from_reth::{RethDatabaseProvider, RethDataProvider};
use reth_ethereum::{
    chainspec::ChainSpecBuilder,
    node::EthereumNode,
    provider::providers::ReadOnlyConfig,
};
use alloy_primitives::B256;
use std::path::Path;

// Open read-only connection to Reth's MDBX database (recommended approach)
let reth_datadir = Path::new("/path/to/reth/datadir");
let spec = ChainSpecBuilder::mainnet().build();

// Use Reth's provider factory builder for safe read-only access
let factory = EthereumNode::provider_factory_builder()
    .open_read_only(spec.into(), ReadOnlyConfig::from_datadir(reth_datadir))?;

let provider = RethDatabaseProvider::new(factory)?;
let tx_hash = B256::from_str("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060")?;
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
use reth_ethereum::{
    chainspec::ChainSpecBuilder,
    node::{EthereumNode, api::NodeTypesWithDBAdapter},
    provider::{
        providers::{ReadOnlyConfig, StaticFileProvider},
        db::{open_db_read_only, DatabaseArguments, ClientVersion, DatabaseEnv},
        ProviderFactory,
    },
};
use std::{path::Path, sync::Arc};

pub fn create_reth_provider(datadir: &Path) -> eyre::Result<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>> {
    // Method 1: Using provider factory builder (recommended)
    let spec = ChainSpecBuilder::mainnet().build();
    let factory = EthereumNode::provider_factory_builder()
        .open_read_only(spec.into(), ReadOnlyConfig::from_datadir(datadir))?;
    
    Ok(factory)
}

// Alternative method for direct database access
pub fn create_reth_provider_manual(datadir: &Path) -> eyre::Result<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>> {
    // Open read-only MDBX database
    let db = Arc::new(open_db_read_only(
        datadir.join("db").as_path(), 
        DatabaseArguments::new(ClientVersion::default())
    )?);
    
    // Create static file provider for older blocks
    let static_file_provider = StaticFileProvider::read_only(
        datadir.join("static_files").as_path(), 
        true // check_consistency
    )?;
    
    // Build chain specification  
    let spec = ChainSpecBuilder::mainnet().build();
    
    // Create provider factory
    let factory = ProviderFactory::<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>::new(
        db.clone(),
        spec.clone(),
        static_file_provider
    );
    
    Ok(factory)
}
```

## Data Access Patterns

### High-Level Provider APIs (Recommended)
```rust
use reth_ethereum::provider::{
    AccountReader, BlockReader, HeaderProvider, 
    ReceiptProvider, TransactionsProvider, StateProviderFactory
};
use alloy_primitives::{Address, B256, BlockNumber};

// Use Reth's provider abstractions for safety and performance
let provider = factory.provider()?;

// Transaction access
let tx_hash = B256::from_str("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060")?;
let tx = provider.transaction_by_hash(tx_hash)?;
let (tx, meta) = provider.transaction_by_hash_with_meta(tx_hash)?;
let receipt = provider.receipt_by_hash(tx_hash)?;

// Block access  
let block = provider.block(BlockNumber::from(18_000_000).into())?;
let block = provider.block_by_hash(block_hash)?;
let header = provider.header_by_number(18_000_000)?;

// State access at specific block
let state_provider = provider.state_by_block_number_or_tag(BlockNumber::from(18_000_000).into())?;
let address = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?; // WETH
let account = state_provider.basic_account(address)?;
let storage_key = B256::ZERO; // Storage slot 0
let storage = state_provider.storage(address, storage_key)?;
let code = state_provider.account_code(address)?;
```

### Direct Table Access (Advanced)
```rust
use reth_db_api::{table::Table, transaction::DbTx, cursor::DbCursorRO};
use reth_db::tables;

// For advanced users requiring direct table access
let provider = factory.provider()?;
let tx = provider.tx_ref()?;

// Access transactions table directly
let mut cursor = tx.cursor_read::<tables::Transactions>()?;

// Seek to specific transaction ID (not hash)
let tx_id = 1000u64.into();
if let Some((found_id, tx_data)) = cursor.seek_exact(tx_id)? {
    // Process raw transaction data
    println!("Found transaction at ID: {:?}", found_id);
}

// Access other tables
let mut header_cursor = tx.cursor_read::<tables::Headers>()?;
let mut receipt_cursor = tx.cursor_read::<tables::Receipts>()?;
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

## AI Development Notes

### Context Loading Strategy
When working with this component:
1. **Start with `mod.rs`** for public interface overview
2. **Check `fetch_from_reth_examples/basic_usage.rs`** for usage patterns
3. **Review `fetch_from_reth_tests/` directory** for behavior understanding
4. **Consult this `fetch_from_reth.md`** for detailed architecture

### Common Patterns
- Database connections use `Arc<RethDatabaseProvider>` for thread safety
- All operations return `Result<T, FetchError>` for error handling
- Batch operations provide significant performance improvements
- Caching is transparent and automatic
- Real transaction hashes from mainnet are used in examples and tests

### Integration Points
This component integrates with:
- **Database Layer**: Direct MDBX access via Reth providers
- **Caching Layer**: Transparent transaction caching with LRU eviction
- **Error Handling**: Comprehensive error types and recovery strategies
- **Performance Monitoring**: Built-in statistics and metrics collection

## Troubleshooting

### Common Issues
1. **Database Not Found**: Verify Reth datadir path
   ```bash
   # Check if Reth database exists
   ls -la ~/.local/share/reth/mainnet/db/
   ```

2. **Permission Denied**: Ensure read access to MDBX files
   ```bash
   # Fix permissions if needed
   chmod -R u+r ~/.local/share/reth/mainnet/db/
   ```

3. **Performance Issues**: Check cache configuration and batch sizes
   ```rust
   // Increase cache size for better performance
   let config = CacheConfig {
       max_size: 10_000,  // Increase from default
       ttl: Duration::from_secs(300),
   };
   ```

4. **Memory Usage**: Monitor virtual memory usage from MDBX mapping
   ```bash
   # Monitor memory usage
   ps aux | grep tx_processor
   pmap -x <PID> | grep -E "total|reth"
   ```

### Debug Commands
```bash
# Check component compilation
cargo check -p tx_processor --lib

# Run component tests with output
cargo test fetch_from_reth -- --nocapture

# Run performance benchmarks
cargo test fetch_from_reth::tests::performance_tests -- --nocapture

# Profile memory usage
cargo test fetch_from_reth::tests::stress_tests -- --ignored --nocapture
```

This module provides the **high-performance foundation** for the entire transaction processing pipeline, enabling direct access to Reth's blockchain data with minimal overhead and maximum reliability.
