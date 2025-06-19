# Transaction Executor Module

## Overview
The Transaction Executor module handles the construction, simulation, submission, and monitoring of protective transactions. It interfaces with Ethereum nodes and DEX routers to execute trades with optimal speed and reliability.

## Components

### `builder.rs`
- Constructs transaction calldata for various DEX protocols
- Supports Uniswap V2, V3, and other AMMs
- Handles token approvals and multi-hop swaps
- Optimizes transaction parameters for speed

### `simulator.rs`
- Uses REVM to simulate transactions before submission
- Validates transaction will succeed on-chain
- Estimates accurate gas requirements
- Calculates expected outputs and slippage

### `submitter.rs`
- Submits transactions to multiple RPC endpoints
- Implements MEV protection via Flashbots
- Manages nonce and gas price optimization
- Handles transaction replacement and cancellation

### `monitor.rs`
- Tracks submitted transaction status
- Detects transaction inclusion/rejection
- Handles resubmission if needed
- Reports final execution results

## Transaction Flow
```
Strategy Selected → Build Transaction → Simulate → Submit → Monitor → Report
```

## Supported Operations

### 1. Token Swap (Sell)
```rust
pub fn build_swap_exact_tokens_for_eth(
    token_address: Address,
    amount_in: U256,
    amount_out_min: U256,
    deadline: U256,
) -> TransactionRequest
```

### 2. Emergency Liquidation
```rust
pub fn build_emergency_sell_all(
    token_address: Address,
    balance: U256,
    accept_any_price: bool,
) -> TransactionRequest
```

### 3. Multi-Route Swap
```rust
pub fn build_multi_route_swap(
    routes: Vec<SwapRoute>,
    amount_in: U256,
    min_total_out: U256,
) -> Vec<TransactionRequest>
```

## Gas Optimization

### Dynamic Gas Pricing
```rust
pub struct GasStrategy {
    pub base_fee: U256,
    pub priority_fee: U256,
    pub max_fee: U256,
    pub escalation_rate: f64,
}
```

### Priority Levels
- **Critical**: 3x base gas, immediate execution
- **High**: 2x base gas, fast execution
- **Normal**: 1.2x base gas, standard execution

## RPC Configuration

### Endpoint Priority
1. Local Reth node (lowest latency)
2. Flashbots Protect (MEV protection)
3. Infura/Alchemy (fallback)
4. Public endpoints (last resort)

### Submission Strategy
- Send to multiple endpoints simultaneously
- Use first successful submission
- Cancel pending submissions on other endpoints

## Simulation Features

### Pre-flight Checks
- Token balance verification
- Approval status check
- Liquidity availability
- Slippage calculation
- Gas estimation

### Failure Handling
- Reverted transactions logged with reason
- Insufficient balance triggers position check
- High slippage may trigger strategy change

## Usage Example
```rust
use eth_kartal::tx_executor::{TransactionExecutor, SwapParams};

let executor = TransactionExecutor::new(config);

// Build swap transaction
let swap_params = SwapParams {
    token_in: token_address,
    amount_in: balance,
    token_out: WETH,
    min_amount_out: calculate_min_output(balance, max_slippage),
    recipient: wallet_address,
    deadline: current_timestamp() + 300, // 5 minutes
};

// Simulate first
let simulation = executor.simulate_swap(&swap_params).await?;
println!("Expected output: {} ETH", simulation.amount_out);

// Execute if profitable
if simulation.is_profitable_after_gas() {
    let tx_hash = executor.execute_swap(swap_params).await?;
    
    // Monitor execution
    let receipt = executor.wait_for_confirmation(tx_hash).await?;
    println!("Transaction confirmed in block {}", receipt.block_number);
}
```

## Integration Points
- **Input**: Strategy parameters from strategy engine
- **Output**: Transaction hash and execution status
- **Dependencies**: Web3 providers, DEX router ABIs, wallet manager

## Security Considerations
- Never expose private keys in logs
- Validate all addresses against whitelist
- Check for unusual gas consumption
- Verify output amounts match expectations
- Use secure RPC connections (HTTPS/WSS)

## Performance Metrics
- Transaction building: <10ms
- Simulation: <50ms
- Submission: <100ms
- Confirmation: Variable (block time)
- Total latency target: <200ms to submission

## Error Recovery
- Automatic retry on network errors
- Nonce management for stuck transactions
- Gas price escalation for slow confirmation
- Circuit breaker integration for repeated failures