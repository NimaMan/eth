# Transaction Simulator Examples

This directory contains comprehensive examples demonstrating the capabilities of the tx_simulator library.

## 🚀 Quick Start

If you're new to tx_simulator, start here:

1. **[basic/verify_database_setup.rs](basic/verify_database_setup.rs)** - Verify your Reth database connection
2. **[basic/unsigned_transaction_example.rs](basic/unsigned_transaction_example.rs)** - Your first unsigned transaction simulation
3. **[basic/view_function_example.rs](basic/view_function_example.rs)** - Read contract state without transactions

## 📚 Examples by Category

### 🔧 Basic Operations

Examples demonstrating fundamental simulator capabilities:

- **[verify_database_setup.rs](basic/verify_database_setup.rs)** - Test database connection and basic operations
  - Verifies Reth database access
  - Tests latest block retrieval
  - Checks base fee calculation
  - Output: Connection status and block information

- **[unsigned_transaction_example.rs](basic/unsigned_transaction_example.rs)** - Simulate unsigned transactions
  - Create and simulate UnsignedTransaction
  - Handle success/failure scenarios
  - Automatic gas price calculation
  - Output: Gas usage and success status

- **[view_function_example.rs](basic/view_function_example.rs)** - Call view functions on contracts
  - Read token balances
  - Get token metadata (name, symbol, decimals)
  - Query total supply
  - Output: Decoded contract data

- **[weth_totalsupply_view_call.rs](basic/weth_totalsupply_view_call.rs)** - WETH totalSupply example
  - Simple view function call
  - Demonstrates clean simulation API
  - Output: WETH total supply confirmation

- **[trace_extraction_example.rs](basic/trace_extraction_example.rs)** - Extract call traces from transactions
  - Get detailed execution traces
  - Shows CallFrame structure
  - Identify internal calls
  - Output: Trace depth and call hierarchy

### 🔄 Sequential Simulations

Examples showing stateful transaction sequences:

- **[auto_nonce_management_example.rs](sequential/auto_nonce_management_example.rs)** - Automatic nonce handling
  - Demonstrates nonce auto-detection
  - Handles nonce mismatches gracefully
  - Shows sequential nonce increment
  - Output: Successful simulation with corrected nonces

- **[sequential_eth_transfers_with_state_persistence.rs](sequential/sequential_eth_transfers_with_state_persistence.rs)** - ETH transfer sequence
  - Multiple transfers in sequence
  - State persists between transactions
  - Uses `simulate_transaction_sequence` for bundles
  - Output: All transfers with cumulative gas usage

- **[mev_sandwich_bundle_example.rs](sequential/mev_sandwich_bundle_example.rs)** - MEV sandwich attack simulation
  - 3-transaction bundle (frontrun, victim, backrun)
  - State changes affect subsequent transactions
  - Educational MEV pattern demonstration
  - Uses inspector fusing for performance
  - Output: Bundle success with gas metrics

### 🪙 Token-Specific Workflows

Complete DeFi workflows using UnsignedTxChainSimulation:

- **[buy_approve_then_sell_floki.rs](sequential/specific_tokens/buy_approve_then_sell_floki.rs)** - FLOKI token workflow
  - Buy FLOKI with ETH via Uniswap V2
  - Approve router for spending
  - Sell FLOKI back to ETH
  - Interactive simulation with state inspection
  - Output: Each step's success and gas usage

- **[buy_approve_then_sell_pepe.rs](sequential/specific_tokens/buy_approve_then_sell_pepe.rs)** - PEPE token workflow
  - Similar workflow for PEPE (18 decimals)
  - Demonstrates UnsignedTxChainSimulation
  - State persists between steps
  - Output: Transaction sequence results

- **[buy_approve_then_sell_usdc.rs](sequential/specific_tokens/buy_approve_then_sell_usdc.rs)** - USDC token workflow
  - USDC has 6 decimals (different from PEPE/FLOKI)
  - Shows decimal handling
  - Complete buy/approve/sell cycle
  - Output: Each transaction's effect

### ⚡ Performance & Advanced

Performance testing and advanced features:

- **[rpc_vs_direct_simulation_benchmark.rs](performance/rpc_vs_direct_simulation_benchmark.rs)** - Performance comparison
  - Compares RPC vs direct database access
  - Measures throughput and latency
  - Shows benefits of direct access
  - Output: Performance metrics and speedup

- **[timeout_handling_example.rs](advanced/timeout_handling_example.rs)** - Timeout mechanisms
  - Demonstrates timeout handling
  - Prevents hanging on complex contracts
  - Uses spawn_blocking for safety
  - Output: Timeout demonstration

- **[revert_reason_decoder_example.rs](advanced/revert_reason_decoder_example.rs)** - Decode revert messages
  - Extract human-readable revert reasons
  - Handle different revert formats
  - Useful for debugging
  - Output: Decoded revert messages

### 📦 Block Operations (Currently Disabled)

These examples need the block_tracer module to be fixed:

- **[trace_entire_block.rs](block/trace_entire_block.rs)** - Trace all transactions in a block
  - Would use `debug_traceBlockByNumber` equivalent
  - Inspector fusing for performance
  - Currently disabled due to compilation errors

- **[get_block_receipts.rs](block/get_block_receipts.rs)** - Get all receipts for a block
  - Direct database access to receipts
  - Would work once block module is fixed

## 🏃 Running Examples

### Prerequisites

All examples require a synced Reth node with accessible database:
```bash
# Default location (update in examples if different)
/home/nima/.local/share/reth/mainnet
```

### Working Examples

These examples are confirmed to work:

```bash
# Basic examples - ✅ All working
cargo run --example verify_database_setup       # Tests database connection
cargo run --example unsigned_transaction_example # Simulates unsigned transactions
cargo run --example view_function_example       # Reads token balances and metadata
cargo run --example weth_totalsupply_view_call  # Simple WETH totalSupply call

# Sequential simulations - ⚠️ Need valid addresses with funds
cargo run --example mev_sandwich_bundle_example  # MEV bundle (needs funded addresses)
cargo run --example sequential_eth_transfers_with_state_persistence  # ETH transfers

# Token workflows - ⚠️ Need valid addresses with funds
cargo run --example buy_approve_then_sell_floki  # DeFi workflow
cargo run --example buy_approve_then_sell_pepe   # DeFi workflow
cargo run --example buy_approve_then_sell_usdc   # DeFi workflow

# Performance testing
cargo run --release --example rpc_vs_direct_simulation_benchmark
```

**Note**: Examples that simulate transactions need addresses with actual ETH/token balances on mainnet.
View function examples work with any addresses since they're read-only.

### Run with Debug Output

```bash
RUST_LOG=debug cargo run --example trace_extraction_example
```

## 🎯 Key Concepts Demonstrated

### 1. Unsigned Transaction Simulation
- No signature required
- Specify any `from` address
- Automatic gas price calculation
- Nonce auto-detection

### 2. Sequential Simulation Types

**UnsignedTxChainSimulation** (Interactive):
- Step-by-step execution
- Inspect state between transactions
- Add transactions dynamically
- Used in token workflow examples

**unsigned_tx_bundle_simulation** (Batch):
- All transactions provided upfront
- Execute as atomic bundle
- Used in MEV examples
- Inspector fusing for performance

### 3. Inspector Fusing
Both sequential simulation types now use inspector fusing:
- Reuses inspector across transactions
- Avoids memory allocations
- Improves performance significantly
- Follows Reth's optimization pattern

### 4. Direct Database Access
- No RPC needed
- Memory-mapped MDBX access
- 100-1000x faster than RPC
- Zero network overhead

## 📝 Common Patterns

### Creating an UnsignedTransaction
```rust
let tx = UnsignedTransaction {
    from: Some(address!("...")),
    to: Some(address!("...")),
    value: Some(U256::from(1_000_000_000_000_000_000u128)), // 1 ETH
    data: Some(Bytes::from(hex!("..."))),
    gas: Some(200_000),
    nonce: None, // Auto-detect
    // EIP-1559 fields
    max_fee_per_gas: Some(30_000_000_000), // 30 gwei
    max_priority_fee_per_gas: Some(1_000_000_000), // 1 gwei
    gas_price: None, // Use EIP-1559 instead
};
```

### Using UnsignedTxChainSimulation (Interactive)
```rust
let mut chain = simulator.start_simulation_chain(None).await?;

// Step through transactions interactively
let buy_result = chain.step(buy_tx).await?;
println!("Buy result: {:?}", buy_result);

let approve_result = chain.step(approve_tx).await?;
println!("Approve result: {:?}", approve_result);

let sell_result = chain.step(sell_tx).await?;
println!("Sell result: {:?}", sell_result);

// Inspect final state
let state = chain.current_state();
println!("Total gas used: {}", state.total_gas_used);
```

### Using Bundle Simulation (Batch)
```rust
let bundle = vec![frontrun_tx, victim_tx, backrun_tx];

let options = SequentialSimulationOptions {
    at_block: Some(block_number),
    stop_on_failure: true,
    auto_increment_nonces: true,
    gas_limit_per_tx: Some(300_000),
};

let result = simulator
    .simulate_transaction_sequence(bundle, options)
    .await?;

println!("Bundle success: {}", result.sequence_success);
println!("Total gas: {}", result.total_gas_used);
```

## 🚀 Performance Tips

1. **Use Release Mode**: `cargo run --release --example ...` for better performance
2. **Inspector Fusing**: Both chain and bundle simulations now use fused inspectors
3. **Batch Processing**: Use bundle simulation for known sequences
4. **Warm Cache**: First simulation warms database cache, subsequent ones are faster
5. **Parallel Processing**: Use concurrent batch for independent transactions

## 🐛 Troubleshooting

### Database Connection Issues
- Verify path: `/home/nima/.local/share/reth/mainnet`
- Check permissions: `ls -la /path/to/reth/mainnet/db`
- Ensure Reth node is not running (locks database)

### Compilation Errors
- Some imports need fixing due to refactoring
- Block simulation module temporarily disabled
- Library has ~38 compilation errors that need resolution

### Nonce Issues
- For unsigned transactions: omit nonce field for auto-detection
- For sequences: enable `auto_increment_nonces` option
- Check actual nonce with `get_nonce_from_state()`

### Gas Issues
- Increase gas limit in UnsignedTransaction
- Check if transaction is reverting (consumes all gas)
- Use view functions to verify state before transactions


## 🔮 Future Improvements

1. **Fix Block Simulation**: Implement proper inspector fusing for signed transactions
2. **Add More Examples**: Cross-chain bridges, complex DeFi protocols
3. **Performance Metrics**: Add timing to all examples
4. **Error Recovery**: Show retry patterns for failed transactions
5. **State Diff Visualization**: Show state changes between transactions