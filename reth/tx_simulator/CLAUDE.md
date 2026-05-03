# Transaction Simulator - Technical Reference

## Module Overview

The `tx_simulator` is a **modular, high-performance Ethereum transaction simulator** that provides direct database access to Reth's state. It's designed as a clean replacement for the monolithic `reth_tx_simulator`, with better organization and focused functionality.

## Core Purpose

- **Direct state access**: Reads directly from Reth's MDBX database files (resolved from `RETH_DATADIR`, `RETH_DB_PATH`, or the repository-level `config.env`)
- **Multiple simulation types**: Signed transactions, unsigned calls, view functions, sequential processing
- **Pure simulation focus**: Returns raw CallFrame traces and basic results (success, gas, revert reason)
- **Parallel processing**: Concurrent simulation of thousands of transactions
- **No signatures required**: Can simulate unsigned transactions (like `debug_traceCall`)
- **Clean architecture**: Simulation only - result processing handled by tx_processor

## Architecture Overview

### Modular Design

Unlike the monolithic `reth_tx_simulator` (54KB single file), `tx_simulator` is organized into focused modules:

```
tx_simulator/
├── src/
│   ├── lib.rs                          # Module exports and re-exports
│   ├── simulator.rs                    # Core TxSimulator struct and DB access
│   ├── types.rs                        # All result types and data structures
│   ├── signed_simulation.rs            # Signed transaction processing
│   ├── unsigned_simulation.rs          # Unsigned call aliases
│   ├── call_simulator.rs               # UnsignedTransaction handling and simulation
│   ├── batch_sequence_simulation.rs    # Sequential transaction processing (state preservation)
│   ├── single_tx/parallel.rs           # Independent parallel processing
│   ├── simulation_chain.rs             # Stateful step-by-step simulation
│   └── view_function_simulator.rs      # View/pure function execution
└── examples/
    ├── basic/                           # Basic simulation examples
    │   ├── verify_database_setup.rs     # Database connection verification
    │   ├── unsigned_transaction_example.rs # Basic UnsignedTransaction simulation
    │   ├── contract_method_simulation.rs     # ERC20 view function calls
    │   ├── contract_method_simulation_weth_total_supply.rs # WETH totalSupply example
    │   └── trace_extraction_example.rs  # CallFrame structure demonstration
    ├── sequential/                      # Sequential transaction examples
    │   ├── buy_approve_then_sell.rs     # Stateful DeFi workflow (SimulationChain)
    │   ├── batch_sequence_example.rs    # Batch sequence processing
    │   ├── mev_sandwich_bundle_example.rs # MEV sandwich attack simulation
    │   └── auto_nonce_management_example.rs # Automatic nonce handling
    ├── performance/                     # Performance testing
    │   └── rpc_vs_direct_simulation_benchmark.rs # Performance comparison
    └── advanced/                        # Advanced features
        └── timeout_handling_example.rs  # Timeout mechanism demonstration
```

### Key Architectural Benefits

1. **Separation of Concerns**: Each simulation type has its own module
2. **Clean Dependencies**: No circular dependencies (unlike reth_tx_simulator ↔ reth_chain_query)
3. **Type Organization**: All types centralized in `types.rs`
4. **Focused Functionality**: Each module has a single responsibility
5. **Better Maintainability**: Easier to understand and modify

## Core Components

### 1. Simulator Core (`simulator.rs`)

The main `TxSimulator` struct manages database connections and provides foundational methods:

```rust
pub struct TxSimulator {
    pub(crate) provider_factory: ProviderFactory<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>,
    pub(crate) evm_config: EthEvmConfig,
}
```

**Key Methods:**
- `new(reth_datadir: &str)` - Initialize with database path
- `with_provider_factory()` - Share database connection across components
- `get_latest_block()` - Get latest block number from local database
- `get_base_fee_at_block()` - Get base fee for EIP-1559 transactions
- `provider_factory()` - Get direct database access for advanced use cases

**Example:** See `examples/general/verify_database_setup.rs`

### 2. Type Definitions (`types.rs`)

Centralized type definitions for all simulation results:

```rust
// Basic simulation result
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
}

// Full simulation with traces
pub struct FullSimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub call_trace: CallFrame,
    pub logs: Vec<alloy_primitives::Log>,
}

// Sequential simulation results
pub struct SequentialSimulationResult {
    pub total_transactions: usize,
    pub successful_transactions: usize,
    pub failed_transactions: usize,
    pub total_gas_used: u64,
    pub results: Vec<SequentialTransactionResult>,
    pub sequence_success: bool,
}

// Chain state information
pub struct ChainStateInfo {
    pub block_number: u64,
    pub transaction_count: usize,
    pub total_gas_used: u64,
    pub nonces: HashMap<Address, u64>,
}
```

### 3. Simulation Modules

#### Signed Transaction Processing (`signed_simulation.rs`)

Handles transactions with valid signatures (v, r, s):

- `simulate_signed_transaction()` - Basic signed transaction simulation
- `simulate_signed_transaction_at_block()` - Historical simulation

**Example:** Standard mempool transaction processing

#### Unsigned Transaction Processing (`call_simulator.rs`)

Core module for `debug_traceCall` equivalent functionality:

- `simulate_unsigned_transaction()` - Basic unsigned simulation (latest block)
- `simulate_unsigned_transaction_at_block()` - Basic simulation at a specific block
- `simulate_unsigned_transaction_with_trace()` - Returns raw CallFrame traces
- `simulate_unsigned_transaction_with_full_trace_at_block()` - Full simulation with CallFrame

**Examples:**
- `examples/general/unsigned_transaction_example.rs` - Basic UnsignedTransaction usage
- `examples/general/trace_extraction_example.rs` - CallFrame structure demonstration

#### Sequential Processing (`batch_sequence_simulation.rs`)

For MEV bundle simulation and complex transaction sequences where all transactions are known upfront:

- `simulate_unsigned_tx_sequence()` - Process complete sequence with state preservation
- Configurable failure handling (stop_on_failure)
- Automatic nonce management

**Examples:**
- `examples/sequential/batch_sequence_example.rs` - Basic sequence processing
- `examples/sequential/mev_sandwich_bundle_example.rs` - MEV bundle simulation

#### Stateful Simulation (`simulation_chain.rs`)

Interactive step-by-step simulation with state preservation between steps:

```rust
pub struct SimulationChain {
    // Maintains forked blockchain state between transactions
}

// Key methods:
pub async fn step(&mut self, call: UnsignedTransaction) -> Result<SimulationResult>
pub async fn step_with_trace(&mut self, call: UnsignedTransaction) -> Result<FullSimulationResult>
pub fn current_state(&self) -> ChainStateInfo
```

**Key Features:**
- State persistence between `step()` calls
- Automatic nonce tracking and increment
- Ability to inspect state between transactions

**Example:** `examples/sequential/buy_approve_then_sell.rs` - DeFi workflow with state inspection

#### Parallel Processing (`single_tx/parallel.rs`)

Independent parallel processing of multiple transactions (no state sharing):

- `simulate_signed_tx_list_parallel()` - Process signed transactions concurrently
- `simulate_unsigned_tx_list_parallel()` - Process UnsignedTransactions concurrently
- Configurable parallelism (semaphore-based concurrency control)
- Per-transaction timeouts
- Aggregated statistics and error handling

**Example:** `examples/performance/rpc_vs_direct_simulation_benchmark.rs`

#### View Functions (`view_function_simulator.rs`)

For calling view/pure functions that don't modify state:

- `simulate_view_function()` - Execute view functions
- `simulate_view_function_from_call()` - From UnsignedTransaction
- Helper functions for encoding/decoding common types
- Optimized for read-only operations

**Examples:**
- `examples/general/contract_method_simulation.rs` - ERC20 view function calls
- `examples/general/contract_method_simulation_weth_total_supply.rs` - Simple totalSupply call

### 4. Advanced Features

#### Nonce Management (`auto_nonce_management_example.rs`)

Automatic nonce detection and management:
- Queries actual nonce from blockchain state
- Handles "nonce too low" scenarios
- Automatic increment in sequential simulations

#### Timeout Handling (`timeout_handling_example.rs`)

Robust timeout mechanisms:
- Uses `tokio::spawn_blocking` for CPU-intensive operations
- Configurable per-transaction timeouts
- Prevents stuck simulations from blocking the runtime

## Simulation Method Reference

### Basic Simulation

```rust
// Unsigned transaction (like debug_traceCall)
let result = simulator.simulate_unsigned_transaction(unsigned_tx).await?;

// At specific block
let result = simulator.simulate_unsigned_transaction_at_block(unsigned_tx, block_number).await?;
```

### Advanced Simulation

```rust
// Full simulation with CallFrame trace
let full_result = simulator
    .simulate_unsigned_transaction_with_full_trace_at_block(unsigned_tx, block_number)
    .await?;

// Access simulation results and raw CallFrame (processing done by tx_processor)
println!("Success: {}", full_result.success);
println!("Gas used: {}", full_result.gas_used);
println!("Call trace depth: {}", full_result.call_trace.calls.len());
```

### Sequential Processing (Batch)

```rust
let options = SequentialSimulationOptions {
    at_block: Some(block_number),
    stop_on_failure: true,
    auto_increment_nonces: true,
    gas_limit_per_tx: Some(300_000),
};

let result = simulator
    .simulate_unsigned_tx_sequence(transactions, options)
    .await?;
```

### Stateful Chain Simulation

```rust
// Start a simulation chain
let mut chain = simulator.start_simulation_chain(Some(block_number)).await?;

// Execute transactions step by step
let buy_result = chain.step(buy_tx).await?;
let approve_result = chain.step(approve_tx).await?;
let sell_result = chain.step_with_trace(sell_tx).await?;

// Inspect current state
let state = chain.current_state();
println!("Transactions executed: {}", state.transaction_count);
println!("Total gas used: {}", state.total_gas_used);
```

### Parallel Processing

```rust
let options = ParallelTxSimulationOptions {
    max_concurrent: 20,
    timeout_per_tx: Some(Duration::from_millis(100)),
    block_number: Some(block_number),
};

let results = simulator.simulate_unsigned_tx_list_parallel(requests, options).await?;
```

## Example Reference

### Basic Examples

1. **Database Setup** (`verify_database_setup.rs`)
   - Verifies Reth database connection
   - Tests basic block access
   - Validates database permissions

2. **Unsigned Transactions** (`unsigned_transaction_example.rs`)
   - Basic UnsignedTransaction simulation
   - ETH transfers and contract calls
   - Success/failure scenarios

3. **View Functions** (`contract_method_simulation.rs`, `contract_method_simulation_weth_total_supply.rs`)
   - ERC20 function calls (balanceOf, totalSupply, decimals)
   - Read-only operations
   - Return value decoding

4. **CallFrame Structure** (`trace_extraction_example.rs`)
   - CallFrame structure demonstration
   - Shows raw simulation output
   - Notes that processing is done by tx_processor

### Sequential Examples

1. **Stateful Workflow** (`buy_approve_then_sell.rs`)
   - DeFi workflow with state preservation
   - Step-by-step execution using SimulationChain
   - Real-time state inspection between steps

2. **Batch Sequences** (`batch_sequence_example.rs`)
   - Complete transaction sequences
   - MEV-style bundle processing
   - State persistence across all transactions

3. **MEV Simulation** (`mev_sandwich_bundle_example.rs`)
   - Sandwich attack patterns
   - Frontrun → Victim → Backrun sequences
   - Educational MEV analysis

4. **Nonce Management** (`auto_nonce_management_example.rs`)
   - Automatic nonce detection
   - Handling existing transaction nonces
   - Sequential transaction nonce increment

### Performance Examples

1. **Benchmarking** (`rpc_vs_direct_simulation_benchmark.rs`)
   - RPC vs direct database comparison
   - Throughput measurements
   - Performance optimization insights

### Advanced Examples

1. **Timeout Handling** (`timeout_handling_example.rs`)
   - Timeout mechanism demonstration
   - Spawn_blocking requirement proof
   - Robust error handling

## Performance Characteristics

### Actual Benchmarks (Measured, Not Estimated)

**Single Transaction:**
- Unsigned simulation: ~2.0ms per transaction (cold cache)
- Signed simulation: ~2.2ms per transaction (cold cache)
- View function calls: ~0.5ms per transaction

**Parallel Processing:**
- 100 transactions: ~2.0ms per transaction (cold cache)
- 1,000 transactions: ~1.2ms per transaction (warming cache)
- 10,000 transactions: ~0.64ms per transaction (hot cache)
- **Maximum throughput**: 1,560 transactions/second

**Key Insights:**
- Performance improves significantly with scale as database caches warm up
- Initial overhead is amortized over large batches
- Direct database access matches RPC performance at scale but with more flexibility

### Database Access Patterns

- **Read-only access**: Safe for concurrent use, no state modification
- **Cache-friendly**: Repeated access to same block state is optimized
- **Memory mapped**: MDBX provides zero-copy access to database pages
- **Static files**: Optimized access to headers and receipts

## Integration Points

### Used By

- **eth_prices**: For view function calls to DEX contracts
- **pyreth**: Python bindings for transaction analysis
- **mempool processors**: For simulating pending transactions

### Database Sharing

```rust
// Share database connection across components
let provider_factory = simulator.provider_factory().clone();
let other_component = OtherComponent::with_provider_factory(provider_factory);
```

### Dependencies

- **reth**: Core Ethereum node implementation
- **revm**: EVM execution engine  
- **revm-inspectors**: Call tracer for detailed execution traces
- **alloy-primitives**: Ethereum types and utilities

## Migration from reth_tx_simulator

### What Was Migrated

✅ **Core Simulation Functionality:**
- Signed transaction simulation → `signed_simulation.rs`
- Unsigned transaction simulation → `call_simulator.rs`
- Sequential transaction processing → `batch_sequence_simulation.rs`
- Parallel processing → `single_tx/parallel.rs`
- Raw CallFrame generation → Built into simulation methods
- View function calls → `view_function_simulator.rs`

✅ **NEW Features:**
- Stateful simulation chains → `simulation_chain.rs`
- Step-by-step transaction execution with state inspection
- Comprehensive example suite with clear naming

### What Was Intentionally Excluded

❌ **Advanced State Change Tracking:**
- `AddressStateChange` calculations
- Token balance change tracking
- ETH balance change detection
- Movement summaries and transfer analysis
- The entire `state_change_calculator.rs` module

**Rationale:** These features were excluded to keep `tx_simulator` focused on core simulation functionality. State change analysis belongs in higher-level modules that use the simulator.

### Migration Guide

**Old usage (reth_tx_simulator):**
```rust
let simulator = RethTxSimulator::new(path)?;
let result = simulator.simulate_unsigned_transaction_with_full_trace_at_block(call, block).await?;
```

**New usage (tx_simulator):**
```rust
let simulator = TxSimulator::new(path)?;
let result = simulator.simulate_unsigned_transaction_with_full_trace_at_block(call, block).await?;
```

**Type Compatibility:**
- `RethTxSimulator` → `TxSimulator`
- `UnsignedTransaction` → Same structure, same fields
- `SimulationResult` → Same structure
- `FullSimulationResult` → Same structure (minus state changes)

## Common Usage Patterns

### 1. Basic Transaction Analysis

```rust
let simulator = TxSimulator::new("/path/to/reth/db")?;
let result = simulator.simulate_unsigned_transaction(unsigned_tx).await?;

if result.success {
    println!("Transaction would succeed, gas: {}", result.gas_used);
} else {
    println!("Transaction would fail: {:?}", result.revert_reason);
}
```

### 2. MEV Bundle Simulation

```rust
let mev_bundle = vec![
    frontrun_call,
    victim_call,
    backrun_call,
];

let result = simulator
    .simulate_unsigned_tx_sequence(mev_bundle, SequentialSimulationOptions::default())
    .await?;

println!("Bundle success: {}", result.sequence_success);
```

### 3. Interactive DeFi Workflow

```rust
// Start simulation chain
let mut chain = simulator.start_simulation_chain(None).await?;

// Execute buy transaction
let buy_result = chain.step(buy_tx).await?;
println!("Buy: {} gas, {} ERC20 events", buy_result.gas_used, count_erc20_events(&buy_result));

// Execute approve transaction  
let approve_result = chain.step(approve_tx).await?;
println!("Approve: {} gas", approve_result.gas_used);

// Execute sell with full trace
let sell_result = chain.step_with_trace(sell_tx).await?;
println!("Sell: {} gas, {} events", sell_result.gas_used, sell_result.logs.len());

// Check final state
let state = chain.current_state();
println!("Total transactions: {}, Total gas: {}", state.transaction_count, state.total_gas_used);
```

### 4. Token Function Analysis

```rust
// Check token balance
let balance_call = UnsignedTransaction {
    to: Some(token_address),
    data: Some(encode_balance_of(user_address)),
    ..Default::default()
};

let result = simulator.simulate_view_function_from_call(balance_call).await?;
let balance = result.decode_uint256();
```

### 5. Concurrent Transaction Testing

```rust
let test_transactions: Vec<UnsignedTransaction> = generate_test_calls();

let options = ParallelTxSimulationOptions {
    max_concurrent: 50,
    timeout_per_tx: Some(Duration::from_secs(1)),
    block_number: Some(latest_block),
};

let results = simulator.simulate_unsigned_tx_list_parallel(test_transactions, options).await?;
println!("Success rate: {}/{}", results.successful, results.total);
```

## Error Handling

### Common Errors

- **Database Access**: "No such file or directory", "Permission denied"
- **Transaction Execution**: "Out of gas", "Nonce too low", "Insufficient balance"
- **Block Access**: "No header for block", "Block not found"
- **Timeout**: "Operation timed out", "Spawn blocking failed"

### Error Recovery Patterns

```rust
match simulator.simulate_unsigned_transaction(unsigned_tx).await {
    Ok(result) => {
        if result.success {
            process_success(&result);
        } else {
            handle_revert(&result.revert_reason);
        }
    }
    Err(e) => {
        if e.to_string().contains("out of gas") {
            retry_with_higher_gas_limit();
        } else {
            log_simulation_error(&e);
        }
    }
}
```

## Testing Strategy

### Unit Tests

Each module includes focused unit tests:
- `signed_simulation` tests with valid signatures
- `call_simulator` tests with various UnsignedTransaction configurations
- `batch_sequence_simulation` tests with complex sequences
- `simulation_chain` tests with stateful workflows
- CallFrame structure tests with known transaction patterns

### Integration Tests

- End-to-end simulation workflows
- Database connection and access patterns
- Error handling and recovery
- Performance benchmarks

### Example-Based Testing

The `examples/` directory serves as comprehensive integration tests:
- Each example demonstrates specific functionality
- Examples are regularly run to verify correctness
- Real mainnet transactions used for realistic testing
- Clear naming convention for easy navigation

## Security Considerations

- **Read-only Database Access**: No modification of actual blockchain state
- **Isolated Execution**: Each simulation runs in its own forked state
- **No Network Access**: All data comes from local database
- **Safe Concurrent Use**: Multiple simulators can access database simultaneously
- **Input Validation**: UnsignedTransaction parameters are validated before execution

## Future Enhancements

Potential areas for expansion:

1. **State Diff Analysis**: Compare state before/after simulation
2. **Gas Optimization**: Suggest optimal gas parameters
3. **Trace Visualization**: Export traces in standard formats
4. **Custom Inspectors**: Support for user-defined execution inspectors
5. **Historical Analysis**: Bulk processing of historical blocks

## Dependencies and Versions

- **reth**: v1.3.12+ (using local paths)
- **revm**: v22.0.1 (exact version compatibility with Reth)
- **revm-inspectors**: v0.19.1
- **alloy-primitives**: v1.0+ (Ethereum types)
- **tokio**: v1.0+ (async runtime)

## Contributing

When adding new functionality:

1. **Follow module organization**: Put new features in appropriate modules
2. **Add comprehensive tests**: Unit tests and examples
3. **Update documentation**: README.md and CLAUDE.md
4. **Maintain performance**: Benchmark new features
5. **Keep it focused**: Avoid feature creep, maintain clear separation of concerns
6. **Create clear examples**: Add examples with descriptive names that demonstrate the new functionality
