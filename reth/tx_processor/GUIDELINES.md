# TX Processor Development Guidelines

## Project Structure

```
tx_processor/
├── TX_PROCESSOR.md          # Module overview and functionality
├── GUIDELINES.md            # This file - development guidelines
├── Cargo.toml              # Module dependencies
├── src/
│   ├── lib.rs              # Module exports and documentation
│   ├── types.rs            # Core data structures shared across modules
│   ├── conversions.rs      # Type conversion utilities
│   ├── state_diff_utils.rs # State difference processing utilities
│   │
│   ├── fetch_from_reth/    # Get data from Reth DB (RPC-free)
│   │   ├── mod.rs
│   │   ├── provider.rs     # Reth database provider implementation
│   │   └── cache.rs        # LRU cache for performance
│   │
│   ├── simulate_signed_tx/ # Simulate transactions using REVM
│   │   ├── mod.rs
│   │   ├── simulation_core.rs    # Core REVM simulation logic
│   │   ├── call_tracer.rs        # Trace internal ETH transfers
│   │   └── internal_transfer_tracker.rs # Extract internal transfers
│   │
│   ├── process_tx/         # Process simulated transactions
│   │   ├── mod.rs
│   │   ├── processor.rs    # Main processor orchestrating the pipeline
│   │   └── state_diff.rs   # Extract and process state changes
│   │
│   ├── decode_events/      # Decode transaction events/logs
│   │   ├── mod.rs
│   │   ├── decoder.rs      # Main event decoder
│   │   ├── erc20.rs        # ERC20 event decoding
│   │   └── uniswap.rs      # Uniswap V2/V3 event decoding
│   │
│   ├── classify_tx/        # Classify transaction types
│   │   ├── mod.rs
│   │   └── classifier.rs   # Transaction classification logic
│   │
│   ├── bin/                # CLI binaries (installed with cargo install)
│   │   └── revm_cli.rs     # Main CLI tool for transaction processing
│   │
│   └── [Legacy folders to be removed]
│       ├── signed_tx_simulator/ # Old location (moved to simulate_signed_tx)
│       ├── tx_processor/       # Old processor (moved to process_tx)
│       ├── database/           # Old database (moved to fetch_from_reth)
│       ├── analysis/           # Old analysis (split into decode_events and classify_tx)
│       └── processor.rs        # Old processor file
├── examples/
│   ├── process_single.rs   # Process a single transaction
│   ├── batch_process.rs    # Batch processing example
│   ├── benchmark.rs        # Performance benchmarking
│   └── process_single_tx.rs # Simple single transaction processing (from bin/)
└── tests/
    ├── integration_test.rs # Integration tests
    └── accuracy_tests.rs   # Accuracy validation tests
```

## Code Organization

### Module Structure

Each submodule should follow this pattern:

```rust
// mod.rs
//! Module documentation explaining purpose and usage

mod implementation;
mod types;
mod tests;

pub use implementation::*;
pub use types::*;
```

### File Naming
- `mod.rs` - Module definition and exports
- `types.rs` - Data structures specific to the module
- `*_impl.rs` - Implementation files (e.g., `provider_impl.rs`)
- `tests.rs` or `*_tests.rs` - Test files

## Coding Standards

### 1. **No RPC Calls**
```rust
// ❌ WRONG
let tx = provider.get_transaction(hash).await?;

// ✅ CORRECT
let tx = db_provider.get_transaction_from_db(hash)?;
```

### 2. **Error Handling**
All errors must be strongly typed and propagated:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Transaction not found: {0}")]
    NotFound(B256),
    
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
}

// Use ? operator for propagation
pub fn process(hash: B256) -> Result<ProcessedTx, ProcessorError> {
    let tx = get_tx(hash)?;
    simulate(tx)?
}
```

### 3. **Performance Tracking**
Always measure performance-critical operations:

```rust
use std::time::Instant;

pub fn process_transaction(hash: B256) -> Result<ProcessedTransaction> {
    let start = Instant::now();
    
    // Database read
    let db_start = Instant::now();
    let tx_data = db_provider.get_transaction(hash)?;
    let db_time = db_start.elapsed();
    
    // Simulation
    let sim_start = Instant::now();
    let sim_result = simulator.simulate(&tx_data)?;
    let sim_time = sim_start.elapsed();
    
    Ok(ProcessedTransaction {
        // ... other fields
        db_read_time_ms: db_time.as_secs_f64() * 1000.0,
        simulation_time_ms: sim_time.as_secs_f64() * 1000.0,
        total_time_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}
```

### 4. **Documentation**
Every public item must be documented:

```rust
/// Processes a single Ethereum transaction extracting all state changes.
/// 
/// # Arguments
/// * `tx_hash` - The transaction hash to process
/// 
/// # Returns
/// * `ProcessedTransaction` with complete state changes and internal transfers
/// 
/// # Errors
/// * `ProcessorError::NotFound` if transaction doesn't exist
/// * `ProcessorError::SimulationFailed` if REVM simulation fails
pub fn process_transaction(tx_hash: B256) -> Result<ProcessedTransaction> {
    // implementation
}
```

## Testing Guidelines

### 1. **Unit Tests**
Every function should have unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_simple_transfer() {
        let processor = TransactionProcessor::new_test();
        let result = processor.process_transaction(KNOWN_TX_HASH).unwrap();
        
        assert_eq!(result.gas_used, 21_000);
        assert_eq!(result.internal_transfers.len(), 0);
    }
}
```

### 2. **Integration Tests**
End-to-end tests with real data:

```rust
#[test]
fn test_complex_defi_transaction() {
    let processor = create_test_processor();
    let complex_tx = "0x7b944d902fb39cf8ff872710dd92268e3564be80a99de005cf9ea50bc98a0e7a";
    
    let result = processor.process_transaction(complex_tx.parse().unwrap()).unwrap();
    
    // Verify internal transfers
    assert!(result.internal_transfers.len() > 0);
    
    // Verify state changes
    assert!(result.state_changes.len() > 2);
}
```

### 3. **Performance Tests**
Benchmark critical paths:

```rust
#[bench]
fn bench_process_transaction(b: &mut Bencher) {
    let processor = create_processor();
    let tx_hash = BENCHMARK_TX;
    
    b.iter(|| {
        processor.process_transaction(tx_hash).unwrap();
    });
}
```

## Production Considerations

### 1. **No Mock Data**
- All examples must use real transaction hashes
- Test data should come from mainnet
- No hardcoded test values in production code

### 2. **Resource Management**
```rust
pub struct ProcessorConfig {
    /// Maximum memory for cache (in MB)
    pub max_cache_memory: usize,
    
    /// Maximum concurrent simulations
    pub max_concurrent_sims: usize,
    
    /// Timeout for single transaction processing
    pub processing_timeout: Duration,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            max_cache_memory: 1024,      // 1GB
            max_concurrent_sims: 100,     
            processing_timeout: Duration::from_secs(5),
        }
    }
}
```

### 3. **Monitoring & Metrics**
```rust
pub struct ProcessorMetrics {
    pub total_processed: AtomicU64,
    pub total_errors: AtomicU64,
    pub avg_processing_time_ms: AtomicF64,
    pub cache_hit_rate: AtomicF64,
}
```

## Example Patterns

### 1. **Batch Processing Pattern**
```rust
pub fn process_batch(tx_hashes: Vec<B256>) -> Vec<Result<ProcessedTransaction>> {
    // Group by block for efficiency
    let grouped = group_by_block(tx_hashes);
    
    // Process each block group
    grouped.into_par_iter()
        .flat_map(|(block, hashes)| {
            let block_state = load_block_state(block);
            hashes.into_iter()
                .map(|hash| process_with_state(hash, &block_state))
                .collect::<Vec<_>>()
        })
        .collect()
}
```

### 2. **Caching Pattern**
```rust
pub struct CachedProcessor {
    inner: TransactionProcessor,
    cache: LruCache<B256, ProcessedTransaction>,
}

impl CachedProcessor {
    pub fn process(&mut self, hash: B256) -> Result<ProcessedTransaction> {
        if let Some(cached) = self.cache.get(&hash) {
            return Ok(cached.clone());
        }
        
        let result = self.inner.process_transaction(hash)?;
        self.cache.put(hash, result.clone());
        Ok(result)
    }
}
```

### 3. **Streaming Pattern**
```rust
pub fn process_stream(
    tx_stream: impl Stream<Item = B256>,
) -> impl Stream<Item = Result<ProcessedTransaction>> {
    tx_stream
        .chunks(100)  // Process in batches
        .flat_map(|batch| {
            stream::iter(process_batch(batch))
        })
}
```

## Security Considerations

1. **Input Validation**: Always validate transaction hashes and data
2. **Resource Limits**: Enforce timeouts and memory limits
3. **Error Information**: Don't leak sensitive information in errors
4. **Dependency Audit**: Regularly audit dependencies for vulnerabilities

## Performance Targets

| Operation | Target | Maximum |
|-----------|--------|---------|
| DB Read | 0.3ms | 1ms |
| Simulation | 2ms | 5ms |
| Total Processing | 3ms | 10ms |
| Batch (100 tx) | 200ms | 500ms |

## Deprecation Policy

When deprecating functionality:
1. Mark with `#[deprecated]` attribute
2. Provide migration guide in documentation
3. Maintain for at least 2 major versions
4. Remove only in major version bumps