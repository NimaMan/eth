# TX Processor

## Overview

The TX Processor is a high-performance Ethereum transaction processing module that provides **sub-millisecond** transaction analysis by combining direct database access with on-demand REVM simulation.

## Core Purpose

This module processes Ethereum transactions to extract:
- Internal ETH transfers via call tracing
- ERC20 token movements from event logs
- Gas usage and execution status
- Contract interactions and creation
- Processed transaction data, including:
    - Transaction metadata
    - Execution analysis
    - Internal transfers
    - State changes
    - Events (logs)
- Complete state changes (balances, storage, nonces)

## Architecture Principles
- NO network calls during processing. We laod the data from reth db when we need it. And we simulate the transaction using REVM.
### RETH DB
Reth db includes the following data:
- Transaction data (transaction, receipt, logs, block)
- State data (balances, storage, nonces)

### REVM
REVM is a transaction simulator that simulates the transaction execution on the Ethereum blockchain. It is used to extract the internal transfers and state changes.

## Module Structure

The project structure is defined in GUIDELINES.md. This document focuses on the module's functionality and usage.

## Core Components

### 1. **Transaction Processor** (`processor.rs`)
The main entry point that orchestrates the two-stage processing pipeline.

```rust
pub struct TransactionProcessor {
    db_provider: DatabaseProvider,
    simulator: TransactionSimulator,
    config: ProcessorConfig,
}

impl TransactionProcessor {
    /// Process a single transaction
    pub fn process_transaction(&self, tx_hash: B256) -> Result<ProcessedTransaction>;
    
    /// Process multiple transactions efficiently
    pub fn process_batch(&self, tx_hashes: Vec<B256>) -> Result<Vec<ProcessedTransaction>>;
}
```

### 2. **Database Provider** (`database/provider.rs`)
Direct access to Reth database for canonical on-chain data.

```rust
pub trait DatabaseProvider {
    /// Get transaction with receipt and logs
    fn get_transaction_data(&self, tx_hash: B256) -> Result<TransactionData>;
    
    /// Get multiple transactions in one query
    fn get_batch(&self, tx_hashes: &[B256]) -> Result<Vec<TransactionData>>;
}
```

### 3. **Transaction Simulator** (`signed_tx_simulator/`)
REVM-based simulation for extracting internal transfers and state changes.

```rust
// The signed_tx_simulator module contains:
// - simulation_core.rs: Core REVM simulation logic
// - call_tracer.rs: Call tracer for internal transfers
// - internal_transfer_tracker.rs: Internal transfer extraction

pub struct TransactionSimulator {
    /// Execute transaction and extract all state changes
    pub fn simulate(&self, tx: &TransactionData) -> Result<SimulationResult>;
    
    /// Simulate with call tracer for internal transfers
    pub fn simulate_with_tracer(&self, tx: &TransactionData) -> Result<TracedSimulation>;
}
```

### 4. **Data Types** (`types.rs`)
Core data structures used throughout the module.

```rust
/// Input: Raw transaction data from database
pub struct TransactionData {
    pub transaction: SignedTransaction,
    pub receipt: Receipt,
    pub block: BlockHeader,
}

/// Output: Fully processed transaction
pub struct ProcessedTransaction {
    // Transaction metadata
    pub hash: B256,
    pub block_number: u64,
    pub timestamp: u64,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_used: u64,
    pub status: bool,
    
    // Execution analysis
    pub transaction_type: TransactionType,
    pub internal_transfers: Vec<InternalTransfer>,
    pub state_changes: HashMap<Address, StateChange>,
    pub events: Vec<DecodedEvent>,
    
    // Performance metrics
    pub db_read_time_ms: f64,
    pub simulation_time_ms: f64,
}
```

## Usage Patterns

### Basic Usage
```rust
// Create processor
let processor = TransactionProcessor::new(config)?;

// Process single transaction
let result = processor.process_transaction(tx_hash)?;

// Access results
println!("Gas used: {}", result.gas_used);
println!("Internal transfers: {}", result.internal_transfers.len());
```

### Batch Processing
```rust
// Process multiple transactions efficiently
let tx_hashes = vec![hash1, hash2, hash3];
let results = processor.process_batch(tx_hashes)?;

// Results are returned in the same order as input
for (hash, result) in tx_hashes.iter().zip(results.iter()) {
    println!("{}: {} gas", hash, result.gas_used);
}
```

### Custom Configuration
```rust
let config = ProcessorConfig {
    enable_call_tracer: true,      // Extract internal transfers
    enable_state_diff: true,       // Extract state changes
    batch_size: 100,              // For batch processing
    cache_size: 10_000,           // LRU cache for recent transactions
};
```

## Performance Characteristics

### Database-Only Mode
- **Latency**: 0.3-0.5ms per transaction
- **Throughput**: 2,000-3,000 TPS
- **Data Available**: Receipt, logs, basic transfer

### Full Simulation Mode
- **Latency**: 2-5ms per transaction
- **Throughput**: 200-500 TPS
- **Data Available**: Everything including internal transfers

### Batch Processing
- **Optimization**: Groups by block for efficient state access
- **Speedup**: 5-10x compared to individual processing
- **Memory**: O(batch_size) memory usage

## Integration Examples

### With Trading System
```rust
// Fast path for arbitrage detection
let processor = TransactionProcessor::new_fast_mode();
let result = processor.process_transaction(mempool_tx)?;

if result.has_arbitrage_pattern() {
    // Execute counter-trade
}
```

### With Analytics Pipeline
```rust
// Full analysis for historical data
let processor = TransactionProcessor::new_full_mode();
let results = processor.process_block(block_number)?;

// Store in analytics database
analytics_db.insert_batch(results)?;
```

## Error Handling

The module uses strongly-typed errors:

```rust
#[derive(Error, Debug)]
pub enum ProcessorError {
    #[error("Transaction not found: {0}")]
    TransactionNotFound(B256),
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] DatabaseError),
    
    #[error("Simulation failed: {0}")]
    SimulationError(#[from] SimulationError),
}
```
## Quick Decision Guide

### Q: Can we extract internal transfers from Reth database without simulation?
**A: No.** Reth does not store trace data. Internal transfers must be computed via simulation.

### Q: What's the performance difference?
**A:** 
- Database only: **0.35ms** (no internal transfers)
- Database + Simulation: **2.0ms** (complete data)
- **5.7x slower** but provides complete transaction analysis

### Q: What's the recommended approach?
**A: Hybrid approach with smart detection:**
1. Always fetch from database first (0.35ms)
2. Detect if simulation needed (+0.05ms)
3. Simulate only complex transactions (+1.65ms)
4. Result: ~0.56ms average (90% fast path, 10% simulation)

## Implementation Strategy

### 1. Three Processing Tiers

```rust
enum ProcessingTier {
    Fast,      // DB only - 0.35ms - Token transfers, basic data
    Smart,     // Auto-detect - 0.40ms - Intelligent routing
    Complete,  // Full simulation - 2.0ms - All internal transfers
}
```

### 2. Smart Detection Rules

**Skip Simulation For:**
- Simple ETH transfers (to != contract)
- ERC20 token transfers (known selectors)
- Read-only calls (staticcall)
- Failed transactions (if only analyzing successful transfers)

**Require Simulation For:**
- Contract deployments (to == null)
- DEX interactions (router contracts)
- Multi-call contracts
- Transactions with ETH value to contracts
- Unknown function selectors with state changes

### 3. Optimization Techniques

**Database Level:**
```rust
// Batch fetch related data
let (tx, receipt, logs) = provider.get_transaction_data(tx_hash)?;

// Use read-only connections
let provider = factory.provider_read_only()?;

// Cache hot contract bytecode
let code_cache = LruCache::<Address, Bytecode>::new(1000);
```

**Simulation Level:**
```rust
// Reuse EVM instances
let evm_pool = EvmPool::new(max_instances: 10);

// Share state between transactions in same block
let mut block_cache = CacheDB::new(provider);
for tx in block_transactions {
    simulate_with_cache(tx, &mut block_cache)?;
}
```

### 4. Production Architecture

```
┌─────────────────┐
│  Request Queue  │
└────────┬────────┘
         │
    ┌────▼────┐
    │ Router  │──────► Smart Detection
    └────┬────┘
         │
    ┌────┴─────────────┬─────────────┐
    │                  │             │
┌───▼───┐      ┌───────▼──────┐  ┌──▼──────────┐
│  Fast │      │   Hybrid     │  │  Complete   │
│  Path │      │   Path       │  │  Analysis   │
│ (90%) │      │   (8%)       │  │   (2%)      │
└───┬───┘      └───────┬──────┘  └──────┬──────┘
    │                  │                 │
    └──────────────────┴─────────────────┘
                       │
                  ┌────▼────┐
                  │ Results │
                  └─────────┘
```
