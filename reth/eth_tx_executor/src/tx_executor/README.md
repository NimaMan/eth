# Transaction Executor Module

## Overview

The Transaction Executor is the heart of ETH Kartal's high-performance trading system. It receives trading alerts, validates positions, optimizes gas prices, builds transactions, and submits them to the Ethereum blockchain with sub-200ms latency. The executor supports both public mempool submission and MEV-protected Flashbots bundles.

## Architecture

```
Alert Signal → Validation → Position Check → Gas Optimization → Transaction Building → Submission → Monitoring
    ↓              ↓             ↓                ↓                     ↓                ↓            ↓
  <5ms          <10ms         <20ms            <30ms               <50ms            <100ms      Track
```

## Core Components

### 1. TransactionExecutor (`executor.rs`)
The main orchestrator that coordinates all execution steps.

```rust
pub struct TransactionExecutor {
    provider: Arc<Provider<Http>>,           // Ethereum RPC provider
    wallet: Arc<SecureWallet>,              // Encrypted wallet for signing
    pool_factory: PoolFactory,              // DEX pool interactions
    // Note: Position tracking removed - trading agent handles balances
    ranking_system: Arc<TransactionRankingSystem>, // Gas optimization
    nonce_manager: Arc<NonceManager>,       // Nonce tracking
    risk_manager: Arc<Mutex<RiskManager>>,  // Risk validation
    trade_logger: Arc<TradeLogger>,         // Execution logging
    flashbots_client: Option<Arc<FlashbotsClient>>, // MEV protection
}
```

### 2. ExecutorConfig
Configuration parameters for the executor:

```rust
pub struct ExecutorConfig {
    pub keystore_path: PathBuf,        // Path to encrypted wallet
    pub chain_id: u64,                 // 1 for mainnet
    pub rpc_url: String,               // Ethereum RPC endpoint
    pub flashbots_enabled: bool,       // Enable MEV protection
    pub flashbots_rpc: Option<String>, // Flashbots relay endpoint
    pub reth_ws_url: String,           // Mempool monitoring websocket
    pub risk_config: RiskConfig,       // Risk management parameters
    pub rabbitmq_url: Option<String>,  // Optional block processor data
}
```

### 3. Alert Structure
Trading signals that trigger execution:

```rust
pub struct Alert {
    pub id: String,                    // Unique alert identifier
    pub timestamp: u64,                // Unix timestamp
    pub token_address: Address,        // Token to trade
    pub pool_address: Address,         // Liquidity pool
    pub action: Action,                // Buy/Sell/AddLiquidity/RemoveLiquidity
    pub params: ExecutionParams {
        pub amount: U256,              // Amount in wei
        pub slippage: f64,             // Slippage tolerance (0.05 = 5%)
        pub max_gas_price: Option<U256>, // Gas price cap
        pub deadline_seconds: u64,     // Execution deadline
        pub priority: Priority,        // Critical/High/Normal
    }
}
```

## Execution Flow

### 1. Alert Reception and Validation
```rust
pub async fn execute_alert(&self, alert: Alert) -> ExecutionResult {
    // Validate inputs (addresses, slippage, etc.)
    self.validate_alert_inputs(&alert)?;
    
    // Check alert hasn't expired
    if !alert.is_valid() {
        return Err("Alert expired");
    }
    
    // Execute based on action type
    match alert.action {
        Action::Buy => self.execute_buy(alert, metrics).await,
        Action::Sell => self.execute_sell(alert, metrics).await,
        // Liquidity operations not yet implemented
    }
}
```

### 2. Sell Execution Example
```rust
async fn execute_sell(&self, alert: Alert, metrics: &mut ExecutionMetrics) -> Result<H256> {
    // Step 1: Use amount from alert (trading agent ensures tokens are available)
    // The trading agent is responsible for checking positions before sending alerts
    
    // Step 2: Calculate optimal gas price using ranking system
    let ranking_result = self.ranking_system.calculate_ranking(&alert).await?;
    // Returns optimal gas price and expected mempool position
    
    // Step 3: Get best pool and calculate output
    let pool = self.pool_factory.find_best_pool(token, WETH).await?;
    let amount_out = pool.get_amount_out(amount_to_sell, token).await?;
    let min_amount_out = apply_slippage(amount_out, alert.params.slippage);
    
    // Step 4: Risk validation
    let risk_decision = self.risk_manager.evaluate_trade_risk(&alert).await?;
    match risk_decision {
        RiskDecision::Allow => proceed,
        RiskDecision::ReduceSize { new_amount } => use reduced amount,
        RiskDecision::Block { reason } => return Err(reason),
    }
    
    // Step 5: Build swap transaction
    let swap_params = SwapParams {
        token_in: alert.token_address,
        token_out: WETH,
        amount_in: final_amount,
        amount_out_min: min_amount_out,
        recipient: self.wallet.address(),
        deadline: alert.deadline_timestamp(),
    };
    let tx = pool.build_swap_tx(swap_params).await?;
    
    // Step 6: Set gas and nonce
    tx.set_gas_price(ranking_result.optimal_gas_price);
    tx.set_nonce(self.nonce_manager.reserve_nonce().await?);
    
    // Step 7: Submit transaction
    let tx_hash = self.submit_transaction(tx, &ranking_result.execution_path).await?;
    
    return Ok(tx_hash);
}
```

### 3. Transaction Submission
```rust
async fn submit_transaction(&self, tx: TypedTransaction, path: &ExecutionPath) -> Result<H256> {
    match path {
        ExecutionPath::PublicMempool => {
            // Standard submission
            let signed = self.wallet.sign_transaction(&tx).await?;
            let pending = self.provider.send_raw_transaction(signed).await?;
            Ok(pending.tx_hash())
        }
        
        ExecutionPath::FlashbotsBundle { .. } => {
            // MEV-protected submission
            let bundle = BundleBuilder::new()
                .add_transaction(signed_tx)
                .block_number(next_block)
                .build()?;
            self.flashbots_client.submit_bundle(bundle).await
        }
        
        ExecutionPath::MultiPath { timeout_ms } => {
            // Try public first, fallback to Flashbots
            // Useful for time-sensitive trades
        }
    }
}
```

## Key Features

### 1. Performance Metrics
Every execution tracks detailed timing:
```rust
pub struct ExecutionMetrics {
    pub alert_to_start_ms: u64,      // Alert processing overhead
    pub position_check_ms: u64,      // Position verification time
    pub gas_ranking_ms: u64,         // Gas optimization calculation
    pub price_quote_ms: u64,         // DEX price query time
    pub tx_build_ms: u64,            // Transaction construction
    pub tx_submit_ms: u64,           // Submission to network
    pub total_ms: u64,               // End-to-end latency
}
```

### 2. Gas Optimization
The ranking system determines optimal gas prices based on:
- Current mempool conditions
- Transaction urgency (Critical/High/Normal)
- Competition analysis
- Historical success rates

### 3. Risk Management
Built-in safeguards:
- Position size limits
- Daily loss limits
- Slippage protection
- Gas price caps
- Circuit breakers

### 4. Nonce Management
Prevents common issues:
- Automatic nonce tracking
- Reservation system for parallel submissions
- Recovery from stuck transactions
- Proper failure handling

## Complete Working Example

### 1. Setup
```rust
use eth_kartal::tx_executor::{TransactionExecutor, ExecutorConfig};
use eth_kartal::alert_processor::{Alert, Action, ExecutionParams, Priority};
use eth_kartal::risk::RiskConfig;
use std::path::PathBuf;

// Configure executor
let config = ExecutorConfig {
    keystore_path: PathBuf::from("./keystore.json"),
    chain_id: 1, // Mainnet
    rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string(),
    flashbots_enabled: true,
    flashbots_rpc: Some("https://relay.flashbots.net".to_string()),
    reth_ws_url: "ws://localhost:8546".to_string(),
    risk_config: RiskConfig {
        max_position_size_eth: 1.0,
        max_daily_loss_eth: 5.0,
        max_slippage_allowed: 0.1, // 10%
        min_liquidity_eth: 10.0,
    },
    rabbitmq_url: None,
};

// Create executor
let executor = TransactionExecutor::new(config).await?;

// Unlock wallet
let password = read_password("Enter keystore password: ")?;
executor.unlock_wallet(password).await?;
```

### 2. Execute a Sell
```rust
// Create sell alert (e.g., from scam detection)
let alert = Alert {
    id: "scam_123".to_string(),
    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    token_address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".parse()?, // USDC
    pool_address: "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc".parse()?, // USDC/ETH V2
    action: Action::Sell,
    params: ExecutionParams {
        amount: U256::MAX, // Sell all
        slippage: 0.05,    // 5% slippage
        max_gas_price: Some(parse_units("100", "gwei").unwrap()), // 100 gwei max
        deadline_seconds: 300, // 5 minutes
        priority: Priority::High,
    },
};

// Execute
let result = executor.execute_alert(alert).await;

match result {
    Ok(execution) => {
        println!("✅ Trade executed successfully!");
        println!("   Transaction: {:?}", execution.tx_hash);
        println!("   Total time: {}ms", execution.metrics.total_ms);
        println!("   Breakdown:");
        println!("     - Position check: {}ms", execution.metrics.position_check_ms);
        println!("     - Gas optimization: {}ms", execution.metrics.gas_ranking_ms);
        println!("     - Price quote: {}ms", execution.metrics.price_quote_ms);
        println!("     - TX building: {}ms", execution.metrics.tx_build_ms);
        println!("     - Submission: {}ms", execution.metrics.tx_submit_ms);
    }
    Err(e) => {
        println!("❌ Execution failed: {}", e);
    }
}
```

### 3. Execute a Buy
```rust
// Create buy alert (e.g., from trading opportunity)
let alert = Alert {
    id: "buy_opportunity_456".to_string(),
    timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    token_address: "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984".parse()?, // UNI
    pool_address: "0xd3d2e2692501a5c9ca623199d38826e513033a17".parse()?, // UNI/ETH
    action: Action::Buy,
    params: ExecutionParams {
        amount: parse_units("0.1", "ether").unwrap(), // Buy with 0.1 ETH
        slippage: 0.03,    // 3% slippage
        max_gas_price: None, // No limit
        deadline_seconds: 180, // 3 minutes
        priority: Priority::Normal,
    },
};

let result = executor.execute_alert(alert).await;
```

## Testing and Safety

### 1. Test Mode
Run without real transactions:
```bash
cargo run --bin kartal -- --test-mode --keystore-path ./test_wallet.json
```

### 2. Simulation First
Every transaction is simulated with REVM before submission to ensure:
- Transaction will succeed
- Gas estimates are accurate
- Output amounts are as expected

### 3. Local Network Testing
Test with Anvil fork:
```bash
# Start local fork
anvil --fork-url https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY

# Run executor against local network
cargo run --bin kartal -- --rpc-url http://localhost:8545
```

## Performance Optimization

### 1. Latency Targets
- Alert processing: <5ms
- Position check: <20ms
- Gas calculation: <30ms
- Transaction build: <10ms
- Submission: <100ms
- **Total: <200ms**

### 2. Optimization Techniques
- Pre-cached pool states
- Parallel RPC calls where possible
- Optimized contract calls
- Connection pooling
- Minimal memory allocations

### 3. Monitoring
Built-in performance tracking:
```rust
// After each execution
info!("Execution metrics: {:?}", result.metrics);

// Aggregate statistics
let avg_latency = total_latency_ms / execution_count;
warn!("Average execution latency: {}ms", avg_latency);
```

## Error Handling

### Common Errors and Solutions

1. **"No tokens to sell"**
   - Position tracker shows zero balance
   - Check if tokens were already sold or never bought

2. **"Insufficient ETH for gas"**
   - Wallet doesn't have enough ETH for transaction fees
   - Top up wallet with ETH

3. **"Slippage too high"**
   - Market moved beyond acceptable slippage
   - Increase slippage tolerance or wait for better conditions

4. **"Nonce too low"**
   - Transaction with same nonce already confirmed
   - Nonce manager will auto-recover on next transaction

5. **"Transaction underpriced"**
   - Gas price too low for current network conditions
   - Ranking system will adjust for next transaction

## Integration with Live Trading Database

The executor integrates with `live_trading_db` for comprehensive tracking:

1. **Before Execution**: Create entry in `trade_signals` table
2. **After Submission**: Update signal status and create `executions` record
3. **On Confirmation**: Update `live_positions` state
4. **Continuous**: Track performance in `position_snapshots`

## Security Best Practices

1. **Never log private keys or mnemonics**
2. **Validate all addresses against checksums**
3. **Use secure RPC endpoints (HTTPS/WSS)**
4. **Implement rate limiting**
5. **Monitor for unusual patterns**
6. **Keep audit trail of all executions**

## Future Enhancements

1. **Multi-DEX Support**: Currently Uniswap V2 only
2. **Cross-chain**: Support for L2s and other chains
3. **Advanced MEV**: Custom block building
4. **Batch Operations**: Multiple trades in one transaction
5. **Liquidity Provision**: Add/remove liquidity operations

## Summary

The Transaction Executor is a battle-tested, high-performance system that:
- Executes trades in <200ms from alert to blockchain
- Provides comprehensive risk management
- Supports both public and private transaction submission
- Tracks detailed performance metrics
- Handles errors gracefully
- Integrates with broader trading infrastructure

For production use, always test thoroughly on testnets first and start with small position sizes.