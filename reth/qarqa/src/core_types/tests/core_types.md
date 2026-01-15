# Core Types Testing Documentation

## Overview

The `core_types` module provides fundamental types, error handling, validation, and resilience patterns for the QARQA system. This document specifies what we test and how we test it.

## What We Are Testing

### 1. Type Safety and Conversions

#### Address Handling
- **Valid address parsing**: Correctly parse 20-byte Ethereum addresses with 0x prefix
- **Invalid address rejection**: Reject malformed addresses (wrong length, invalid hex, missing prefix)
- **Address formatting**: Consistent display format for logging and UI
- **SQL injection prevention**: Reject addresses containing SQL metacharacters
- **TryFrom implementation**: Safe conversion from strings without panics

#### Transaction Types
- **Hash validation**: 32-byte transaction hashes with proper formatting
- **Block number bounds**: Ensure block numbers are within valid ranges
- **Value overflow protection**: U256 values don't overflow on conversion
- **Timestamp validation**: Reject future timestamps and invalid dates

### 2. Error Handling

#### Error Propagation
- **Error chain preservation**: Original error context is maintained
- **Error type conversions**: All external errors map to QarqaError variants
- **Clone implementation**: Errors can be cloned for multi-path handling
- **Display formatting**: User-friendly error messages

#### Error Recovery
- **Retry mechanisms**: Transient errors trigger appropriate retries
- **Fallback strategies**: Graceful degradation when primary path fails
- **Circuit breaker**: Prevent cascading failures
- **Timeout handling**: Operations fail cleanly on timeout

### 3. Input Validation

#### Security Validation
- **SQL injection**: Detect and reject SQL injection attempts
- **XSS prevention**: Sanitize inputs that could contain scripts
- **Path traversal**: Block directory traversal attempts
- **Command injection**: Prevent shell command injection

#### Business Logic Validation
- **Value ranges**: ETH values within reasonable bounds (0 to max supply)
- **Address checksums**: Validate EIP-55 checksums when present
- **Label sanitization**: Only allow safe characters in labels
- **Batch size limits**: Prevent resource exhaustion

### 4. Resilience Patterns

#### Retry Logic
- **Exponential backoff**: Delays increase appropriately
- **Jitter application**: Prevent thundering herd
- **Maximum attempts**: Respect retry limits
- **Error classification**: Only retry transient errors

#### Circuit Breaker
- **State transitions**: Closed → Open → Half-Open → Closed
- **Failure counting**: Accurate failure threshold tracking
- **Timeout behavior**: Automatic half-open attempts
- **Thread safety**: Safe concurrent access

#### Rate Limiting
- **Request counting**: Accurate request tracking
- **Time windows**: Sliding window implementation
- **Burst handling**: Allow reasonable bursts
- **Per-client limits**: Track limits per address/IP

### 5. Utility Functions

#### Numeric Utilities
- **Wei/ETH conversion**: Accurate 18 decimal conversions
- **Percentage calculations**: Handle edge cases (0 values)
- **Statistical functions**: Correct mean, std dev, correlation
- **Rounding precision**: Maintain decimal accuracy

#### Data Processing
- **Moving averages**: Correct window calculations
- **Normalization**: Min-max scaling with bounds
- **Correlation analysis**: Pearson correlation coefficient
- **Format utilities**: Address shortening for display

## How We Test It

### Unit Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, U256};
    use std::str::FromStr;

    #[test]
    fn test_address_validation_accepts_valid_checksum() {
        // Arrange
        let valid_address = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
        
        // Act
        let result = EthAddress::from_str(valid_address);
        
        // Assert
        assert!(result.is_ok());
        let address = result.unwrap();
        assert_eq!(format!("{:?}", address.address), valid_address.to_lowercase());
    }

    #[test]
    fn test_address_validation_rejects_sql_injection() {
        // Arrange
        let malicious = "0x1234'; DROP TABLE users; --";
        
        // Act
        let result = EthAddress::from_str(malicious);
        
        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QarqaError::InvalidInput(_)));
    }
}
```

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_wei_eth_conversion_roundtrip(wei_value in 0u128..=u128::MAX) {
        let wei = U256::from(wei_value);
        let eth = wei_to_eth(wei);
        let back_to_wei = eth_to_wei(eth);
        
        // Allow for floating point precision loss
        let diff = if wei > back_to_wei {
            wei - back_to_wei
        } else {
            back_to_wei - wei
        };
        
        assert!(diff < U256::from(1000u64)); // Less than 1000 wei difference
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_resilience_with_real_scenarios() {
    // Test retry with actual network calls
    let config = RetryConfig::default();
    let result = retry_with_backoff(&config, || async {
        // Simulate intermittent network failure
        make_network_request().await
    }).await;
    
    assert!(result.is_ok());
}
```

### Performance Tests

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_address_validation(c: &mut Criterion) {
    c.bench_function("validate_ethereum_address", |b| {
        b.iter(|| {
            let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f5b899";
            let _ = EthAddress::from_str(black_box(addr));
        })
    });
}
```

## Test Cases

### Validation Test Cases

1. **Valid Inputs**
   - Standard Ethereum addresses (40 hex chars with 0x)
   - Mixed case addresses (EIP-55 checksum)
   - Contract addresses
   - Zero address (0x0000...)

2. **Invalid Inputs**
   - Empty strings
   - Missing 0x prefix
   - Wrong length (not 42 chars)
   - Invalid hex characters
   - SQL injection attempts
   - XSS payloads

3. **Edge Cases**
   - Maximum U256 values
   - Minimum positive values
   - Unicode in labels
   - Very long input strings

### Resilience Test Cases

1. **Retry Scenarios**
   - First attempt succeeds
   - Succeeds after N retries
   - All attempts fail
   - Timeout during retry
   - Panic recovery

2. **Circuit Breaker Scenarios**
   - Normal operation (closed)
   - Failure threshold reached (open)
   - Recovery timeout (half-open)
   - Successful recovery (closed)
   - Oscillating failures

3. **Rate Limiter Scenarios**
   - Under limit requests
   - Exactly at limit
   - Burst of requests
   - Distributed over time
   - Multi-client scenarios

### Error Handling Test Cases

1. **Error Propagation**
   - Database errors
   - Network timeouts
   - Parsing failures
   - Validation errors
   - System errors

2. **Error Recovery**
   - Transient vs permanent
   - Fallback activation
   - Graceful degradation
   - Error logging
   - Metric collection

## Test Data

### Valid Test Addresses
```rust
pub const VALID_ADDRESSES: &[&str] = &[
    "0x0000000000000000000000000000000000000000", // Zero address
    "0x742d35Cc6634C0532925a3b844Bc9e7595f5b899", // Random valid
    "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed", // Checksum address
    "0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe", // EthDev donation
];
```

### Invalid Test Data
```rust
pub const INVALID_INPUTS: &[(&str, &str)] = &[
    ("", "Empty input"),
    ("0x", "Only prefix"),
    ("742d35Cc6634C0532925a3b844Bc9e7595f5b899", "Missing 0x"),
    ("0xGGGG35Cc6634C0532925a3b844Bc9e7595f5b899", "Invalid hex"),
    ("0x742d35", "Too short"),
    ("'; DROP TABLE users; --", "SQL injection"),
    ("<script>alert('xss')</script>", "XSS attempt"),
];
```

## Expected Test Outcomes

### Validation Tests
- All valid addresses parse successfully
- All invalid inputs return appropriate errors
- Error messages contain helpful context
- No panics or undefined behavior

### Resilience Tests
- Retry delays follow exponential pattern
- Circuit breaker prevents cascade failures
- Rate limiter maintains accurate counts
- All patterns thread-safe

### Performance Benchmarks
- Address validation: < 1 microsecond
- Error creation: < 100 nanoseconds
- Retry overhead: < 10% of operation time
- Circuit breaker check: < 50 nanoseconds

## Running the Tests

```bash
# Run all core_types tests
cargo test -p qarqa-core-types

# Run with logging
RUST_LOG=debug cargo test -p qarqa-core-types -- --nocapture

# Run specific test
cargo test -p qarqa-core-types test_address_validation

# Run benchmarks
cargo bench -p qarqa-core-types
```

## Test Maintenance

- Review test coverage monthly
- Update test data with new edge cases
- Benchmark after performance changes
- Document any flaky tests
- Keep tests independent and deterministic