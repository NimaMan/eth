# QARQA Data Access

## 🎯 **Overview**

The Data Access module handles all database connections, queries, and data fetching operations for the QARQA analytics system. It provides high-performance access to blockchain data with optimized query patterns and robust error handling.

## 🏗️ **Architecture**

### **Module Structure**
```
data_access/
├── src/
│   ├── lib.rs                  # Public API exports
│   ├── database.rs             # Database connection management
│   ├── address_fetcher.rs      # Address-based transaction queries
│   ├── transaction_fetcher.rs  # Transaction data retrieval
│   └── block_fetcher.rs        # Block data access
├── tests/                      # Integration tests
└── Cargo.toml                  # Database dependencies
```

### **Core Principles**
- **Performance First**: O(1) lookups using optimized indexes
- **Connection Pooling**: Efficient database connection management
- **Error Resilience**: Graceful handling of database failures
- **Type Safety**: Leverages core_types for consistent data structures

## 🗄️ **Database Schema Requirements**

### **Core Tables**
```sql
-- Primary transaction storage
CREATE TABLE transactions (
    hash BYTEA PRIMARY KEY,
    from_address BYTEA NOT NULL,
    to_address BYTEA,
    value NUMERIC NOT NULL,
    gas_price BIGINT NOT NULL,
    gas_limit BIGINT NOT NULL,
    gas_used BIGINT NOT NULL,
    block_number BIGINT NOT NULL,
    transaction_index INTEGER NOT NULL,
    status SMALLINT NOT NULL,
    timestamp BIGINT NOT NULL,
    INDEX idx_tx_block (block_number),
    INDEX idx_tx_from (from_address),
    INDEX idx_tx_to (to_address),
    INDEX idx_tx_timestamp (timestamp)
);

-- O(1) address lookup optimization
CREATE TABLE participants (
    address BYTEA NOT NULL,
    transaction_hash BYTEA NOT NULL,
    direction SMALLINT NOT NULL, -- 0=from, 1=to, 2=internal
    block_number BIGINT NOT NULL,
    timestamp BIGINT NOT NULL,
    PRIMARY KEY (address, transaction_hash),
    INDEX idx_participants_address (address, block_number DESC),
    INDEX idx_participants_block (block_number),
    FOREIGN KEY (transaction_hash) REFERENCES transactions(hash)
);

-- Internal ETH transfers
CREATE TABLE internal_transfers (
    transaction_hash BYTEA NOT NULL,
    from_address BYTEA NOT NULL,
    to_address BYTEA NOT NULL,
    value NUMERIC NOT NULL,
    transfer_index INTEGER NOT NULL,
    transfer_type SMALLINT NOT NULL, -- 0=call, 1=create, 2=suicide
    PRIMARY KEY (transaction_hash, transfer_index),
    INDEX idx_internal_from (from_address),
    INDEX idx_internal_to (to_address),
    FOREIGN KEY (transaction_hash) REFERENCES transactions(hash)
);

-- ERC-20 token transfers
CREATE TABLE token_transfers (
    transaction_hash BYTEA NOT NULL,
    token_address BYTEA NOT NULL,
    from_address BYTEA NOT NULL,
    to_address BYTEA NOT NULL,
    value NUMERIC NOT NULL,
    transfer_index INTEGER NOT NULL,
    PRIMARY KEY (transaction_hash, transfer_index),
    INDEX idx_token_contract (token_address),
    INDEX idx_token_from (from_address),
    INDEX idx_token_to (to_address),
    FOREIGN KEY (transaction_hash) REFERENCES transactions(hash)
);
```

### **Performance Indexes**
```sql
-- Critical indexes for O(1) performance
CREATE INDEX CONCURRENTLY idx_participants_address_time 
    ON participants (address, timestamp DESC);

CREATE INDEX CONCURRENTLY idx_tx_hash_block 
    ON transactions (hash, block_number);

CREATE INDEX CONCURRENTLY idx_internal_addresses 
    ON internal_transfers (from_address, to_address);
```

## 📊 **Core Components**

### **1. Database Manager (`database.rs`)**
Handles connection pooling and database lifecycle management.

```rust
use qarqa_data_access::DatabaseManager;

// Initialize with connection pooling
let db_manager = DatabaseManager::new()
    .with_connection_string("postgresql://user:pass@localhost/eth_db")
    .with_pool_size(10)
    .with_timeout(Duration::from_secs(30))
    .build()
    .await?;

// Get connection from pool
let mut conn = db_manager.get_connection().await?;

// Automatic connection return to pool on drop
```

**Features:**
- **Connection Pooling**: Up to 20 concurrent connections
- **Health Checking**: Automatic connection validation
- **Retry Logic**: Exponential backoff for failed connections
- **Graceful Degradation**: Falls back to readonly operations

### **2. Address Fetcher (`address_fetcher.rs`)**
Provides O(1) address-based transaction lookups using the `participants` table.

```rust
use qarqa_data_access::AddressFetcher;

let fetcher = AddressFetcher::new(db_manager);

// Get all transactions for an address (fast O(1) lookup)
let transactions = fetcher
    .get_transactions_for_address(&address)
    .await?;

// Get transactions in time range
let recent_txs = fetcher
    .get_transactions_for_address_in_range(
        &address,
        start_timestamp,
        end_timestamp
    )
    .await?;

// Get transaction counts for multiple addresses
let counts = fetcher
    .get_transaction_counts(&addresses)
    .await?;
```

**Performance Characteristics:**
- **O(1) Lookups**: Uses `participants` table index
- **Batch Operations**: Process multiple addresses efficiently
- **Memory Efficient**: Streaming results for large datasets
- **Time Range Filtering**: Fast timestamp-based filtering

### **3. Transaction Fetcher (`transaction_fetcher.rs`)**
Retrieves complete transaction data with internal transfers and token movements.

```rust
use qarqa_data_access::TransactionFetcher;

let fetcher = TransactionFetcher::new(db_manager);

// Get complete transaction with all transfers
let tx = fetcher
    .get_transaction_complete(&tx_hash)
    .await?;

// Batch fetch multiple transactions
let transactions = fetcher
    .get_transactions_batch(&tx_hashes)
    .await?;

// Get transactions in block range
let block_txs = fetcher
    .get_transactions_in_block_range(start_block, end_block)
    .await?;
```

**Data Completeness:**
- **Main Transaction**: Basic transaction data
- **Internal Transfers**: All internal ETH movements
- **Token Transfers**: All ERC-20 token movements
- **Metadata**: Block information, timestamps, gas costs

### **4. Block Fetcher (`block_fetcher.rs`)**
Handles block-level data access and statistics.

```rust
use qarqa_data_access::BlockFetcher;

let fetcher = BlockFetcher::new(db_manager);

// Get latest processed block
let latest = fetcher.get_latest_block().await?;

// Get block statistics
let stats = fetcher.get_block_stats(block_number).await?;

// Check if block exists
let exists = fetcher.block_exists(block_number).await?;
```

## 🛠️ **Usage Examples**

### **Basic Address Analysis**
```rust
use qarqa_data_access::*;
use qarqa_core_types::*;

#[tokio::main]
async fn main() -> QarqaResult<()> {
    // Initialize database connection
    let db = DatabaseManager::new()
        .with_connection_string("postgresql://localhost/eth_db")
        .build()
        .await?;
    
    let address_fetcher = AddressFetcher::new(db.clone());
    let tx_fetcher = TransactionFetcher::new(db);
    
    // Analyze an address
    let address = "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7".parse()?;
    
    // Get transaction count (fast)
    let tx_count = address_fetcher
        .get_transaction_count(&address)
        .await?;
    
    println!("Address has {} transactions", tx_count);
    
    // Get recent transactions
    let recent_txs = address_fetcher
        .get_transactions_for_address_in_range(
            &address,
            1698000000, // Start timestamp
            1698086400  // End timestamp (24 hours later)
        )
        .await?;
    
    // Analyze each transaction
    for tx_hash in recent_txs {
        let complete_tx = tx_fetcher
            .get_transaction_complete(&tx_hash)
            .await?;
        
        println!(
            "Transaction {}: {} ETH, {} internal transfers, {} token transfers",
            format_address_short(&tx_hash),
            wei_to_eth(complete_tx.value),
            complete_tx.internal_transfers.len(),
            complete_tx.token_transfers.len()
        );
    }
    
    Ok(())
}
```

### **Batch Address Processing**
```rust
use qarqa_data_access::*;

async fn analyze_multiple_addresses(
    addresses: Vec<Address>
) -> QarqaResult<HashMap<Address, u64>> {
    let db = DatabaseManager::new()
        .with_connection_string("postgresql://localhost/eth_db")
        .build()
        .await?;
    
    let fetcher = AddressFetcher::new(db);
    
    // Batch process for efficiency
    let counts = fetcher
        .get_transaction_counts(&addresses)
        .await?;
    
    // Filter active addresses
    let active_addresses: HashMap<Address, u64> = counts
        .into_iter()
        .filter(|(_, count)| *count > 10)
        .collect();
    
    Ok(active_addresses)
}
```

### **Time-Series Analysis**
```rust
use qarqa_data_access::*;
use std::collections::HashMap;

async fn daily_transaction_volume(
    address: &Address,
    start_day: u64,
    end_day: u64
) -> QarqaResult<HashMap<u64, f64>> {
    let db = DatabaseManager::new()
        .with_connection_string("postgresql://localhost/eth_db")
        .build()
        .await?;
    
    let address_fetcher = AddressFetcher::new(db.clone());
    let tx_fetcher = TransactionFetcher::new(db);
    
    let mut daily_volumes = HashMap::new();
    
    for day in start_day..=end_day {
        let day_start = day * 86400; // Convert day to timestamp
        let day_end = day_start + 86400;
        
        // Get transactions for this day
        let tx_hashes = address_fetcher
            .get_transactions_for_address_in_range(
                address,
                day_start,
                day_end
            )
            .await?;
        
        // Calculate total volume
        let mut total_volume = 0.0;
        for tx_hash in tx_hashes {
            let tx = tx_fetcher
                .get_transaction_complete(&tx_hash)
                .await?;
            
            total_volume += wei_to_eth(tx.value);
            
            // Add internal transfer volumes
            for internal in &tx.internal_transfers {
                total_volume += wei_to_eth(internal.amount);
            }
        }
        
        daily_volumes.insert(day, total_volume);
    }
    
    Ok(daily_volumes)
}
```

## 🧪 **Testing**

### **Running Tests**
```bash
cd /home/nima/code/crypto/rust/qarqa/data_access
cargo test
```

### **Integration Tests**
```bash
# Test with real database (requires setup)
cargo test --features integration-tests

# Test with mock database
cargo test --features mock-db
```

### **Test Coverage**
- ✅ **Connection Management**: Pool creation, health checks, cleanup
- ✅ **Address Queries**: O(1) lookup performance, batch operations
- ✅ **Transaction Fetching**: Complete data retrieval, error handling
- ✅ **Error Scenarios**: Database failures, connection timeouts, invalid queries

### **Performance Tests**
```rust
#[tokio::test]
async fn test_address_lookup_performance() {
    let db = setup_test_db().await;
    let fetcher = AddressFetcher::new(db);
    
    let start = Instant::now();
    
    // Test O(1) performance with 1000 lookups
    for _ in 0..1000 {
        let _ = fetcher
            .get_transaction_count(&random_address())
            .await
            .unwrap();
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(100)); // < 0.1ms per lookup
}
```

## ⚡ **Performance Optimization**

### **Query Patterns**
```sql
-- Optimized address lookup (O(1))
SELECT transaction_hash 
FROM participants 
WHERE address = $1 
ORDER BY timestamp DESC 
LIMIT 1000;

-- Batch address counts (single query)
SELECT address, COUNT(*) 
FROM participants 
WHERE address = ANY($1) 
GROUP BY address;

-- Time-range filtering (uses index)
SELECT transaction_hash 
FROM participants 
WHERE address = $1 
  AND timestamp BETWEEN $2 AND $3 
ORDER BY timestamp DESC;
```

### **Connection Pooling Configuration**
```rust
let db = DatabaseManager::new()
    .with_pool_size(20)           // Max connections
    .with_idle_timeout(600)       // 10 minute idle timeout
    .with_connection_timeout(30)  // 30 second connection timeout
    .with_retry_attempts(3)       // Retry failed connections
    .build()
    .await?;
```

### **Memory Management**
```rust
// Use streaming for large result sets
let mut stream = address_fetcher
    .get_transactions_stream(&address)
    .await?;

while let Some(tx_hash) = stream.next().await {
    // Process one transaction at a time
    process_transaction(tx_hash?).await?;
}
```

## 📈 **Performance Characteristics**

| Operation | Time Complexity | Typical Performance |
|-----------|----------------|-------------------|
| Address Lookup | O(1) | <1ms |
| Transaction Fetch | O(1) | <5ms |
| Batch Address Counts | O(n) | <10ms for 100 addresses |
| Time Range Query | O(log n) | <10ms for 1 week range |
| Connection Pool Get | O(1) | <0.1ms |

## 🔗 **Integration with Other Modules**

### **Used By**
- **tx_simulation**: Fetches transaction data for simulation
- **network_building**: Retrieves transaction data for network construction
- **api_layer**: Provides data for CLI and API responses

### **Dependencies**
- **qarqa_core_types**: Uses Transaction, Address, and error types
- **tokio-postgres**: PostgreSQL async driver
- **bb8**: Connection pooling
- **tracing**: Structured logging

## 🔄 **Development Guidelines**

### **Adding New Queries**
1. Design for performance - use existing indexes
2. Add comprehensive error handling
3. Include integration tests
4. Document expected performance characteristics
5. Consider connection pool impact

### **Database Migrations**
1. Use migration scripts for schema changes
2. Test with production-size datasets
3. Ensure backward compatibility
4. Monitor performance impact

### **Error Handling Best Practices**
1. Use `QarqaResult<T>` for all database operations
2. Log errors with structured context
3. Implement retry logic for transient failures
4. Provide fallback behaviors where possible

This module provides fast, reliable access to blockchain data, enabling real-time analysis and high-performance queries across the QARQA analytics system.