# Process TX Tests

Comprehensive test suite for the `process_tx` module, covering state change extraction, Python service integration, real transaction validation, and performance benchmarks.

## Test Structure

```
process_tx_tests/
├── mod.rs                        # Module exports
├── state_extraction_tests.rs     # Unit tests for state change extraction
├── python_integration_tests.rs   # Python service integration tests
├── real_transaction_tests.rs     # Real mainnet transaction tests
├── performance_tests.rs          # Performance benchmarks
└── README.md                     # This file
```

## Test Categories

### 1. State Extraction Tests (`state_extraction_tests.rs`)

Unit tests for core state change extraction functionality:

- **Token Amount Formatting**: Tests decimal formatting for different token types (USDC, WETH, etc.)
- **ETH Amount Formatting**: Tests ETH amount formatting with proper sign handling
- **Signed Amount Arithmetic**: Tests addition and subtraction of signed amounts
- **Python Format Conversion**: Tests conversion to Python-compatible format
- **Event Count Extraction**: Tests extraction of event counts from logs
- **Error Handling**: Tests error display and serialization
- **Edge Cases**: Tests zero amounts, large numbers, unusual decimals

**Key Test Functions:**
```rust
#[test]
fn test_format_token_amount_usdc()        // USDC formatting (6 decimals)
fn test_format_token_amount_weth()        // WETH formatting (18 decimals)
fn test_signed_amount_arithmetic()        // Arithmetic operations
fn test_convert_to_python_format()        // Python format conversion
fn test_extract_event_counts()            // Event counting
```

**Running State Tests:**
```bash
cargo test state_extraction_tests
```

### 2. Python Integration Tests (`python_integration_tests.rs`)

Tests for Python validation service integration:

- **Client Creation**: Tests HTTP client creation and configuration
- **Request Serialization**: Tests JSON serialization/deserialization
- **Health Checks**: Tests service health endpoint
- **Transaction Validation**: Tests single transaction validation
- **Batch Processing**: Tests batch transaction validation
- **Error Handling**: Tests error scenarios and edge cases

**Key Test Functions:**
```rust
#[tokio::test]
#[ignore] // Only run when Python service available
async fn test_health_check_integration()

#[tokio::test]
#[ignore] // Only run when Python service available
async fn test_single_transaction_validation()

#[tokio::test]
#[ignore] // Only run when Python service available
async fn test_batch_validation()
```

**Running Integration Tests:**
```bash
# Run offline tests (no service required)
cargo test python_integration_tests

# Run integration tests (requires Python service)
cargo test python_integration_tests -- --ignored
```

### 3. Real Transaction Tests (`real_transaction_tests.rs`)

Tests using actual mainnet transactions to validate processing accuracy:

**Test Transactions:**
- **Simple ETH Transfer**: `0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060` (Block 46147)
- **Complex DeFi**: `0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae` (Block 22646153)
- **ERC20 Transfer**: `0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b` (Block 18500000)

**Test Categories:**
- Simple ETH transfers (minimal state changes)
- Complex DeFi transactions (multiple tokens, internal transfers)
- ERC20 token transfers (token movement validation)
- Batch processing validation
- Python comparison validation
- Error handling for invalid transactions

**Running Real Transaction Tests:**
```bash
# Run offline tests
cargo test real_transaction_tests

# Run integration tests (requires Reth node)
cargo test real_transaction_tests -- --ignored
```

### 4. Performance Tests (`performance_tests.rs`)

Benchmarks and performance validation:

- **Single Transaction Performance**: Measures processing time for individual transactions
- **Batch Processing Performance**: Tests efficiency of batch operations
- **Rust vs Python Performance**: Compares processing speeds
- **Concurrent Processing**: Tests parallel transaction processing
- **Memory Stability**: Tests for memory leaks over multiple iterations
- **Processing Time Breakdown**: Detailed performance profiling

**Performance Targets:**
- Single transaction: <100ms (target)
- Batch processing: More efficient than sequential
- Concurrent processing: >1.5x speedup
- Memory usage: Stable over time

**Running Performance Tests:**
```bash
# Run performance benchmarks
cargo test performance_tests -- --ignored
```

## Running All Tests

### Quick Test (Offline Only)
```bash
cargo test process_tx_tests
```

### Full Test Suite (Requires Services)
```bash
# Ensure services are running:
# 1. Reth node at http://127.0.0.1:8545
# 2. Python service at http://127.0.0.1:18000

# Run all tests including ignored ones
cargo test process_tx_tests -- --ignored
```

### Specific Test Categories
```bash
# Unit tests only
cargo test state_extraction_tests

# Integration tests (with services)
cargo test python_integration_tests -- --ignored
cargo test real_transaction_tests -- --ignored

# Performance benchmarks
cargo test performance_tests -- --ignored
```

### Debugging Tests
```bash
# Run with output
cargo test process_tx_tests -- --nocapture

# Run specific test
cargo test test_format_token_amount_usdc -- --exact

# Run with debug logging
RUST_LOG=debug cargo test process_tx_tests -- --nocapture
```

## Test Requirements

### For Offline Tests
- No external dependencies
- Tests basic functionality and serialization
- Always pass in CI/CD environments

### For Integration Tests
- **Reth Node**: Running at `http://127.0.0.1:8545`
  - Must be synced to at least block 22,646,153
  - Archive node recommended for historical transactions
- **Python Service**: Running at `http://127.0.0.1:18000`
  - Started with: `python validation_service.py`
  - Health check: `curl http://127.0.0.1:18000/health`

### For Performance Tests
- Same as integration tests
- Stable system load for accurate benchmarks
- Multiple test runs for statistical significance

## Test Data

### Known Test Transactions
All test transactions are real mainnet transactions with verified outcomes:

1. **Simple Transfer** (Block 46147)
   - Hash: `0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060`
   - Gas: 21,000 (exact)
   - No logs, no internal transfers
   - Expected processing: <10ms

2. **Complex DeFi** (Block 22646153)
   - Hash: `0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae`
   - Gas: 315,099
   - 13 logs, internal transfers
   - Expected processing: 20-50ms

3. **ERC20 Transfer** (Block 18500000)
   - Hash: `0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b`
   - Gas: ~65,000
   - Transfer event logs
   - Expected processing: 10-30ms

### Test Coverage

| Component | Coverage | Status |
|-----------|----------|--------|
| **Core Functions** | | |
| Token amount formatting | ✅ | Complete unit tests |
| ETH amount formatting | ✅ | Complete unit tests |
| Signed amount arithmetic | ✅ | Complete unit tests |
| Python format conversion | ✅ | Complete unit tests |
| Event count extraction | ✅ | Complete unit tests |
| **Integration** | | |
| Python client | ✅ | HTTP client tests |
| Health checks | ✅ | Service availability tests |
| Transaction validation | ✅ | Real transaction tests |
| Batch processing | ✅ | Multiple transaction tests |
| **Performance** | | |
| Single transaction | ✅ | Timing benchmarks |
| Batch processing | ✅ | Efficiency tests |
| Concurrent processing | ✅ | Parallel execution tests |
| Memory stability | ✅ | Leak detection tests |
| **Error Handling** | | |
| Invalid transactions | ✅ | Error propagation tests |
| Service unavailable | ✅ | Network error tests |
| Malformed data | ✅ | Data validation tests |

## Continuous Integration

### CI Test Strategy
```yaml
# Example GitHub Actions
test-process-tx:
  runs-on: ubuntu-latest
  steps:
    - name: Run offline tests
      run: cargo test process_tx_tests
    
    - name: Start services
      run: |
        # Start Reth node (mock or real)
        # Start Python service
        
    - name: Run integration tests
      run: cargo test process_tx_tests -- --ignored
      
    - name: Performance benchmarks
      run: cargo test performance_tests -- --ignored
```

### Local Development
```bash
# Pre-commit hook
cargo test process_tx_tests || exit 1

# Development testing
cargo test process_tx_tests -- --nocapture

# Full validation
./scripts/run_full_test_suite.sh
```

## Test Utilities

### Helper Functions
```rust
// State extraction test utilities
pub fn create_test_signed_amount(value: u64, negative: bool) -> SignedAmount
pub fn create_test_address(value: u8) -> RevmAddress

// Integration test utilities
pub async fn test_python_service_available() -> bool
pub async fn test_reth_node_available() -> bool

// Performance test utilities
pub async fn benchmark_transaction(tx_hash: &str, description: &str) -> TransactionBenchmark
pub struct PerformanceMetrics { /* ... */ }
```

### Mock Data
Tests include comprehensive mock data for:
- Transaction logs with various event types
- State changes with different token configurations
- Error responses from services
- Performance baseline measurements

## Troubleshooting

### Common Issues

1. **"Service not available"**
   ```bash
   # Check services
   curl http://127.0.0.1:8545 -X POST -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
   curl http://127.0.0.1:18000/health
   ```

2. **"Transaction not found"**
   - Ensure node is synced to required block
   - Check transaction hash is correct
   - Verify node has archive data

3. **"Test timeout"**
   - Check network connectivity
   - Reduce test concurrency
   - Increase timeout values

4. **"Performance test failures"**
   - Run on stable system
   - Check for background processes
   - Adjust performance targets

### Debug Commands
```bash
# Check test compilation
cargo check --tests --package revm_tx_simulator

# Run with full output
cargo test process_tx_tests -- --nocapture --test-threads=1

# Run specific failing test
cargo test test_name -- --exact --nocapture

# Profile performance
cargo test performance_tests -- --ignored --nocapture
```

## Future Improvements

1. **Expand Test Coverage:**
   - Add CREATE2 deployment tests
   - Test complex internal transfer patterns
   - Add gas refund edge cases
   - Test hardfork boundary conditions

2. **Enhanced Performance Testing:**
   - Add memory profiling
   - Test with 1000+ transaction batches
   - Add throughput benchmarks
   - Benchmark against other implementations

3. **Property-Based Testing:**
   - Use proptest for fuzzing
   - Generate random valid transactions
   - Test invariants across implementations

4. **Integration Testing:**
   - Test against multiple Ethereum clients
   - Cross-validate with other simulators
   - Test network failure scenarios

5. **Automated Benchmarking:**
   - Continuous performance monitoring
   - Performance regression detection
   - Automated optimization suggestions