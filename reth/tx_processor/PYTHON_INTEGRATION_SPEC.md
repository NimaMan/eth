# Python Integration Test Specification

## Overview

The Python integration test validates that Rust transaction processing produces identical state changes to the Python implementation. Both systems process the same transaction and compare the resulting state changes to ensure accuracy.

## Test Architecture

```
┌─────────────────┐    ┌─────────────────┐
│  Rust Process   │    │ Python Service  │
│     TX          │    │     (Port       │
│                 │    │     18000)      │
└─────────────────┘    └─────────────────┘
         │                       │
         │ Extract State Changes  │
         ▼                       ▼
┌─────────────────┐    ┌─────────────────┐
│ Rust State      │    │ Python State    │
│ Changes         │    │ Changes         │
└─────────────────┘    └─────────────────┘
         │                       │
         └───────┬───────────────┘
                 ▼
        ┌─────────────────┐
        │   Comparison    │
        │   Algorithm     │
        └─────────────────┘
                 │
                 ▼
        ┌─────────────────┐
        │ Validation      │
        │ Result          │
        └─────────────────┘
```

## Data Flow

### 1. Rust Processing
```rust
let rust_result = extract_state_changes_python_format(tx_hash, rpc_url).await?;
```

**Rust Output Format:**
```json
{
  "state_changes": {
    "0x742d35Cc6641C5bD23d8F8e8E8B94f39C3B66eB3": {
      "eth_net": "-0.5",
      "token_net": {
        "USDC": "1000.0",
        "WETH": "0.5"
      }
    }
  },
  "metadata": {
    "tx_hash": "0x...",
    "processing_time_ms": 25.5,
    "addresses_affected": 2,
    "tokens_involved": 2
  }
}
```

### 2. Python Processing
```http
POST http://127.0.0.1:18000/validate/transaction/0x1234...
{
  "tx_hash": "0x1234...",
  "include_state_changes": true,
  "include_trace": true
}
```

**Python Output Format:**
```json
{
  "success": true,
  "tx_hash": "0x...",
  "processed_transaction": {
    "state_changes": {
      "0x742d35Cc6641C5bD23d8F8e8E8B94f39C3B66eB3": {
        "eth_net": "-0.5",
        "token_net": {
          "USDC": "1000.0", 
          "WETH": "0.5"
        }
      }
    }
  },
  "processing_time_ms": 45.2
}
```

### 3. Comparison Process
```rust
let validation_result = compare_with_python(tx_hash, rpc_url, python_url).await?;
```

## Equality Rules

### Core Principle
**Two state change results are considered equal if they represent the same final state for all addresses, with acceptable formatting variations.**

### 1. Address Equality Rules

#### ✅ **Addresses with Zero Changes (Acceptable)**
- **Rust**: Address appears with `eth_net: "0"` and empty `token_net: {}`
- **Python**: Address does not appear in results
- **Verdict**: ✅ **EQUAL** - Zero changes can be represented by presence with zeros or absence

**Example:**
```json
// Rust
{
  "0x1234...": {
    "eth_net": "0",
    "token_net": {}
  }
}

// Python  
{
  // Address not present
}

// Result: EQUAL ✅
```

#### ✅ **Address Order (Irrelevant)**
- Address order in the map does not matter
- Both implementations may return addresses in different orders

#### ❌ **Missing Non-Zero Changes (Error)**
- If an address has non-zero changes in one implementation but is missing in the other
- **Verdict**: ❌ **NOT EQUAL**

### 2. ETH Net Change Equality

#### ✅ **Acceptable ETH Formats**
```json
// All equivalent representations of zero ETH
"eth_net": "0"
"eth_net": "0.0" 
"eth_net": "0.000000000000000000"

// All equivalent representations of 1.5 ETH
"eth_net": "1.5"
"eth_net": "1.500000000000000000"
"eth_net": "1.5000"

// All equivalent representations of -0.5 ETH  
"eth_net": "-0.5"
"eth_net": "-0.500000000000000000"
```

#### ❌ **Unacceptable ETH Differences**
```json
// Different actual values
"eth_net": "1.5"  vs  "eth_net": "1.6"  // ❌ NOT EQUAL

// Wrong signs
"eth_net": "1.5"  vs  "eth_net": "-1.5"  // ❌ NOT EQUAL
```

### 3. Token Net Change Equality

#### ✅ **Acceptable Token Formats**
```json
// USDC (6 decimals) - all equivalent
"USDC": "1000"
"USDC": "1000.0"
"USDC": "1000.000000"

// WETH (18 decimals) - all equivalent  
"WETH": "0.5"
"WETH": "0.500000000000000000"

// Zero amounts - all equivalent
"USDC": "0"
"USDC": "0.0"
// Or token not present in token_net map
```

#### ✅ **Missing Zero Token Amounts**
- **Rust**: `"token_net": {"USDC": "0", "WETH": "1.5"}`
- **Python**: `"token_net": {"WETH": "1.5"}` (USDC omitted)
- **Verdict**: ✅ **EQUAL** - Zero amounts can be omitted

#### ❌ **Unacceptable Token Differences**
```json
// Different amounts
"USDC": "1000.0"  vs  "USDC": "999.0"  // ❌ NOT EQUAL

// Different signs
"USDC": "1000.0"  vs  "USDC": "-1000.0"  // ❌ NOT EQUAL

// Missing non-zero amounts
{"USDC": "1000.0"}  vs  {}  // ❌ NOT EQUAL
```

### 4. Token Symbol Consistency

#### ✅ **Required Symbol Mapping**
Both implementations must use the same token symbols:
```rust
USDC, USDT, DAI, WETH, WBTC, MATIC, LINK, UNI, AAVE, etc.
```

#### ❌ **Symbol Mismatches**
- **Rust**: `"USDC": "1000"`
- **Python**: `"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48": "1000"`
- **Verdict**: ❌ **NOT EQUAL** - Must use consistent symbol names

## Comparison Algorithm

### Implementation
```rust
pub fn compare_state_changes(
    rust_changes: &PythonCompatibleStateChanges,
    python_changes: &serde_json::Value,
) -> ValidationResult {
    let mut differences = Vec::new();
    
    // 1. Extract Python state changes
    let python_state_changes = extract_python_state_changes(python_changes)?;
    
    // 2. Normalize both datasets
    let rust_normalized = normalize_state_changes(&rust_changes.state_changes);
    let python_normalized = normalize_state_changes(&python_state_changes);
    
    // 3. Compare all addresses
    let all_addresses = get_all_addresses(&rust_normalized, &python_normalized);
    
    for address in all_addresses {
        let rust_change = rust_normalized.get(address).unwrap_or(&default_change());
        let python_change = python_normalized.get(address).unwrap_or(&default_change());
        
        // Compare ETH changes
        if !eth_amounts_equal(&rust_change.eth_net, &python_change.eth_net) {
            differences.push(ValidationDifference {
                field: format!("{}.eth_net", address),
                rust_value: serde_json::to_value(&rust_change.eth_net).unwrap(),
                python_value: serde_json::to_value(&python_change.eth_net).unwrap(),
                description: "ETH net change mismatch".to_string(),
            });
        }
        
        // Compare token changes
        compare_token_changes(address, &rust_change.token_net, &python_change.token_net, &mut differences);
    }
    
    ValidationResult {
        matches: differences.is_empty(),
        differences,
        // ... other fields
    }
}
```

### Normalization Process
```rust
fn normalize_state_changes(changes: &HashMap<String, AddressStateChange>) -> HashMap<String, NormalizedChange> {
    let mut normalized = HashMap::new();
    
    for (address, change) in changes {
        let normalized_change = NormalizedChange {
            address: address.clone(),
            eth_net: normalize_decimal(&change.eth_net),
            token_net: normalize_token_map(&change.token_net),
        };
        
        // Only include addresses with non-zero changes
        if !is_zero_change(&normalized_change) {
            normalized.insert(address.clone(), normalized_change);
        }
    }
    
    normalized
}

fn normalize_decimal(amount_str: &str) -> BigDecimal {
    // Parse string to BigDecimal for precise comparison
    // Remove trailing zeros: "1.500000" -> "1.5"
    BigDecimal::from_str(amount_str).unwrap().normalized()
}

fn is_zero_change(change: &NormalizedChange) -> bool {
    change.eth_net.is_zero() && change.token_net.values().all(|amount| amount.is_zero())
}
```

## Test Cases

### 1. Perfect Match
```json
// Rust
{
  "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}}
}

// Python
{
  "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}}
}

// Result: ✅ EQUAL
```

### 2. Zero Address Differences (Acceptable)
```json
// Rust
{
  "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}},
  "0x5678": {"eth_net": "0", "token_net": {}}
}

// Python
{
  "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}}
}

// Result: ✅ EQUAL (zero changes can be omitted)
```

### 3. Formatting Differences (Acceptable)
```json
// Rust
{
  "0x1234": {"eth_net": "1.500000000000000000", "token_net": {"USDC": "1000.000000"}}
}

// Python
{
  "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}}
}

// Result: ✅ EQUAL (same values, different formatting)
```

### 4. Value Mismatch (Error)
```json
// Rust
{
  "0x1234": {"eth_net": "1.5", "token_net": {"USDC": "1000"}}
}

// Python
{
  "0x1234": {"eth_net": "1.6", "token_net": {"USDC": "1000"}}
}

// Result: ❌ NOT EQUAL (different ETH values)
```

### 5. Missing Token (Error)
```json
// Rust
{
  "0x1234": {"eth_net": "0", "token_net": {"USDC": "1000", "WETH": "0.5"}}
}

// Python
{
  "0x1234": {"eth_net": "0", "token_net": {"USDC": "1000"}}
}

// Result: ❌ NOT EQUAL (missing WETH in Python)
```

## Error Reporting

### Difference Types
```rust
pub struct ValidationDifference {
    pub field: String,           // "0x1234.eth_net" or "0x1234.token_net.USDC"
    pub rust_value: serde_json::Value,
    pub python_value: serde_json::Value,
    pub description: String,     // Human-readable description
}
```

### Example Difference Report
```rust
ValidationResult {
    matches: false,
    differences: vec![
        ValidationDifference {
            field: "0x742d35Cc6641C5bD23d8F8e8E8B94f39C3B66eB3.eth_net".to_string(),
            rust_value: json!("1.5"),
            python_value: json!("1.6"),
            description: "ETH net change mismatch: Rust=1.5 ETH, Python=1.6 ETH".to_string(),
        },
        ValidationDifference {
            field: "0x742d35Cc6641C5bD23d8F8e8E8B94f39C3B66eB3.token_net.USDC".to_string(),
            rust_value: json!("1000.0"),
            python_value: json!(null),
            description: "Token present in Rust but missing in Python".to_string(),
        }
    ],
}
```

## Test Execution

### Running Integration Tests
```bash
# Unit tests (offline)
cargo test python_integration_tests

# Integration tests (requires services)
cargo test python_integration_tests -- --ignored

# Specific comparison test
cargo test test_compare_with_python_integration -- --ignored --nocapture
```

### Example Test Output
```
🔍 Comparing transaction: 0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060
✅ Transaction: 0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060
   Match Status: PASS
   Rust Processing: 25.1ms
   Python Processing: 45.2ms
   Performance: Rust is 1.8x faster
   Rust State Changes:
     Addresses affected: 2
     Tokens involved: 0
     0x742d35Cc: ETH: -0.021, Tokens: 0
     ... and 1 more addresses
```

### Debugging Failed Comparisons
```bash
# Run with detailed output
cargo run --bin python_comparison -- 0x1234... --verbose

# Expected output for failures:
❌ Transaction: 0x1234...
   Match Status: FAIL
   Differences Found: 2
     1. 0x1234.eth_net: ETH net change mismatch
     2. 0x1234.token_net.USDC: Token amount mismatch
```

## Configuration

### Tolerance Settings
```rust
pub struct ComparisonConfig {
    pub decimal_precision: u32,     // Default: 18 (wei precision)
    pub ignore_zero_addresses: bool, // Default: true
    pub ignore_zero_tokens: bool,   // Default: true
    pub strict_ordering: bool,      // Default: false
}
```

### Environment Variables
```bash
# Test configuration
RUST_RPC_URL=http://127.0.0.1:8545
PYTHON_SERVICE_URL=http://127.0.0.1:18000
COMPARISON_PRECISION=18
IGNORE_ZERO_CHANGES=true
```

This specification ensures that the Python integration test provides meaningful validation while accepting reasonable formatting differences and implementation variations.