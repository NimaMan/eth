# Tests

## Overview

Integration tests for validating mempool processor functionality across Rust and Python systems.

## Test Components

### Live Pool Detection Test
- **Purpose**: Validates Rust mempool processor correctly detects transactions on DeFi pools published by Python system
- **Key Files**: 
  - `test_live_pool_detection.rs` - Main Rust test
  - `pool_publisher_test.py` - Test pool publisher
  - `run_live_pool_detection_test.sh` - Test runner

### Validation Tests
- **Purpose**: Address compatibility and cross-system integration testing
- **Files**: 
  - `test_zmq_subscriber.py` - ZMQ subscription testing
  - `validate_test_setup.sh` - Environment validation

### **Rust vs Python State Change Comparison Test** 🆕
- **Purpose**: Comprehensive validation that Rust and Python detect identical state changes for complex transactions
- **Files**:
  - `rust_python_state_comparison_test.py` - Main comparison test framework
  - `rust_validation_expectations.json` - Expected results for Rust validation
- **Test Coverage**: 3 complex transactions with internal transfers, DeFi interactions, and MEV activity

## Running Tests

### Quick Live Pool Detection Test
```bash
./run_live_pool_detection_test.sh
```

### Full Test with Options
```bash
./run_live_pool_detection_full.sh --verbose
```

### **Complex Transaction State Change Comparison** 🆕
```bash
# Activate conda environment first
source /home/nima/miniconda3/etc/profile.d/conda.sh && conda activate qw

# Run comprehensive Rust vs Python comparison
python tests/rust_python_state_comparison_test.py
```

### Manual Testing
```bash
# Terminal 1: Start pool publisher
python3 pool_publisher_test.py

# Terminal 2: Run Rust test  
cargo run --release --bin test_live_pool_detection
```

## Test Features

1. **EIP-55 Address Checksumming** - Ensures Rust and Python use compatible address formats
2. **ZMQ Pool Subscription** - Validates real-time pool updates via ZeroMQ
3. **Transaction Detection** - Monitors mempool for pool interactions  
4. **Cross-System Integration** - Tests Python → Rust data flow
5. **🆕 Complex State Change Validation** - Compares Rust and Python state change detection on complex transactions

## Expected Results

- Pool cache populated with 8+ pools
- Transactions detected interacting with known pools
- Address formats match between systems
- No errors in processing pipeline
- **🆕 Identical state changes detected by both Rust and Python systems**

## **Complex Transaction Test Details** 🆕

### Test Transactions Selected
1. **Transaction 1**: `0x290fa323...` (Complexity Score: 43)
   - 1 ETH value transfer, 14 internal calls, 7 contract logs
   - Expected: 2 addresses with ETH changes (sender loses 1 ETH, WETH contract gains 1 ETH)

2. **Transaction 2**: `0x01e025cc...` (Complexity Score: 35) 
   - 0.051 ETH multi-step transaction, 11 internal calls, 5 logs
   - Expected: 5 addresses with complex ETH flow distribution

3. **Transaction 3**: `0x799252ab...` (Complexity Score: 22)
   - MEV transaction with minimal ETH value, 5 internal calls, 4 logs  
   - Expected: 2 addresses with nano-ETH changes (7.224e-09 ETH)

### Validation Criteria
- **ETH Precision**: Must match within 1e-15 tolerance
- **Address Coverage**: All addresses with state changes must be detected by both systems
- **Movement Tracking**: In/out counts and totals must match
- **Zero Filtering**: Both systems must filter out addresses with no meaningful changes

### Current Status
- ✅ **Python Analysis**: Complete baseline established for all 3 transactions
- ⏳ **Rust Analysis**: Pending (dependency issues to be resolved)
- 📋 **Expectations**: Generated in `rust_validation_expectations.json`

When Rust compilation issues are resolved, this test will:
1. Run `cargo run --bin test_comprehensive_state_diff` for each transaction
2. Compare address-by-address ETH and token changes  
3. Validate precision and consistency
4. Generate pass/fail report with detailed discrepancy analysis

## Integration

These tests validate the core functionality that enables the production scam detection service to:
1. Subscribe to pool updates from Python system
2. Apply EIP-55 checksumming for all addresses
3. Detect pool interactions in real-time
4. **🆕 Generate identical state change analysis as Python system**
5. Trigger protective measures when needed

## Test Framework Architecture

The complex transaction comparison test follows this architecture:
1. **Transaction Selection**: Automated complexity scoring to find real-world complex transactions
2. **Python Baseline**: Comprehensive state change analysis using proven Python implementation  
3. **Rust Validation**: When available, runs identical analysis with Rust system
4. **Precision Comparison**: Address-by-address validation with configurable tolerance
5. **Expectation Management**: Stored expectations enable regression testing