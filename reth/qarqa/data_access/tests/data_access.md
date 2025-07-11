# Data Access Testing Documentation

## Overview

The `data_access` module handles all database interactions for QARQA, providing high-performance data fetching with O(1) address lookups. This document specifies comprehensive testing requirements for database operations, connection management, and error handling.

## What We Are Testing

### 1. Database Connection Management

#### Connection Pool
- **Pool initialization**: Correct size and timeout configuration
- **Connection acquisition**: Fair scheduling under load
- **Connection recycling**: Proper cleanup after use
- **Pool exhaustion**: Graceful handling when no connections available
- **Connection health**: Automatic bad connection detection and removal

#### Connection Resilience
- **Network interruptions**: Automatic reconnection after network failures
- **Database restarts**: Recover from database server restarts
- **Slow queries**: Timeout handling for long-running queries
- **Connection leaks**: Detection and prevention of connection leaks
- **Concurrent access**: Thread-safe connection usage

### 2. Query Operations

#### Address Transaction Fetching
- **O(1) lookup performance**: Verify index usage on participants table
- **Block range filtering**: Correct start/end block queries
- **Result limiting**: Proper LIMIT clause application
- **Sort ordering**: Consistent ordering by block number
- **Large result sets**: Memory-efficient streaming

#### Transaction Data Fetching
- **Single transaction fetch**: By hash with all fields
- **Batch transaction fetch**: Multiple transactions efficiently
- **Missing transactions**: Proper None/empty handling
- **Data integrity**: All fields correctly deserialized
- **Join performance**: Efficient joins when needed

#### Block Data Fetching
- **Latest block queries**: Current chain head retrieval
- **Block range queries**: Efficient range scans
- **Timestamp queries**: Block by timestamp lookup
- **Missing blocks**: Handle gaps in block data
- **Reorg handling**: Detect and handle chain reorganizations

### 3. Error Handling

#### Database Errors
- **Connection failures**: Proper error types and messages
- **Query syntax errors**: Should never occur (prepared statements)
- **Constraint violations**: Handle unique/foreign key violations
- **Type mismatches**: Catch deserialization errors
- **Permission errors**: Handle access denied scenarios

#### Data Integrity
- **Null handling**: Proper Option<T> usage
- **Type conversions**: Safe parsing of database values
- **Encoding issues**: Handle UTF-8 and hex encoding
- **Truncation**: Detect data truncation
- **Precision loss**: Numeric precision preservation

### 4. Performance

#### Query Optimization
- **Index usage**: Verify query plans use indexes
- **Batch operations**: Efficient bulk inserts/updates
- **Connection pooling overhead**: Minimal acquisition time
- **Prepared statements**: Reuse for common queries
- **Result streaming**: Large result set handling

#### Resource Management
- **Memory usage**: No unbounded growth
- **Connection limits**: Respect database limits
- **Query timeouts**: Enforce maximum query time
- **Cursor management**: Proper cursor cleanup
- **Transaction scope**: Minimize transaction duration

### 5. Data Access Patterns

#### Participants Table (O(1) Lookups)
- **Address indexing**: Verify index on address column
- **Compound queries**: Address + block range efficiency
- **Direction filtering**: IN/OUT/BOTH participant types
- **Token transfer data**: JSON field handling
- **Count queries**: Efficient COUNT operations

#### Cross-Table Operations
- **Transaction enrichment**: Join with blocks table
- **Address labeling**: Optional label lookups
- **Historical queries**: Time-based data access
- **Aggregations**: SUM, AVG, COUNT operations
- **Pagination**: Offset/limit for large results

## How We Test It

### Unit Tests with Test Database

```rust
#[tokio::test]
async fn test_connection_pool_exhaustion() {
    // Arrange
    let db_url = test_database_url();
    let manager = DatabaseManager::new(&db_url)
        .with_max_connections(2)
        .await
        .unwrap();
    
    // Act - Acquire all connections
    let conn1 = manager.acquire().await.unwrap();
    let conn2 = manager.acquire().await.unwrap();
    
    // Try to acquire one more (should timeout)
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        manager.acquire()
    ).await;
    
    // Assert
    assert!(result.is_err()); // Timeout
    
    // Cleanup
    drop(conn1);
    drop(conn2);
}
```

### Integration Tests with Real Data

```rust
#[tokio::test]
#[ignore] // Requires test database with data
async fn test_address_transaction_lookup_performance() {
    // Arrange
    let db = setup_test_database().await;
    let fetcher = AddressDataFetcher::new(db.pool());
    let address = Address::from_str(VITALIK_ADDRESS).unwrap();
    
    // Act - Time the query
    let start = Instant::now();
    let transactions = fetcher.get_address_transactions(
        address,
        Some(17_000_000),
        Some(17_100_000),
        Some(100)
    ).await.unwrap();
    let duration = start.elapsed();
    
    // Assert - Should be O(1), very fast
    assert!(duration < Duration::from_millis(10));
    assert!(!transactions.is_empty());
    assert!(transactions.len() <= 100);
}
```

### Error Simulation Tests

```rust
#[tokio::test]
async fn test_database_connection_recovery() {
    // Arrange
    let db = TestDatabase::new().await;
    let manager = DatabaseManager::new(&db.url()).await.unwrap();
    
    // Act - Simulate database restart
    db.stop().await;
    let query_result = manager.health_check().await;
    assert!(query_result.is_err());
    
    db.start().await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Assert - Should recover
    let health = manager.health_check().await;
    assert!(health.is_ok());
}
```

### Performance Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_address_lookups(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let db = runtime.block_on(setup_bench_database());
    
    let mut group = c.benchmark_group("address_lookups");
    
    for block_range in [1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("block_range", block_range),
            block_range,
            |b, &range| {
                b.to_async(&runtime).iter(|| async {
                    let _ = db.get_address_transactions(
                        TEST_ADDRESS,
                        Some(17_000_000),
                        Some(17_000_000 + range),
                        None
                    ).await;
                });
            }
        );
    }
    group.finish();
}
```

## Test Cases

### Connection Management Tests

1. **Normal Operations**
   - Single connection acquire/release
   - Multiple concurrent connections
   - Connection reuse after release
   - Idle connection timeout
   - Connection keep-alive

2. **Error Scenarios**
   - Database unavailable at startup
   - Connection lost during query
   - Connection pool exhausted
   - Invalid credentials
   - Network timeout

3. **Load Testing**
   - 100 concurrent queries
   - Sustained load for 1 hour
   - Burst traffic patterns
   - Connection pool sizing
   - Query queue behavior

### Query Operation Tests

1. **Address Queries**
   - Single address lookup
   - Address with no transactions
   - Address with 1M+ transactions
   - Multiple addresses batch
   - Complex filter combinations

2. **Transaction Queries**
   - Recent transaction (last 100 blocks)
   - Historical transaction (1M+ blocks ago)
   - Non-existent transaction
   - Batch of 1000 transactions
   - Failed transaction handling

3. **Block Queries**
   - Latest block number
   - Block by number
   - Block by timestamp
   - Block range (1000 blocks)
   - Missing block handling

### Data Integrity Tests

1. **Type Safety**
   - Address format validation
   - Hash format validation
   - Numeric precision
   - Timestamp accuracy
   - JSON field parsing

2. **Null Handling**
   - Optional fields
   - Empty results
   - Partial data
   - Default values
   - Missing columns

3. **Large Data**
   - Long input data fields
   - Large token transfer arrays
   - High precision numbers
   - Unicode in data
   - Binary data handling

## Test Data

### Test Database Schema
```sql
-- Ensure test database has production schema
CREATE INDEX IF NOT EXISTS idx_address_transactions_address 
ON address_transactions(address);

CREATE INDEX IF NOT EXISTS idx_address_transactions_block 
ON address_transactions(block_number);

CREATE INDEX IF NOT EXISTS idx_transactions_hash 
ON transactions(hash);
```

### Test Addresses
```rust
pub const TEST_ADDRESSES: &[&str] = &[
    "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", // vitalik.eth
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
    "0x0000000000000000000000000000000000000000", // Zero address
];
```

### Test Transactions
```rust
pub const TEST_TRANSACTIONS: &[&str] = &[
    // Simple ETH transfer
    "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060",
    // Complex DeFi transaction
    "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006",
];
```

## Expected Test Outcomes

### Performance Targets
- Address lookup: < 5ms for any address
- Transaction fetch: < 10ms for single transaction
- Batch operations: < 100ms for 1000 items
- Connection acquisition: < 1ms from pool
- Health check: < 10ms response time

### Reliability Targets
- Connection recovery: < 5 seconds after database restart
- Query retry success: 99% within 3 attempts
- Pool exhaustion recovery: Graceful queuing
- Error messages: Clear and actionable
- No connection leaks over 24 hours

### Data Accuracy
- 100% data integrity for all queries
- Proper null handling without panics
- Accurate numeric conversions
- Consistent transaction ordering
- Correct block number filtering

## Running the Tests

```bash
# Setup test database
./scripts/setup_test_db.sh

# Run all data_access tests
cargo test -p qarqa-data-access

# Run integration tests (requires database)
cargo test -p qarqa-data-access --features integration-tests -- --ignored

# Run benchmarks
cargo bench -p qarqa-data-access

# Run with connection debugging
RUST_LOG=sqlx=debug cargo test -p qarqa-data-access
```

## Test Environment Setup

### PostgreSQL Test Database
```bash
# Create test database
createdb qarqa_test

# Run migrations
sqlx migrate run --database-url postgresql://localhost/qarqa_test

# Load test data
psql qarqa_test < tests/fixtures/test_data.sql
```

### Environment Variables
```bash
# Test database URL
export TEST_DATABASE_URL="postgresql://postgres:postgres@localhost/qarqa_test"

# Connection pool settings for tests
export TEST_MAX_CONNECTIONS=5
export TEST_CONNECTION_TIMEOUT=1000
```

## Test Maintenance

- Monitor query performance weekly
- Update test data after schema changes
- Profile memory usage monthly
- Review connection pool metrics
- Document any flaky tests with fixes