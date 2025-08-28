# Transaction Simulator Examples

This directory contains comprehensive examples demonstrating various features of the transaction simulator. Each example is self-contained and demonstrates specific functionality with clear inputs and outputs.

## Prerequisites

All examples require a local Reth database. Update the path in each example:
```rust
let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
```

## Examples Overview

### 1. `verify_database_setup.rs` - Comprehensive Database Setup Verification
**Purpose**: Comprehensive test that verifies simulator initialization, database access, base fee calculation, and provider factory access.

**Input**: 
- Reth database path

**Output**:
```
🔧 Basic TX Simulator Functionality Test
✅ Simulator initialized successfully
✅ Latest block retrieved: 21234567
✅ Database access working correctly
✅ Base fee at block 21234467: 25.3 gwei
✅ Provider factory access working
✅ All basic functionality tests passed!
```

**Use Case**: Run this first to ensure your setup is working correctly before using any other examples.

---

### 2. `weth_totalsupply_view_call.rs` - WETH View Function Test
**Purpose**: Tests pure simulation functionality with a view function call to WETH contract.

**Input**:
- WETH contract address (0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2)
- totalSupply() function selector

**Output**:
```
Testing tx_simulator - pure simulation library
Latest block: 18500123
Simulating WETH totalSupply() call...
✅ tx_simulator is working correctly!
This is a clean library focused only on transaction simulation.
```

**Use Case**: Verifying tx_simulator core functionality with read-only contract calls.

---

### 3. `batch_simulation_demo.rs` - Concurrent Batch Processing
**Purpose**: Shows how to simulate multiple transactions concurrently with performance comparison.

**Input**:
- Array of 10 call requests (unsigned transactions)
- Batch configuration (max_concurrent: 5)

**Output**:
```
1. Sequential Processing:
   Call 0 ✓
   Call 1 ✓
   ...
Time: 89ms (8.9ms per transaction)

2. Concurrent Processing (5 parallel):
   Processed 10 calls
   Success rate: 100%
Time: 24ms (2.4ms per transaction)
Speed improvement: 3.7x
```

**Use Case**: High-throughput simulation for mempool analysis or mass testing.

---


### 4. `view_function_example.rs` - Read-Only Contract Calls
**Purpose**: Shows how to call view functions without creating transactions.

**Input**:
- Contract address (e.g., USDC)
- Function selectors for balanceOf, totalSupply, decimals

**Output**:
```
Querying USDC contract...

1. Total Supply:
   Raw: 0x00000000000000000000000000000000000...
   Decoded: 25,123,456,789 USDC

2. Balance of address:
   Balance: 1,000,000 USDC

3. Token decimals: 6
4. Token name: "USD Coin"
5. Token symbol: "USDC"
```

**Use Case**: Reading contract state, checking balances, getting token metadata.

---

### 5. `nonce_adaptation_demo.rs` - Automatic Nonce Management
**Purpose**: Demonstrates how the simulator handles nonce mismatches.

**Input**:
- Transaction with wrong nonce (e.g., nonce: 0)
- Actual account nonce (e.g., 157)

**Output**:
```
Testing with wrong nonce 0...
✅ Simulation succeeded (nonce was automatically adapted!)
  Gas used: 21,000

Testing without nonce (auto-detection)...
✅ Simulation succeeded with auto-detected nonce!
```

**Use Case**: Simulating mempool transactions where nonces might be outdated.

---

### 6. `timeout_proof.rs` - Timeout and Reliability Testing
**Purpose**: Proves that simulations can timeout and handle edge cases.

**Input**:
- Complex recursive contract call
- Timeout setting (100ms)

**Output**:
```
Testing timeout handling...
  Complex call simulation...
  ⏱️ Timed out after 100ms (as expected)

Testing normal transaction...
  ✅ Completed in 2ms
```

**Use Case**: Ensuring the simulator won't hang on complex transactions.

---

### 7. `sequential_simulation_demo.rs` - Sequential Transaction Simulation
**Purpose**: Simulates a sequence of transactions where each builds on previous state changes.

**Input**:
- Array of 3 transactions from same sender
- Sequential simulation options

**Output**:
```
Setting up 3 transactions:
  - Tx 0: Transfer 0.1 ETH to 0xdead...
  - Tx 1: Transfer 0.2 ETH to 0xbeef...
  - Tx 2: Transfer 0.3 ETH to 0xcafe...

Running sequential simulation...
✅ Sequence complete: 3/3 successful
  - Total gas used: 63,000
  - Transaction 0: ✓ (21,000 gas)
  - Transaction 1: ✓ (21,000 gas) [nonce auto-incremented]
  - Transaction 2: ✓ (21,000 gas) [nonce auto-incremented]
```

**Use Case**: MEV bundle simulation, testing protocol interactions, multi-step DeFi operations.

---

### 8. `sequential_sandwich_attack.rs` - Sequential Sandwich Attack Simulation
**Purpose**: Demonstrates sequential simulation of a sandwich attack - a 3-transaction sequence where an MEV bot profits by manipulating DEX prices around a victim's trade.

**Input**:
- Bundle of 3 transactions: frontrun, victim, backrun
- Atomic execution requirement

**Output**:
```
Bundle transactions:
  1. Frontrun: MEV bot buys token (increases price)
  2. Victim: User's swap transaction
  3. Backrun: MEV bot sells token (captures profit)

✅ Bundle simulation complete:
  - All 3 transactions successful
  - Total gas: 450,000
  - Frontrun: ✓ (150,000 gas)
  - Victim: ✓ (150,000 gas)
  - Backrun: ✓ (150,000 gas)
  - Bundle is profitable!
```

**Use Case**: MEV research, bundle validation, understanding sandwich attacks.

---

## Running Examples

### Run a specific example:
```bash
cargo run --example verify_database_setup
cargo run --example weth_totalsupply_view_call
cargo run --example batch_simulation_demo
```

### Run with debug output:
```bash
RUST_LOG=debug cargo run --example comprehensive_transaction_trace_analysis
```

### Run all examples:
```bash
for example in verify_database_setup weth_totalsupply_view_call batch_simulation_demo view_function_example nonce_adaptation_demo timeout_proof sequential_simulation_demo sequential_sandwich_attack comprehensive_transaction_trace_analysis; do
    echo "Running $example..."
    cargo run --example $example
done
```

## Key Concepts Demonstrated

1. **Direct Database Access**: All examples use Reth's local database, no RPC needed
2. **Signed vs Unsigned**: Shows both signed transaction and call request simulation
3. **State Forking**: Each simulation runs in isolated forked state
4. **Performance**: Batch processing shows ~2ms per transaction with concurrency
5. **Call Traces**: Raw CallFrame structure demonstration
6. **Sequential State**: Transactions can build on each other's state changes
7. **Error Handling**: Proper handling of reverts, timeouts, and nonce issues

## Common Patterns

### Creating a CallRequest (Unsigned Transaction)
```rust
let call = CallRequest {
    from: Some(address!("...")),
    to: Some(address!("...")),
    value: Some(U256::from(1_000_000_000_000_000_000u128)), // 1 ETH
    data: Some(Bytes::from(hex!("..."))),
    gas: Some(200_000),
    gas_price: None, // Use EIP-1559 instead
    max_fee_per_gas: Some(30_000_000_000), // 30 gwei
    max_priority_fee_per_gas: Some(1_000_000_000), // 1 gwei
    nonce: None, // Auto-detect from state
};
```

### Simulating at Specific Block
```rust
let block_number = 22_950_000;
let result = simulator.simulate_call_at_block(call, block_number).await?;
```

### Extracting Full Traces
```rust
let result = simulator.simulate_unsigned_transaction_with_trace(call, None).await?;
println!("Call trace has {} child calls", result.call_trace.calls.len());
println!("Logs generated: {}", result.call_trace.logs.len());
// Note: Internal transaction extraction should be done by tx_processor
```

## Performance Considerations

- **Cold Cache**: First simulation ~2ms due to database cache warming
- **Warm Cache**: Subsequent simulations ~0.5-1ms
- **Batch Processing**: Use concurrent batch for >10 transactions
- **Timeout**: Set reasonable timeouts for complex contracts
- **Block Selection**: Older blocks may be slower to access

## Troubleshooting

### "No such file or directory"
- Ensure Reth database path is correct
- Check permissions: `ls -la /path/to/reth/mainnet/db`

### "Nonce too low"
- For unsigned transactions, omit the nonce field
- For signed transactions, ensure nonce matches current state

### "Out of gas"
- Increase gas limit in CallRequest
- Check if transaction is reverting (consumes all gas)

### Timeouts
- Complex contracts may need longer timeouts
- Recursive calls can be gas-intensive