# Transaction Simulator (tx_simulator)

A high-performance Ethereum transaction simulator that provides direct database access to Reth's state, bypassing RPC overhead. This library focuses purely on simulation: signed transactions, unsigned calls (like `debug_traceCall`), sequential transaction processing, and raw CallFrame generation.

## Features

- **Direct Database Access**: Reads directly from Reth's MDBX database files for maximum performance
- **Multiple Simulation Types**: Supports signed transactions, unsigned calls, view functions, and sequential processing
- **Raw CallFrame Generation**: Returns complete call traces for tx_processor to analyze
- **Batch Processing**: Concurrent simulation of thousands of transactions with configurable parallelism
- **No Signatures Required**: Can simulate unsigned transactions for testing and analysis
- **Modular Design**: Clean separation of concerns across different simulation types

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
tx_simulator = { path = "../tx_simulator" }
tokio = { version = "1", features = ["full"] }
eyre = "0.6"
```

### Basic Usage

```rust
use tx_simulator::{TxSimulator, CallRequest};
use alloy_primitives::{Address, U256, Bytes};
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simulator with Reth database path
    let simulator = TxSimulator::new("/home/user/.local/share/reth/mainnet")?;
    
    // Create a call request (unsigned transaction)
    let call = CallRequest {
        from: Some(Address::ZERO),
        to: Some("0xA0b86a33E6C2C76F8c4f8e60b55C2E6F4Fd9a3dB".parse()?),
        value: Some(U256::from(1000000000000000000u64)), // 1 ETH
        data: Some(Bytes::new()),
        gas: Some(21000),
        ..Default::default()
    };
    
    // Simulate the transaction
    let result = simulator.simulate_call(call).await?;
    
    println!("Success: {}", result.success);
    println!("Gas used: {}", result.gas_used);
    
    Ok(())
}
```

## Simulation Types

### 1. Unsigned Transaction Simulation (debug_traceCall equivalent)

Simulate contract calls without signatures:

```rust
use tx_simulator::CallRequest;

// Basic simulation
let result = simulator.simulate_call(call_request).await?;

// Simulation at specific block
let result = simulator.simulate_call_at_block(call_request, 18_500_000).await?;

// Full simulation with CallFrame trace
let full_result = simulator
    .simulate_unsigned_transaction_with_full_trace_at_block(call_request, 18_500_000)
    .await?;

println!("Success: {}", full_result.success);
println!("Gas used: {}", full_result.gas_used);
println!("Call trace depth: {}", full_result.call_trace.calls.len());
```

### 2. Signed Transaction Simulation

Process fully signed transactions with valid signatures:

```rust
use reth_primitives::TransactionSigned;

// Simulate signed transaction from mempool or RPC
let signed_tx: TransactionSigned = // ... get from somewhere
let result = simulator.simulate_signed_transaction(&signed_tx).await?;

// Simulate at specific block
let result = simulator
    .simulate_signed_transaction_at_block(&signed_tx, 18_500_000)
    .await?;
```

### 3. Sequential Transaction Processing

#### Batch Sequence Simulation (All at Once)

Simulate a complete sequence where all transactions are known upfront:

```rust
use tx_simulator::{CallRequest, SequentialSimulationOptions};

let transactions = vec![
    enable_trading_call,
    first_swap_call,
    second_swap_call,
];

let options = SequentialSimulationOptions {
    fail_on_revert: true,
    advance_block_between_txs: false,
};

let result = simulator
    .simulate_transaction_sequence(transactions, options)
    .await?;

for (i, tx_result) in result.results.iter().enumerate() {
    println!("Transaction {}: success={}, gas={}", 
        i, tx_result.success, tx_result.gas_used);
}
```

#### Step-by-Step Simulation (SimulationChain)

For workflows where you need to inspect results between transactions:

```rust
use tx_simulator::SimulationChain;

// Start a simulation chain
let mut chain = simulator.start_simulation_chain(None).await?;

// Step 1: Buy tokens
let buy_result = chain.step(buy_tx).await?;
let tokens_received = extract_tokens(&buy_result);

// Step 2: Approve based on Step 1 results
let approve_tx = create_approve_tx(tokens_received);
let approve_result = chain.step(approve_tx).await?;

// Step 3: Sell tokens
let sell_result = chain.step(sell_tx).await?;

// Inspect state at any point
let state = chain.current_state();
println!("Total gas used: {}", state.total_gas_used);
```

### 4. View Function Calls

Execute view/pure functions that don't modify state:

```rust
use alloy_primitives::Address;

let token_address: Address = "0xA0b86a33E6C2C76F8c4f8e60b55C2E6F4Fd9a3dB".parse()?;

// Get token balance
let balance_selector = [0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
let call_data = simulator.encode_view_function_call_with_address(
    balance_selector, 
    Address::ZERO // account to check
);

let result = simulator.simulate_view_function(
    token_address,
    call_data,
    None // latest block
).await?;

let balance = result.decode_uint256();
println!("Token balance: {}", balance);
```

### 5. Batch Processing

Process many transactions concurrently:

```rust
use tx_simulator::BatchSimulationOptions;
use std::time::Duration;

let transactions: Vec<CallRequest> = // ... your transactions

let options = BatchSimulationOptions {
    max_concurrent: 20,
    timeout_per_tx: Some(Duration::from_millis(100)),
    block_number: Some(18_500_000),
};

let results = simulator.simulate_batch(transactions, options).await?;

println!("Total: {}, Successful: {}, Failed: {}", 
    results.total_transactions,
    results.successful_transactions, 
    results.failed_transactions
);
```

## Data Structures

### CallRequest

```rust
pub struct CallRequest {
    pub from: Option<Address>,           // Sender address
    pub to: Option<Address>,             // Recipient address  
    pub gas: Option<u64>,                // Gas limit
    pub gas_price: Option<u128>,         // Legacy gas price
    pub max_fee_per_gas: Option<u128>,   // EIP-1559 max fee
    pub max_priority_fee_per_gas: Option<u128>, // EIP-1559 priority fee
    pub value: Option<U256>,             // ETH value to send
    pub data: Option<Bytes>,             // Call data
    pub nonce: Option<u64>,              // Transaction nonce
}
```

### SimulationResult

```rust
pub struct SimulationResult {
    pub success: bool,                   // Transaction succeeded
    pub gas_used: u64,                   // Gas consumed
    pub revert_reason: Option<String>,   // Revert message if failed
}
```

### FullSimulationResult

```rust
pub struct FullSimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub call_trace: CallFrame,                      // Raw call trace for tx_processor
}
```

## Performance Characteristics

Based on real benchmarks:

- **Small batches (100 tx)**: ~2.0ms per transaction  
- **Large batches (10K tx)**: ~0.64ms per transaction
- **Database cache warmup**: Performance improves significantly with scale
- **Throughput**: 1,500+ transactions/second for large batches

## Examples

The `examples/` directory contains comprehensive demonstrations:

- `verify_database_setup.rs` - Test database connection and setup verification
- `weth_totalsupply_view_call.rs` - WETH totalSupply() view function call test
- `view_function_example.rs` - View/pure function calls  
- `sequential_simulation_demo.rs` - Sequential transaction processing
- `batch_simulation_demo.rs` - Concurrent batch processing
- `comprehensive_transaction_trace_analysis.rs` - Multi-transaction trace analysis demo
- `sequential_sandwich_attack.rs` - MEV simulation example
- `nonce_adaptation_demo.rs` - Nonce handling demonstration
- `timeout_proof.rs` - Timeout and error handling

Run examples:

```bash
cargo run --example verify_database_setup
cargo run --example weth_totalsupply_view_call  
cargo run --example batch_simulation_demo
```

## Database Setup

The simulator requires access to a Reth database:

1. **Install Reth**: Follow [Reth installation guide](https://github.com/paradigmxyz/reth)
2. **Sync Database**: Run `reth node` to sync mainnet (takes ~2-3 days)
3. **Database Location**: Default at `~/.local/share/reth/mainnet`

The simulator uses read-only access and is safe for concurrent use.

## Error Handling

Common errors and solutions:

- **"No such file or directory"**: Check Reth database path
- **"Permission denied"**: Ensure read access to MDBX files  
- **"Nonce too low"**: Transaction already executed
- **"Out of gas"**: Increase gas limit
- **Timeout**: Increase timeout or reduce complexity

## Performance Tips

1. **Reuse simulator instance** - Database connection is expensive
2. **Batch similar transactions** - Amortizes overhead across operations
3. **Set appropriate timeouts** - Prevent hanging on complex transactions  
4. **Specify block numbers** - Avoids repeated latest block lookups
5. **Use concurrent processing** - Maximize throughput for bulk operations

## Architecture

The library is organized into focused modules:

- `simulator.rs` - Core TxSimulator struct and database management
- `types.rs` - All result types and data structures
- `signed_simulation.rs` - Signed transaction processing
- `unsigned_simulation.rs` - Unsigned call simulation  
- `call_simulator.rs` - CallRequest handling and simulation
- `batch_sequence_simulation.rs` - Sequential transaction processing
- `parallel_tx_simulation.rs` - Concurrent batch processing
- `view_function_simulator.rs` - View/pure function execution

## License

Licensed under the MIT OR Apache-2.0 license.