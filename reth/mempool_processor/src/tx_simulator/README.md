# Transaction Simulator Module

## Overview

The transaction simulator module provides high-performance transaction simulation for mempool analysis. It uses the node's `debug_traceCall` RPC method to simulate pending transactions and extract state changes for signal detection.

**Key Features:**
- Fast simulation (~5ms per transaction) using debug_traceCall
- Extracts ETH and ERC20 token transfers from trace logs
- Implements WETH=ETH logic for accurate state tracking
- Returns state changes in a standardized format for signal detection

## Current Implementation

The module consists of three main components:

1. **`debug_tracecall_simulator.rs`** - Main simulator using debug_traceCall RPC
2. **`debug_tracecall_state_diff_calculator.rs`** - Calculates state changes with WETH=ETH logic
3. **`state_diff_types.rs`** - Type definitions for state changes

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Transaction Simulator Module                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────┐      ┌───────────────────────┐         │
│  │ Mempool Service    │─────▶│ debug_traceCall       │         │
│  │                    │      │ Simulator             │         │
│  └────────────────────┘      │ (~5ms per tx)         │         │
│                              └───────────┬───────────┘         │
│                                          │                      │
│                                          ▼                      │
│                              ┌───────────────────────┐         │
│                              │ Extract from Trace:   │         │
│                              │ - ETH transfers       │         │
│                              │ - ERC20 transfers     │         │
│                              └───────────┬───────────┘         │
│                                          │                      │
│                                          ▼                      │
│                              ┌─────────────────────────┐       │
│                              │ State Diff Calculator   │       │
│                              │ - WETH=ETH logic       │       │
│                              │ - Token aggregation    │       │
│                              └───────────┬─────────────┘       │
│                                          │                      │
│                                          ▼                      │
│                              ┌───────────────────────┐         │
│                              │ CalculatedAccountChanges│        │
│                              │ - Address changes      │         │
│                              │ - ETH net change       │         │
│                              │ - Token net changes    │         │
│                              └───────────────────────┘         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## How It Works

### 1. Transaction Simulation
The `DebugTraceCallSimulator` receives a pending transaction and uses the node's `debug_traceCall` RPC method with the `callTracer` to simulate execution:

```rust
let trace_result = provider.request(
    "debug_traceCall",
    (call_request, "latest", json!({"tracer": "callTracer", "tracerConfig": {"withLog": true}}))
).await?;
```

### 2. Transfer Extraction
From the trace result, the simulator extracts:
- **ETH transfers**: From value transfers in calls
- **ERC20 transfers**: From Transfer event logs (topic `0xddf252ad...`)

### 3. State Change Calculation
The `DebugTraceCallStateDiffCalculator` processes these transfers:
- Aggregates transfers by address
- Applies WETH=ETH logic (WETH transfers are treated as ETH)
- Calculates net changes per address
- Returns `CalculatedAccountChanges` compatible with the signal detection system

## Usage Example

```rust
use mempool_processor::tx_simulator::DebugTraceCallSimulator;
use mempool_processor::mempool_fetcher::types::TransactionView;
use revm_context::BlockEnv;

// Initialize simulator
let simulator = DebugTraceCallSimulator::new("http://127.0.0.1:8545").await?;

// Convert transaction to TransactionView
let tx_view = TransactionView::from_ethers_tx(&pending_tx);

// Create block environment
let block_env = BlockEnv::default();

// Simulate transaction
let state_changes = simulator.process_transaction(&tx_view, &block_env).await?;

if let Some(changes) = state_changes {
    for (address, account_changes) in changes {
        println!("Address: {:?}", address);
        println!("  ETH change: {:?}", account_changes.eth_net_change);
        for (token, amount) in &account_changes.token_net_changes {
            println!("  Token {:?} change: {:?}", token, amount);
        }
    }
}
```

## Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| **Average Latency** | 5-7ms | Using local node |
| **P95 Latency** | 10ms | Network dependent |
| **P99 Latency** | 15ms | Complex transactions |
| **Throughput** | 150-200 TPS | Single connection |
| **Memory Usage** | ~50MB | Minimal state |
| **CPU Usage** | Low | Mostly I/O bound |
| **RPC Calls** | 1 per tx | debug_traceCall |

## Simulation Flow

### Transaction Processing Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Mempool Transaction                             │
└────────────────────────────────┬───────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    Convert to TransactionView                            │
│  - Ethers::Transaction → TransactionView                                │
│  - Extract: from, to, value, data, gas, nonce                          │
└────────────────────────────────┬───────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                     debug_traceCall Simulator                            │
│  1. Prepare call request with transaction details                       │
│  2. Call debug_traceCall with callTracer + withLog                     │
│  3. Parse trace result for logs and internal calls                     │
└────────────────────────────────┬───────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      Extract Transfers from Trace                        │
│                                                                          │
│  1. Parse ERC20 Transfer events (topic 0xddf252ad...)                  │
│  2. Extract ETH transfers from value fields                            │
│  3. Recursively process internal calls                                  │
└────────────────────────────────┬───────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                  State Diff Calculator (WETH=ETH)                        │
│                                                                          │
│  1. Process ETH transfers (including WETH as ETH)                      │
│  2. Process token transfers by contract address                        │
│  3. Aggregate by address with net changes                              │
│  4. Apply known token symbols and decimals                             │
└────────────────────────────────┬───────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Return CalculatedAccountChanges                   │
│                                                                          │
│  HashMap<Address, CalculatedAccountChanges> {                          │
│    address: RevmAddress,                                                │
│    eth_net_change: SignedAmount,                                       │
│    token_net_changes: HashMap<Address, SignedAmount>,                  │
│    movements: AccountMovements                                          │
│  }                                                                       │
└─────────────────────────────────────────────────────────────────────────┘
```

## Key Components

### `debug_tracecall_simulator.rs`
- Main simulator using debug_traceCall RPC method
- Extracts ETH and ERC20 transfers from trace logs
- Returns state changes in CalculatedAccountChanges format
- ~5ms per transaction performance

### `debug_tracecall_state_diff_calculator.rs`
- Processes raw transfers into net state changes
- Implements WETH=ETH logic (WETH transfers counted as ETH)
- Aggregates movements by address and token
- Applies known token symbols and decimal conversions

### `state_diff_types.rs`
- Type definitions for state tracking
- StateDiffTracker, StateChange, MempoolStateDiff types
- Used for compatibility with broader system

## Special Features

### WETH=ETH Logic

The state diff calculator includes special handling for WETH (Wrapped ETH) transfers:

```rust
// WETH transfers are treated as ETH movements
if token_address == WETH_ADDRESS {
    // Track as ETH movement, not token movement
    track_movement(MovementType::Denom, from, to, amount);
}
```

This is important because:
- WETH deposits/withdrawals affect ETH balances
- Many DeFi protocols use WETH interchangeably with ETH
- Accurate state tracking requires unified ETH/WETH accounting

### Known Token Support

The calculator automatically recognizes common tokens and applies proper decimal conversion:
- Stablecoins: USDC (6 decimals), USDT (6 decimals), DAI (18 decimals)
- Wrapped tokens: WETH, WBTC (8 decimals)
- Other major tokens: UNI, AAVE, LINK
- Liquid staking: stETH, cbETH, rETH

## Prerequisites

- Ethereum RPC node with debug API enabled
- Rust 1.70+
- Access to mempool transactions

## Configuration

The simulator requires an RPC endpoint with debug API access:

```rust
let rpc_url = "http://127.0.0.1:8545";
let simulator = DebugTraceCallSimulator::new(rpc_url).await?;
```

For best performance:
- Use a local node (reduces network latency)
- Ensure debug API is enabled on your node
- Consider connection pooling for high throughput

## Testing

### Running Tests
```bash
# Run tx_simulator tests
cargo test tx_simulator

# Run with output
cargo test tx_simulator -- --nocapture
```

### Examples

The module includes example code in the `examples/` directory demonstrating:
- Basic transaction simulation
- State change analysis
- Performance testing

## Troubleshooting

### Common Issues

#### "debug_traceCall not available"
Enable debug API on your node:
```bash
# Geth
geth --http.api eth,net,web3,debug

# Reth
reth node --http.api eth,net,web3,debug
```

#### Simulation timeout
Increase timeout or check node performance.

#### RPC connection errors
Verify endpoint is accessible:
```bash
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  http://127.0.0.1:8545
```

## Summary

The transaction simulator module provides fast, reliable transaction simulation for mempool analysis:
- Uses debug_traceCall for ~5ms simulation time
- Extracts all ETH and token transfers from execution trace
- Implements WETH=ETH logic for accurate state tracking
- Returns standardized format for signal detection system

The module is production-ready and optimized for high-throughput mempool monitoring.