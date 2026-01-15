# QARQA Testing Documentation

## Overview

This document provides comprehensive testing guidelines and specifications for the QARQA fund flow analytics system. All tests must be production-ready, use real data patterns, and cover both success and failure scenarios.

## Testing Philosophy

1. **No Mock Data**: All tests use realistic data patterns from actual Ethereum transactions
2. **Comprehensive Coverage**: Test success paths, error cases, edge cases, and performance
3. **Production Scenarios**: Tests simulate real-world usage patterns and failure modes
4. **Clear Documentation**: Every test clearly states what it's testing and why
5. **Maintainable**: Tests are easy to understand, modify, and debug

## Test Structure

```
qarqa/
├── TESTS.md                           # This file
├── tests/                             # Integration tests
│   ├── integration/
│   │   ├── integration.md             # Integration test documentation
│   │   ├── full_pipeline_test.rs
│   │   ├── database_integration_test.rs
│   │   └── rpc_integration_test.rs
│   └── common/
│       ├── mod.rs                     # Shared test utilities
│       └── fixtures.rs                # Test data fixtures
│
├── core_types/
│   ├── src/
│   │   └── [source files with unit tests]
│   └── tests/
│       ├── core_types.md              # Core types test documentation
│       ├── validation_tests.rs
│       ├── resilience_tests.rs
│       └── error_handling_tests.rs
│
├── data_access/
│   ├── src/
│   │   └── [source files with unit tests]
│   └── tests/
│       ├── data_access.md             # Data access test documentation
│       ├── connection_pool_tests.rs
│       ├── query_performance_tests.rs
│       └── error_recovery_tests.rs
│
├── tx_simulation/
│   ├── src/
│   │   └── [source files with unit tests]
│   └── tests/
│       ├── tx_simulation.md           # Transaction simulation test documentation
│       ├── revm_integration_tests.rs
│       ├── state_change_tests.rs
│       └── fund_flow_accuracy_tests.rs
│
├── network_building/
│   ├── src/
│   │   └── [source files with unit tests]
│   └── tests/
│       ├── network_building.md        # Network building test documentation
│       ├── graph_construction_tests.rs
│       ├── visualization_tests.rs
│       └── performance_tests.rs
│
└── api_layer/
    ├── src/
    │   └── [source files with unit tests]
    └── tests/
        ├── api_layer.md               # API layer test documentation
        ├── endpoint_tests.rs
        ├── authentication_tests.rs
        └── rate_limiting_tests.rs
```

## Test Categories

### 1. Unit Tests
- **Location**: In `src/` files within `#[cfg(test)]` modules
- **Scope**: Individual functions and methods
- **Requirements**:
  - Test both success and failure cases
  - Use realistic input values
  - Clear test names: `test_<function>_<scenario>`
  - Document complex test logic

### 2. Integration Tests
- **Location**: Module-level `tests/` directories
- **Scope**: Component interactions within a module
- **Requirements**:
  - Test real component integration
  - Use test database when needed
  - Clean up resources after tests
  - Test error propagation

### 3. System Tests
- **Location**: Root-level `tests/integration/`
- **Scope**: Full pipeline testing
- **Requirements**:
  - Test complete workflows
  - Verify data flow through all components
  - Test with real transaction data
  - Performance validation

### 4. Property-Based Tests
- **Tool**: `proptest` crate
- **Scope**: Invariants and properties
- **Requirements**:
  - Test mathematical properties
  - Generate random valid inputs
  - Verify invariants hold
  - Shrink failing cases

### 5. Performance Tests
- **Tool**: `criterion` crate
- **Scope**: Performance benchmarks
- **Requirements**:
  - Baseline performance metrics
  - Regression detection
  - Memory usage profiling
  - Concurrent operation testing

## Test Data

### Real Transaction Examples
```rust
// Use actual mainnet transactions for testing
pub const TEST_TRANSACTIONS: &[&str] = &[
    "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006", // Complex DeFi
    "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b", // Simple transfer
    "0x2c2e15d46f6e2a9a1f3e6a8f9f1a7c3e4b5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f", // Token swap
];
```

### Test Addresses
```rust
pub const TEST_ADDRESSES: &[&str] = &[
    "0x742d35Cc6634C0532925a3b844Bc9e7595f5b899", // Known contract
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
];
```

## Test Requirements

### Core Types Tests

#### Validation Tests (`core_types/tests/validation_tests.rs`)
- **Address Validation**
  - Valid Ethereum addresses
  - Invalid formats (wrong length, bad hex)
  - SQL injection attempts
  - XSS attempts
  
- **Transaction Hash Validation**
  - Valid 32-byte hashes
  - Invalid formats
  - Edge cases (all zeros, all ones)

- **Numeric Validation**
  - U256 boundary tests
  - Overflow/underflow handling
  - Decimal precision
  - Negative value rejection

#### Resilience Tests (`core_types/tests/resilience_tests.rs`)
- **Retry Logic**
  - Exponential backoff timing
  - Max retry limits
  - Jitter application
  - Concurrent retries
  
- **Circuit Breaker**
  - State transitions (closed → open → half-open)
  - Failure threshold counting
  - Timeout behavior
  - Concurrent access

- **Rate Limiting**
  - Request counting
  - Time window sliding
  - Burst handling
  - Multi-tenant scenarios

### Data Access Tests

#### Connection Pool Tests (`data_access/tests/connection_pool_tests.rs`)
- Pool exhaustion handling
- Connection timeout behavior
- Concurrent access patterns
- Connection recycling
- Error recovery

#### Query Performance Tests (`data_access/tests/query_performance_tests.rs`)
- Large result set handling
- Query timeout enforcement
- Index usage verification
- Batch query optimization
- Memory usage under load

### Transaction Simulation Tests

#### REVM Integration Tests (`tx_simulation/tests/revm_integration_tests.rs`)
- Simple ETH transfers
- Complex DeFi transactions
- Failed transactions
- Internal transfers extraction
- Gas calculation accuracy

#### State Change Tests (`tx_simulation/tests/state_change_tests.rs`)
- Balance change calculations
- Token transfer tracking
- Multi-hop fund flows
- Circular transfer detection
- Zero-value transfer handling

### Network Building Tests

#### Graph Construction Tests (`network_building/tests/graph_construction_tests.rs`)
- Node creation and deduplication
- Edge weight aggregation
- Cycle detection
- Subgraph extraction
- Large network handling (1000+ nodes)

#### Visualization Tests (`network_building/tests/visualization_tests.rs`)
- JSON serialization correctness
- Layout algorithm convergence
- Data size limits
- Format conversions
- Performance with complex networks

### API Layer Tests

#### Endpoint Tests (`api_layer/tests/endpoint_tests.rs`)
- All HTTP methods
- Request validation
- Response format verification
- Error response codes
- CORS handling

#### Authentication Tests (`api_layer/tests/authentication_tests.rs`)
- JWT token validation
- Token expiration
- Permission checking
- Rate limit per user
- Security headers

## Test Execution

### Running All Tests
```bash
# Run all tests with output
cargo test --all --verbose

# Run with specific log level
RUST_LOG=debug cargo test --all

# Run tests in parallel
cargo test --all --release -- --test-threads=8
```

### Running Specific Test Categories
```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# Specific module tests
cargo test -p qarqa-core-types

# Performance benchmarks
cargo bench
```

### Test Coverage
```bash
# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# With branch coverage
cargo tarpaulin --branch --out Xml
```

## Continuous Integration

### Required Checks
1. All tests pass
2. No compilation warnings
3. Code coverage > 80%
4. No security vulnerabilities (`cargo audit`)
5. Performance benchmarks within tolerance

### Test Environment
- PostgreSQL 14+ with test database
- Local Ethereum node (for integration tests)
- Redis (for caching tests)
- Minimum 8GB RAM for full test suite

## Test Writing Guidelines

### 1. Test Naming
```rust
#[test]
fn test_validate_address_with_valid_checksum() { }

#[test]
fn test_retry_logic_exhausts_attempts_on_persistent_failure() { }
```

### 2. Test Structure
```rust
#[test]
fn test_example() {
    // Arrange
    let input = prepare_test_data();
    
    // Act
    let result = function_under_test(input);
    
    // Assert
    assert_eq!(result, expected_value);
    
    // Cleanup (if needed)
    cleanup_resources();
}
```

### 3. Error Testing
```rust
#[test]
fn test_error_handling() {
    let result = risky_operation();
    
    assert!(result.is_err());
    match result.unwrap_err() {
        QarqaError::InvalidInput(msg) => {
            assert!(msg.contains("expected text"));
        }
        _ => panic!("Wrong error type"),
    }
}
```

### 4. Async Testing
```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### 5. Property Testing
```rust
proptest! {
    #[test]
    fn test_address_parsing_roundtrip(s in "[0-9a-fA-F]{40}") {
        let addr_str = format!("0x{}", s);
        let parsed = parse_address(&addr_str).unwrap();
        let formatted = format!("{:?}", parsed);
        assert_eq!(formatted.to_lowercase(), addr_str.to_lowercase());
    }
}
```

## Test Maintenance

### Regular Tasks
1. **Weekly**: Run full test suite with coverage
2. **Monthly**: Review and update test data
3. **Quarterly**: Performance baseline updates
4. **On Change**: Update tests with code changes

### Test Data Management
- Use version-controlled test fixtures
- Document test data sources
- Regularly validate test data relevance
- Anonymize sensitive data

## Troubleshooting

### Common Issues
1. **Database Connection Failures**
   - Check DATABASE_URL environment variable
   - Ensure test database exists
   - Verify connection pool settings

2. **Flaky Tests**
   - Add proper waits for async operations
   - Use deterministic test data
   - Isolate tests from external dependencies

3. **Performance Test Variations**
   - Run on consistent hardware
   - Minimize background processes
   - Use release builds for benchmarks

## Future Improvements

1. **Mutation Testing**: Ensure test quality with `cargo-mutants`
2. **Fuzz Testing**: Add fuzzing for input validation
3. **Load Testing**: Simulate production load patterns
4. **Chaos Testing**: Test system resilience
5. **Contract Testing**: Verify API contracts