# Simulate Signed TX Examples

This directory contains comprehensive examples demonstrating various ways to use the `simulate_signed_tx` module for Ethereum transaction simulation and analysis.

## Overview

The examples progress from simple high-level API usage to complex transaction analysis, covering:
- Basic transaction simulation
- Internal transfer extraction  
- Advanced tracing and MEV detection
- DeFi-specific analysis (Uniswap multi-hop swaps)

## Available Examples

### 1. Basic Usage (`basic_usage.rs`)
The simplest example showing how to simulate a transaction by its hash.

```bash
cargo run --bin simulate_basic_usage
```

**Features:**
- Simple transaction simulation  
- Basic result extraction
- Minimal setup required (just hash + RPC URL)
- Automatic fetching and simulation

**Expected Output:**
```
Simulating transaction...

Transaction simulation complete!
- Status: Success(Stop)
- Gas used: 315099
- Gas refunded: 78774
```

### 2. Simulate by Hash (`simulate_by_hash.rs`)
Shows how to fetch and simulate any transaction from the network.

```bash
cargo run --bin simulate_by_hash
# Or with a specific transaction:
cargo run --bin simulate_by_hash -- 0x<your_tx_hash>
```

**Features:**
- Command-line transaction hash input
- Transaction metadata display
- Simulation timing
- Minimal code demonstration

**Expected Output:**
```
Simulating transaction: 0xf7bd...d1ae
Using RPC: http://127.0.0.1:8545

Simulation Results:
==================
Gas Used: 315099
Gas Refunded: 78774
Status: Success(Stop)
Output Data Length: 0

✅ Module successfully simulated the signed transaction!
```

### 3. Simulate Transaction (`simulate_transaction.rs`)
More comprehensive example with result verification.

```bash
cargo run --bin simulate_transaction
# Or with a specific transaction:
cargo run --bin simulate_transaction -- 0x<your_tx_hash>
```

**Features:**
- Detailed result analysis
- Gas usage verification  
- Output data inspection
- Performance timing

**Expected Output:**
```
🚀 Transaction Simulation Example
=================================

📋 Transaction: 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae
🌐 RPC URL: http://127.0.0.1:8545

⏳ Simulating transaction...

✅ Simulation Complete!
⏱️  Time: 25ms

📊 Results:
  • Status: Success(Stop)
  • Gas Used: 315099
  • Gas Refunded: 78774
  • Output Data: 0 bytes
  • Logs: 13

✅ Verification passed!
```

### 4. Extract Internal Transfers (`extract_internal_transfers.rs`)
Demonstrates concepts for extracting internal ETH transfers.

```bash
cargo run --bin extract_internal_transfers
```

**Features:**
- Transaction log analysis
- Internal transfer concepts
- CallTracer usage explanation

### 5. Advanced Tracing (`advanced_tracing.rs`)
Advanced transaction analysis including pattern detection.

```bash
cargo run --bin advanced_tracing
```

**Features:**
- Log event analysis
- ERC20 transfer detection
- Swap event identification
- MEV pattern detection
- Gas efficiency metrics
- Contract interaction statistics

**Expected Output:**
```
🔍 Advanced Transaction Analysis Example
=======================================

📋 Analyzing transaction: 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae
⏳ Simulating transaction...

📊 Simulation Results:
  • Status: Success(Stop)
  • Gas Used: 315099
  • Gas Refunded: 78774 (25.00% of used)

📜 Log Analysis:
  • Total Events: 13
  • Unique Contracts: 5
  • Event Types: 4

🔎 Pattern Detection:
  • ERC20 Transfers: 8
  • DEX Swaps: 1

📊 Event Breakdown:
  • 0x40e9cecb...: 2 occurrences
  • Swap (V3): 1 occurrences
  • Transfer: 8 occurrences
  • Withdrawal: 2 occurrences

🏭 Top Contract Interactions:
  • 0xc02a...6cc2: 5 events
  • 0xdac1...1ec7: 3 events
  • 0x0000...8a90: 2 events

⛽ Gas Efficiency Analysis:
  • Average gas per event: 24238
  • Gas refund ratio: 25.00%

⚡ MEV Indicators:
  • Flash loan usage detected
```

### 6. CallTracer Usage (`call_tracer_usage.rs`)
Educational example explaining CallTracer concepts.

```bash
cargo run --bin call_tracer_usage
```

**Features:**
- CallTracer concept explanation
- Use case demonstrations
- Implementation guidance

**Note:** CallTracer is fully integrated into the simulation. All examples now capture internal transfers automatically during transaction execution without additional RPC calls.

### 7. Uniswap Multi-hop Simulation (`uniswap_multihop_simulation.rs`)
Analyzes complex DeFi transactions with Uniswap interactions.

```bash
cargo run --bin uniswap_multihop_simulation
```

**Features:**
- Uniswap protocol detection (V2, V3, V4)
- Swap routing analysis
- Token flow tracking
- MEV indicator detection
- Multi-hop swap identification

**Expected Output:**
```
🦄 Uniswap Multi-Hop Swap Analysis
==================================

📋 Analyzing transaction: 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae
⏳ Simulating transaction...

📊 Simulation Results:
  • Status: Success(Stop)
  • Gas Used: 315099
  • Total Events: 13

🔍 Uniswap Protocol Detection:
  • Uniswap V3 Swaps: 1

📍 Swap Routing Analysis:
  Total Swaps: 1

  Swap #1:
    Protocol: Uniswap V3
    Pool: 0x11b8...97f6

💰 Token Flow Analysis:
  USDC:
    • Transfer Count: 2
    • Unique Addresses: 1
  WETH:
    • Transfer Count: 3
    • Unique Addresses: 1
  0xdac1...1ec7:
    • Transfer Count: 3
    • Unique Addresses: 1

⚡ MEV Indicators:
  • WETH wrapping/unwrapping detected

📈 Trading Insights:
  • Average gas per swap: 315099
  • Execution efficiency: 25.00%
```

## Prerequisites

1. **Local Reth Node**: All examples expect a local Reth node running at `http://127.0.0.1:8545`
2. **Network Sync**: The node should be synced to at least block 20,000,000 for the default examples

## Common Patterns

### Transaction Hash Used
Most examples use this transaction by default:
```
0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae
```

This is a complex DeFi transaction that demonstrates:
- Multiple contract interactions
- ERC20 transfers
- Uniswap V3 swaps
- Gas refunds

### Error Handling
All examples use proper error handling with `anyhow::Result` and will display clear error messages if:
- The RPC connection fails
- The transaction is not found
- The simulation encounters an error

## Extending the Examples

To create your own example:

1. Create a new file in this directory
2. Add it as a binary in `/Cargo.toml`:
   ```toml
   [[bin]]
   name = "my_example"
   path = "src/simulate_signed_tx/examples/my_example.rs"
   ```
3. Import the simulation function:
   ```rust
   use revm_tx_simulator_lib::simulate_signed_tx::simulate_signed_tx;
   ```
4. Run your example:
   ```bash
   cargo run --bin my_example
   ```

## Running the Examples

### From This Directory
```bash
cd src/simulate_signed_tx/examples
cargo run --bin <example_name>
```

### From Project Root
All examples are configured as binaries in Cargo.toml:
```bash
cargo run --bin <example_name>
```

### With Custom RPC
```bash
RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR-KEY cargo run --bin <example_name>
```

## Module Architecture

The examples demonstrate different layers of the module:

1. **High Level** (`simulate_signed_tx`): Simple API for quick results
2. **Mid Level** (with configuration): Control over simulation parameters  
3. **Low Level** (CallTracer concepts): Understanding inspector integration

## Technical Notes

### Blob Gas Support
All examples handle Cancun hardfork requirements including blob gas fields for blocks >= 19,426,587.

### Performance Benchmarks
Typical performance metrics on local Reth node:

| Transaction Type | Simulation Time | Memory Usage |
|-----------------|-----------------|--------------|
| Simple Transfer | 2-3ms | <1MB |
| ERC20 Transfer | 3-5ms | 1-2MB |
| Uniswap Swap | 5-10ms | 2-5MB |
| Complex DeFi | 10-20ms | 5-10MB |
| Flash Loan | 20-50ms | 10-20MB |

### Memory Usage
Complex transactions may require significant memory for state loading. Ensure adequate system resources.

## Troubleshooting

**Common Issues:**
- **"Transaction not found"**: Ensure node is fully synced to the required block
- **"RPC timeout"**: Use local node or increase timeout settings  
- **"Simulation failed"**: Check if transaction requires specific state or block
- **"Header validation error"**: Usually indicates missing blob gas for Cancun blocks

**Debug Tips:**
- Enable `RUST_LOG=debug` for detailed tracing
- Compare results with Etherscan for validation
- Use local Reth node for best performance (2-5ms vs 100-500ms for remote)

## Integration Examples

### With Analytics Pipeline
```rust
// Feed simulation results to analytics
let result = simulate_signed_tx(hash, rpc).await?;
analytics_pipeline
    .ingest_simulation(result)
    .detect_patterns()
    .store_insights()
    .await?;
```

### With Trading Systems  
```rust
// Pre-trade simulation
let simulation = simulate_signed_tx(proposed_tx_hash, rpc).await?;
if simulation.gas_used < gas_limit && simulation.result_type.is_success() {
    execute_trade(proposed_tx).await?;
}
```

### With Monitoring Systems
```rust
// Real-time transaction monitoring
mempool_stream
    .filter_transactions(criteria)
    .map(|tx| simulate_signed_tx(tx.hash, rpc))
    .filter_map(|result| detect_anomalies(result))
    .for_each(|anomaly| alert_system.notify(anomaly))
    .await;
```

## Additional Analysis Capabilities

### State Change Analysis
While the examples focus on simulation results, you can extend them to analyze state changes:

```rust
// After simulation, extract state changes
let state_summary = extract_state_changes(&simulation_output);
for (address, changes) in state_summary {
    println!("Address {}: balance_change={}, nonce_change={}", 
        address, changes.balance_change, changes.nonce_change);
}
```

### Event Log Decoding
Decode and analyze specific events:

```rust
// Decode known event signatures
const TRANSFER_TOPIC: &str = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
const SWAP_V2_TOPIC: &str = "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
const SWAP_V3_TOPIC: &str = "c42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

for log in &output.logs {
    match log.topics().first() {
        Some(topic) if format!("{:x}", topic) == TRANSFER_TOPIC => decode_transfer(&log),
        Some(topic) if format!("{:x}", topic) == SWAP_V2_TOPIC => decode_swap_v2(&log),
        Some(topic) if format!("{:x}", topic) == SWAP_V3_TOPIC => decode_swap_v3(&log),
        _ => continue,
    }
}
```

### Gas Optimization Analysis
Track gas usage patterns:

```rust
// Analyze gas efficiency
let gas_metrics = GasMetrics {
    total_used: output.gas_used,
    refunded: output.gas_refunded,
    efficiency: (output.gas_refunded as f64 / output.gas_used as f64) * 100.0,
    per_event: output.gas_used / output.logs.len() as u64,
};
```

## Additional Resources

- REVM Documentation: https://github.com/bluealloy/revm
- Ethereum Yellow Paper: For understanding opcodes and gas
- Reth Documentation: https://paradigmxyz.github.io/reth/
- Alloy Documentation: https://alloy-rs.github.io/alloy/
- Example transactions for testing are available in each example file