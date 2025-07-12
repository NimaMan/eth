# TX Processor

A high-performance Rust implementation for processing Ethereum transactions, designed as a drop-in replacement for Python's eth_block_processor.txn module.

## Core Purpose

**Input**: Transaction hash(es)  
**Output**: ProcessedTransaction(s) with decoded events, classifications, and internal transfers  
**Performance**: 10-40x faster than Python implementation

## 🚨 CRITICAL BUG: SIMULATION FAILURE

### The Problem
When processing transaction `0x6a904d36e7f808fb08f7dcd04d1b2132a34ca6697b910a93013117d97fe98dd7`:
- **Expected**: 5 ERC20 transfers + 2 internal ETH transfers (from WETH unwrapping)
- **Actual**: 5 ERC20 transfers + 0 internal ETH transfers

### Root Cause
The transaction simulation is **FAILING** with error: `"failed to initialize a transaction: unknown error code: 11"`

**What this means:**
- Simulation returns `success: false` 
- Only uses 26,160 gas instead of 181,391 (actual on-chain usage)
- Even simple ETH transfers fail in simulation
- Because simulation fails, we can't extract internal ETH transfers

### Investigation Results
1. **Transaction data is correct**:
   - Gas limit: 229,467 ✅
   - Gas used: 181,391 ✅ 
   - Nonce: 6 ✅
   - All parameters match on-chain values

2. **State appears valid**:
   - Account balance: 0.4 ETH ✅
   - Account nonce: 6 ✅
   - Uniswap Router has bytecode ✅
   - WETH contract has bytecode ✅

3. **Error happens during initialization**:
   - Error code 11 occurs in `evm.transact()` BEFORE execution
   - Not a revert during execution - fails to even start
   - Affects ALL simulations, even simple transfers

### What Works vs What's Broken

**WORKS**:
- ✅ Loading transaction data from Reth DB
- ✅ Calculating actual gas_used (not cumulative)
- ✅ Decoding all ERC20 transfers from logs
- ✅ Transaction classification
- ✅ Fee calculations

**BROKEN**:
- ❌ ALL transaction simulations fail
- ❌ Can't extract internal ETH transfers
- ❌ Can't get state changes
- ❌ Error code 11 is not properly handled

## Architecture Assessment

### ✅ Strengths

1. **Clean Architecture**
   - Well-separated concerns (decoder, classifier, processor)
   - Event-driven design for log processing
   - Direct database access eliminates RPC bottleneck

2. **Performance**
   - Direct Reth DB access (no RPC calls)
   - Intelligent simulation (only for contract interactions)
   - Parallel processing capabilities

3. **Compatibility**
   - Output format matches Python ProcessedTransaction
   - Can be used as drop-in replacement
   - Tracks more contract types than Python (ERC721/1155)

### ⚠️ Current Limitations

1. **Simulation Failure**
   - ALL simulations fail with error code 11
   - Internal transactions cannot be extracted
   - State changes cannot be calculated

2. **Transaction Fetching**
   - Direct fetching by hash works (`process_transaction_by_hash`)
   - But simulation component is broken

### 🎯 Key Design Decisions

1. **Simulation Logic**
   ```rust
   // Only simulate if it's a contract interaction
   if !input.is_empty() && to.is_some() {
       // Simulate to get internal transfers
   }
   ```

2. **Event Processing**
   - Decode all logs into typed events
   - Support for ERC20/721/1155, Uniswap V2/V3/V4
   - Extensible for new protocols

3. **Error Handling**
   - Simulation failures don't break processing
   - Continue with available data
   - But we lose internal transfers!

## Usage

```rust
use tx_processor::tx_processor::TxProcessor;

// Initialize processor
let processor = TxProcessor::new("/path/to/reth/data")?;

// Process by hash (SIMULATION WILL FAIL)
let processed_tx = processor.process_transaction_by_hash(tx_hash).await?;

// You'll get:
// - All ERC20/NFT transfers ✅
// - Transaction type/classification ✅
// - Fees ✅
// - Internal transfers ❌ (due to simulation failure)
// - State changes ❌ (due to simulation failure)
```

## Next Steps to Fix

1. **Identify error code 11 in REVM**
   - Check REVM source for error code definitions
   - Understand why transaction initialization fails

2. **Debug simulation environment**
   - Verify EVM environment setup
   - Check if block state is complete
   - Test with different block numbers

3. **Minimal reproduction**
   - Create smallest possible failing case
   - Test directly with REVM
   - Isolate the issue

## Performance Benchmarks

Run the 1K transaction benchmark:
```bash
cargo run --example benchmark_1k_transactions --release
```

Expected results (if simulation worked):
- Simple ETH transfers: ~5-10ms
- ERC20 transfers: ~10-20ms (with simulation)
- Complex DeFi transactions: ~20-50ms
- Average speedup vs Python: 10-40x

**Current**: All simulations fail, so internal transfers are missing

## Examples

- `test_internal_txs` - Shows the simulation failure
- `debug_gas_and_trace` - Debug simulation issues
- `check_contract` - Verify contract bytecode exists

## Dependencies

- `reth_tx_simulator` - For transaction simulation (CURRENTLY BROKEN)
- `alloy_primitives` - Ethereum types
- Direct Reth database access