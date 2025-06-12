# Transaction Simulation Module

This module provides high-performance Ethereum transaction simulation using REVM and a local Ethereum node. It enables accurate replay of confirmed transactions, extraction of state changes, and analysis of execution results.

## Overview

The module offers a clean API for simulating transactions by their hash or raw bytes, leveraging:
- **REVM**: The Rust Ethereum Virtual Machine for fast, accurate execution
- **AlloyDB**: Lazy state loading from your local node (e.g., Reth)
- **CacheDB**: In-memory state modifications during simulation

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Transaction   │────►│   simulate_signed_tx │────►│  Simulation    │
│   Hash/Bytes    │     │   (Main API)        │     │  Output        │
└─────────────────┘     └─────────┬──────────┘     └─────────────────┘
                                  │
                    ┌─────────────▼──────────────┐
                    │     REVM Execution         │
                    │  ┌────────┐  ┌──────────┐ │
                    │  │CacheDB │  │ AlloyDB  │ │
                    │  │(Memory)│◄─┤(RPC Fork)│ │
                    │  └────────┘  └──────────┘ │
                    └────────────────────────────┘
                                  │
                    ┌─────────────▼──────────────┐
                    │   Local Ethereum Node      │
                    │   (http://127.0.0.1:8545)  │
                    └────────────────────────────┘
```

## Core Components

### Main Entry Points (`lib.rs`)

```rust
/// Simulate a transaction by its hash
pub async fn simulate_signed_tx(
    tx_hash: H256,
    rpc_url: &str,
) -> Result<SimulationOutput>

/// Simulate from raw signed transaction bytes
pub async fn simulate_signed_tx_bytes(
    signed_tx_bytes: &[u8],
    block_number: u64,
    rpc_url: &str,
) -> Result<SimulationOutput>
```

### Simulation Output (`simulation_core.rs`)

```rust
pub struct SimulationOutput {
    pub result_type: ExecutionResultType,  // Success/Revert/Halt
    pub gas_used: u64,                     // Gas consumed
    pub gas_refunded: u64,                 // Gas refunded
    pub logs: Vec<RevmLog>,                // Event logs emitted
    pub output_data: RevmBytes,            // Return data
    pub internal_transfers: Vec<InternalTransfer>,  // Internal ETH transfers
}

pub enum ExecutionResultType {
    Success(SuccessReason),
    Revert,
    Halt(HaltReason),
}
```

### Additional Components

- **CallTracer** (`call_tracer.rs`): REVM Inspector that tracks internal ETH transfers during simulation
- **InternalTransferTracker** (`internal_transfer_tracker.rs`): Utilities for integrating internal transfers into state changes
- **SpecUtils** (`spec_utils.rs`): Determine hardfork rules based on block number

**Internal Transfers:** The simulation automatically captures all internal ETH transfers during execution using the integrated CallTracer. No additional RPC calls are needed - everything is captured in a single simulation pass.

## Usage Examples

### Basic Transaction Simulation

```rust
use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;
use ethers_core::types::H256;

#[tokio::main]
async fn main() -> Result<()> {
    let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
        .parse::<H256>()?;
    let rpc_url = "http://127.0.0.1:8545";
    
    let output = simulate_signed_tx(tx_hash, rpc_url).await?;
    
    println!("Gas used: {}", output.gas_used);
    println!("Status: {:?}", output.result_type);
    println!("Logs: {}", output.logs.len());
    
    Ok(())
}
```

### Analyzing Event Logs

```rust
// Parse ERC20 Transfer events from simulation output
const TRANSFER_TOPIC: &str = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

for log in &output.logs {
    if let Some(topic) = log.topics().first() {
        if format!("{:x}", topic) == TRANSFER_TOPIC {
            // This is an ERC20 transfer
            let from = Address::from_slice(&log.topics()[1][12..]);
            let to = Address::from_slice(&log.topics()[2][12..]);
            let amount = U256::from_be_bytes(&log.data);
            println!("Transfer: {} -> {}, amount: {}", from, to, amount);
        }
    }
}
```

## State Diff Extraction

The module integrates with the `process_tx` module for detailed state change analysis:

```rust
use revm_tx_simulator_lib::{
    simulate_signed_tx::simulate_signed_tx,
    generate_calculated_account_changes,
};

// Simulate transaction
let output = simulate_signed_tx(tx_hash, rpc_url).await?;

// Extract state changes (requires access to the final database state)
// This functionality is available through the process_tx module
```

## Features

### ✅ Implemented
- Transaction simulation by hash or raw bytes
- Accurate gas calculation and refunds
- Event log extraction
- Support for all Ethereum hardforks (auto-detected)
- Blob gas support for Cancun+ blocks
- CallTracer fully integrated - captures internal transfers automatically during simulation
- Comprehensive test suite
- Multiple working examples

### 🚧 Planned Enhancements
- Direct state diff extraction API
- Mempool transaction simulation
- Batch simulation support
- Performance optimizations for high-throughput

## Technical Details

### Database Architecture
1. **AlloyDB**: Connects to your local node and fetches state on-demand
2. **CacheDB**: Wraps AlloyDB, caching reads and storing all modifications
3. **Fork Point**: State is forked from `block_number - 1` for accurate simulation

### Hardfork Support
The module automatically detects and applies the correct EVM rules based on block number:
- Frontier (0)
- Homestead (1,150,000)
- Byzantium (4,370,000)
- Constantinople/Petersburg (7,280,000)
- Istanbul (9,069,000)
- Berlin (12,244,000)
- London (12,965,000)
- Paris/Merge (15,537,394)
- Shanghai (17,034,870)
- Cancun (19,426,587+)

### Blob Gas Handling
For Cancun+ blocks, blob gas fields are automatically configured:
```rust
if block_number >= 19_426_587 {
    block_env.blob_excess_gas_and_price = Some(BlobExcessGasAndPrice {
        excess_blob_gas: 0,
        blob_gasprice: 1,
    });
}
```

## Examples

See the `examples/` directory for comprehensive examples:
- `basic_usage.rs` - Simplest usage
- `simulate_by_hash.rs` - CLI tool for any transaction
- `simulate_transaction.rs` - Detailed result analysis
- `extract_internal_transfers.rs` - Internal ETH transfer concepts
- `advanced_tracing.rs` - Pattern detection and MEV analysis
- `call_tracer_usage.rs` - Understanding CallTracer
- `uniswap_multihop_simulation.rs` - DeFi transaction analysis

Run any example:
```bash
cargo run --bin simulate_by_hash -- 0x<your_tx_hash>
```

## Dependencies

### Required
- Local Ethereum node with RPC enabled (e.g., Reth)
- REVM v25.0.0
- Alloy provider and types
- Ethers for transaction/block fetching
- Tokio for async runtime

### Cargo.toml
```toml
[dependencies]
revm = { version = "25.0.0", features = ["std", "alloydb", "asyncdb"] }
revm-primitives = "19.2.0"
revm-context = "6.0.0"
revm-database = "5.0.0"
alloy-provider = { version = "1.0.6", features = ["reqwest", "ws"] }
alloy-network = "1.0.6"
ethers-core = "2.0.14"
ethers-providers = "2.0.14"
tokio = { version = "1.44", features = ["rt-multi-thread", "macros"] }
anyhow = "1.0"
```

## Testing

Run the test suite:
```bash
cargo test -p revm_tx_simulator
```

Key test files:
- `simulation_tests.rs` - Core simulation logic
- `real_transaction_tests.rs` - Tests with actual mainnet transactions
- `example_tests.rs` - Validates all examples compile and run

## Performance

Typical performance on local Reth node:
- Simple transfer: 2-3ms
- ERC20 transfer: 3-5ms  
- Complex DeFi tx: 10-20ms
- State loading: Lazy (on-demand via RPC)

## Troubleshooting

### Common Issues

**"Transaction not found"**
- Ensure your node is synced past the transaction's block
- Verify the transaction hash is correct

**"Header validation error: excess_blob_gas not set"**
- This is handled automatically for Cancun+ blocks
- If persisting, check your REVM version

**Performance Issues**
- Use a local node for best performance
- Remote nodes add 100-500ms latency per state fetch

### Debug Tips
- Enable `RUST_LOG=debug` for detailed tracing
- Compare results with Etherscan
- Test with known transactions first

## Contributing

When adding features:
1. Update the appropriate module file
2. Add tests in the `tests/` directory
3. Create an example if it demonstrates new functionality
4. Update this README

## License

See the project root for license information.