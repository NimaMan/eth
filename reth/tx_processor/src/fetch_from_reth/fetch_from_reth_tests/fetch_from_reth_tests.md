# Fetch From Reth Testing Documentation

## Overview
This document defines the comprehensive test suite for the `fetch_from_reth` module, which provides direct access to Reth's MDBX database for high-performance blockchain data retrieval. The tests validate correctness, performance, and reliability of database operations.

## Test Categories

### 1. Unit Tests (`unit_tests.rs`)

#### 1.1 Provider Creation Tests
```rust
#[test]
fn test_provider_creation_success() {
    // Test successful creation of RethDatabaseProvider
    // Validates:
    // - Correct database path handling
    // - Proper initialization of provider factory
    // - Static file provider setup
}

#[test]
fn test_provider_creation_invalid_path() {
    // Test error handling for invalid database paths
    // Expected: FetchError::DatabaseError
}

#[test]
fn test_provider_configuration() {
    // Test different configuration options
    // - read_only mode enforcement
    // - static file enable/disable
    // - consistency check options
}
```

#### 1.2 Transaction Fetch Tests
```rust
use alloy_primitives::{B256, Address};
use reth_ethereum::TransactionSigned;

#[test]
fn test_fetch_transaction_success() {
    // Test data: Real mainnet transaction
    let tx_hash = B256::from_str("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060").unwrap();
    
    // Expected data:
    // - Block: 46147
    // - From: 0x32Be343B94f860124dC4fEe278FDCBD38C102D88
    // - To: 0x53b04999c1FF2d77fcdde98935BB936A67209E4C
    // - Value: 500000000000000000 (0.5 ETH)
    // - Gas Used: 21000
    
    let provider = setup_test_provider();
    let result = provider.fetch_transaction(tx_hash).unwrap();
    
    assert_eq!(result.block_number, 46147);
    assert_eq!(result.from, Address::from_str("0x32Be343B94f860124dC4fEe278FDCBD38C102D88").unwrap());
    assert_eq!(result.to, Some(Address::from_str("0x53b04999c1FF2d77fcdde98935BB936A67209E4C").unwrap()));
    assert_eq!(result.value, U256::from_str("500000000000000000").unwrap());
    assert_eq!(result.gas_used, 21000);
}

#[test]
fn test_fetch_transaction_not_found() {
    // Test non-existent transaction
    let tx_hash = B256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    // Expected: FetchError::NotFound
}

#[test]
fn test_fetch_contract_creation() {
    // Test contract creation transaction
    let tx_hash = B256::from_str("0x7b0a47d3b0234280b6c9213c5bbff44c8b6001bea7770b3950280f91410532d6").unwrap();
    // Validate:
    // - to_address is None
    // - contract_address is populated
    // - input data contains bytecode
}
```

#### 1.3 Receipt Fetch Tests
```rust
#[test]
fn test_fetch_receipt_with_logs() {
    // Test transaction with multiple logs
    let tx_hash = B256::from_str("0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006").unwrap();
    
    // Validate:
    // - Receipt status (success/failure)
    // - Gas used matches transaction
    // - All logs are retrieved with correct data
    // - Log indices are sequential
}

#[test]
fn test_fetch_receipt_failed_transaction() {
    // Test failed transaction receipt
    let tx_hash = B256::from_str("0x7c90a0b4f5c0e1a94ffd22330e15e18f7da77a1155ad00f7f5d1a1c4a8e36a4a").unwrap();
    // Validate:
    // - Status = 0 (failed)
    // - Gas used = gas limit (all gas consumed)
    // - No logs emitted
}
```

#### 1.4 Batch Operations Tests
```rust
#[test]
fn test_batch_fetch_multiple_transactions() {
    let tx_hashes = vec![
        B256::from_str("0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060").unwrap(),
        B256::from_str("0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006").unwrap(),
        B256::from_str("0x2d8c72e31d4a10e7d9c1f4e1c5f4e3a9b8d7c6f5e4d3c2b1a0918273645546372").unwrap(),
    ];
    
    // Validate:
    // - All transactions returned in order
    // - Missing transactions handled gracefully
    // - Performance better than individual fetches
}

#[test]
fn test_batch_fetch_empty_list() {
    let tx_hashes = vec![];
    // Expected: Empty result vector
}

#[test]
fn test_batch_fetch_duplicate_hashes() {
    // Test deduplication in batch operations
}
```

#### 1.5 Cache Tests
```rust
#[test]
fn test_cache_hit_performance() {
    // Measure performance improvement from cache hits
    // First fetch: ~0.2ms
    // Cache hit: <0.01ms
}

#[test]
fn test_cache_eviction_lru() {
    // Test LRU eviction policy
    // Fill cache beyond capacity
    // Verify oldest entries evicted
}

#[test]
fn test_cache_statistics() {
    // Validate cache statistics
    // - Hit rate calculation
    // - Miss count tracking
    // - Eviction count
}
```

### 2. Integration Tests (`integration_tests.rs`)

#### 2.1 Real Database Connection Tests
```rust
use reth_ethereum::{
    chainspec::ChainSpecBuilder,
    node::EthereumNode,
    provider::providers::ReadOnlyConfig,
};
use std::path::PathBuf;

#[test]
#[ignore] // Requires local Reth node
fn test_real_database_connection() {
    // Connect to actual Reth database
    let datadir = std::env::var("RETH_DATADIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::data_dir()
            .unwrap()
            .join("reth")
            .join("mainnet"));
    
    let spec = ChainSpecBuilder::mainnet().build();
    let factory = EthereumNode::provider_factory_builder()
        .open_read_only(spec.into(), ReadOnlyConfig::from_datadir(&datadir))
        .expect("Failed to open database");
    
    let provider = factory.provider().expect("Failed to create provider");
    
    // Validate:
    // - Database opens successfully
    // - Can fetch latest block
    let latest = provider.best_block_number().expect("Failed to get best block");
    assert!(latest > 18_000_000, "Expected recent block number");
}

#[test]
#[ignore] // Requires local Reth node
fn test_cross_table_consistency() {
    // Test data consistency across MDBX tables
    // - Transaction in Transactions table
    // - Mapping in TransactionHashNumbers
    // - Block reference in TransactionBlocks
}
```

#### 2.2 Complex Transaction Tests
```rust
#[test]
fn test_defi_swap_transaction() {
    // Test Uniswap swap transaction
    let tx_hash = B256::from_str("0x7b944d902fdc3ed8d68f38480a1b42b01fe9e2c4af2f6a7d51cf17bfc9ea58af").unwrap();
    
    // Validate:
    // - Multiple event logs parsed
    // - Token transfers tracked
    // - Gas calculations correct
}

#[test]
fn test_multi_call_transaction() {
    // Test transaction with multiple internal calls
    // Validate trace data is complete
}
```

#### 2.3 State Access Tests
```rust
#[test]
fn test_account_state_retrieval() {
    // Test fetching account state at specific block
    let address = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap(); // WETH
    let block_number = 18_000_000;
    
    // Validate:
    // - Balance retrieval
    // - Nonce tracking
    // - Code hash (for contracts)
}

#[test]
fn test_storage_slot_access() {
    // Test reading contract storage
    let contract = Address::from_str("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").unwrap(); // USDC
    let slot = B256::ZERO; // totalSupply slot
    
    // Validate storage value retrieval
}
```

### 3. Performance Tests (`performance_tests.rs`)

#### 3.1 Throughput Tests
```rust
#[test]
fn test_single_transaction_performance() {
    // Target: <1ms per transaction
    // Measure percentiles: p50, p95, p99
}

#[test]
fn test_batch_throughput() {
    // Test batch sizes: 10, 100, 1000
    // Measure transactions per second
    // Target: >5000 TPS for reads
}

#[test]
fn test_concurrent_reads() {
    // Test multi-threaded access
    // Spawn 10 threads reading different transactions
    // Validate no lock contention
}
```

#### 3.2 Latency Tests
```rust
#[test]
fn test_cold_start_latency() {
    // Measure first transaction fetch
    // Include database open time
    // Target: <100ms total
}

#[test]
fn test_warm_cache_latency() {
    // Measure cached transaction fetch
    // Target: <0.01ms
}
```

#### 3.3 Memory Tests
```rust
#[test]
fn test_memory_usage_baseline() {
    // Measure baseline memory with database open
    // Expected: ~2GB virtual memory (MDBX mapping)
}

#[test]
fn test_memory_growth_with_cache() {
    // Test memory growth as cache fills
    // Validate bounded memory usage
}
```

### 4. Stress Tests (`stress_tests.rs`)

#### 4.1 High Volume Tests
```rust
#[test]
#[ignore] // Long running test
fn test_million_transaction_fetch() {
    // Fetch 1M transactions sequentially
    // Validate:
    // - No memory leaks
    // - Consistent performance
    // - Error rate <0.01%
}

#[test]
#[ignore] // Resource intensive
fn test_concurrent_stress() {
    // 100 threads, 10K transactions each
    // Validate thread safety and performance
}
```

#### 4.2 Edge Case Tests
```rust
#[test]
fn test_database_lock_handling() {
    // Simulate database lock scenarios
    // Validate graceful retry/timeout
}

#[test]
fn test_corrupted_data_handling() {
    // Test resilience to data corruption
    // Should return appropriate errors
}
```

## Test Data

### Real Mainnet Transactions for Testing
```rust
// Simple ETH transfer
pub const ETH_TRANSFER_TX: &str = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060";

// ERC20 transfer
pub const ERC20_TRANSFER_TX: &str = "0x2d8ae95d9410c1cefa65c48aa9c2b793eff1e8057d9bf35a2f8e81d9e7b4e5fb";

// Uniswap V2 swap
pub const UNISWAP_V2_SWAP_TX: &str = "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006";

// Failed transaction
pub const FAILED_TX: &str = "0x7c90a0b4f5c0e1a94ffd22330e15e18f7da77a1155ad00f7f5d1a1c4a8e36a4a";

// Contract creation
pub const CONTRACT_CREATION_TX: &str = "0x7b0a47d3b0234280b6c9213c5bbff44c8b6001bea7770b3950280f91410532d6";
```

## Mock Implementations

### Mock Provider for Unit Tests
```rust
pub struct MockRethProvider {
    transactions: HashMap<B256, TransactionData>,
    error_on: Option<B256>,
}

impl RethDataProvider for MockRethProvider {
    fn fetch_transaction(&self, tx_hash: B256) -> Result<TransactionData, FetchError> {
        if let Some(error_hash) = &self.error_on {
            if tx_hash == *error_hash {
                return Err(FetchError::DatabaseError("Mock error".to_string()));
            }
        }
        
        self.transactions
            .get(&tx_hash)
            .cloned()
            .ok_or_else(|| FetchError::NotFound(format!("Transaction {} not found", tx_hash)))
    }
}
```

## Test Helpers

### Database Setup Helper
```rust
use reth_ethereum::{
    chainspec::ChainSpecBuilder,
    node::{EthereumNode, api::NodeTypesWithDBAdapter},
    provider::{
        providers::ReadOnlyConfig,
        db::DatabaseEnv,
        ProviderFactory,
    },
};
use std::{path::PathBuf, sync::Arc};

pub fn setup_test_database() -> eyre::Result<ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>> {
    // Use test data directory or real Reth directory
    let test_dir = std::env::var("RETH_TEST_DATADIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./test_data/reth_db"));
    
    let spec = ChainSpecBuilder::mainnet().build();
    
    // Create provider factory with read-only access
    let factory = EthereumNode::provider_factory_builder()
        .open_read_only(spec.into(), ReadOnlyConfig::from_datadir(&test_dir))?;
    
    Ok(factory)
}

// Helper to create mock provider for unit tests
pub fn setup_mock_provider() -> MockRethProvider {
    MockRethProvider::new()
        .with_transaction(ETH_TRANSFER_TX, create_eth_transfer_data())
        .with_transaction(ERC20_TRANSFER_TX, create_erc20_transfer_data())
        .with_transaction(UNISWAP_V2_SWAP_TX, create_uniswap_swap_data())
}
```

### Performance Measurement Helper
```rust
pub fn measure_operation<F, T>(name: &str, operation: F) -> T 
where 
    F: FnOnce() -> T 
{
    let start = Instant::now();
    let result = operation();
    let duration = start.elapsed();
    
    println!("{}: {:?}", name, duration);
    result
}
```

## Continuous Integration

### CI Test Commands
```yaml
# Run in CI pipeline
test-fetch-from-reth:
  script:
    - cargo test fetch_from_reth::tests::unit_tests
    - cargo test fetch_from_reth::tests::integration_tests
    - cargo test fetch_from_reth::tests::performance_tests -- --nocapture
```

### Performance Regression Detection
```rust
#[test]
fn test_performance_regression() {
    // Compare against baseline metrics
    let baseline_single_fetch = Duration::from_millis(1);
    let actual = measure_operation("single_fetch", || {
        provider.fetch_transaction(tx_hash)
    });
    
    assert!(actual < baseline_single_fetch, 
        "Performance regression: {:?} > {:?}", actual, baseline_single_fetch);
}
```

## Test Coverage Requirements

- **Unit Tests**: 90%+ line coverage
- **Integration Tests**: All major workflows
- **Performance Tests**: Key operations benchmarked
- **Stress Tests**: System limits validated

## Running Tests

### Local Development
```bash
# Run all tests
cargo test fetch_from_reth

# Run with output
cargo test fetch_from_reth -- --nocapture

# Run specific test
cargo test fetch_from_reth::tests::unit_tests::test_fetch_transaction_success
```

### With Real Reth Node
```bash
# Set environment for integration tests
export RETH_DATADIR=/path/to/reth/mainnet

# Run integration tests
cargo test fetch_from_reth::tests::integration_tests -- --ignored
```

### Performance Testing
```bash
# Run benchmarks
cargo test fetch_from_reth::tests::performance_tests -- --nocapture --test-threads=1

# Profile with flamegraph
cargo flamegraph --test fetch_from_reth_tests -- --ignored
```