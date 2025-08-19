# TxProcessor Examples

Examples demonstrating the high-performance transaction processing capabilities of ethtx.TxProcessor.

## Examples

### 01_performance_benchmark.py
Comprehensive performance comparison between Rust (ethtx) and Python implementations.
- Tests with 1000+ real transactions
- Measures individual and batch processing speeds
- Shows 91.5x performance improvement

### 02_compatibility_validation.py
Validates that ethtx produces identical results to Python implementation.
- Compares all fields of ProcessedTransaction
- Ensures drop-in compatibility
- Tests various transaction types

### 03_state_changes_comparison.py
Compares state change extraction between implementations.
- Validates internal transactions
- Checks ERC20 transfer detection
- Ensures state changes match

### 04_batch_processing_optimization.py
Demonstrates optimized batch processing techniques.
- Parallel processing with multiple workers
- Memory-efficient caching strategies
- Error handling and retry logic

## Usage

```python
import ethtx

# Initialize processor (uses hardcoded Reth path)
processor = ethtx.TxProcessor()

# Process single transaction
tx = processor.process_transaction("0xabc...")

# Batch processing (much faster!)
txs = processor.process_transactions_batch(["0xabc...", "0xdef..."])

# Detailed batch with error tracking
result = processor.process_transactions_detailed(
    tx_hashes, 
    parallel=True, 
    max_workers=4
)
```

## Performance

- **Single transaction**: ~2-3ms
- **Batch processing**: ~1825 tx/sec (4 workers)
- **Complex DeFi txs**: ~4-5ms each