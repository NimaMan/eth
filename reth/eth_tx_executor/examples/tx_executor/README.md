# Transaction Executor Examples

This directory contains examples demonstrating how to use ETH Kartal's high-performance transaction executor for automated trading on Ethereum.

## Examples

### 1. `basic_executor_demo.rs` - Basic Trading Operations
A complete example showing how to execute buy and sell trades with the transaction executor.

**Features:**
- Command-line interface for easy testing
- Support for common tokens (USDC, USDT, DAI, UNI, LINK)
- Dry-run mode for safe testing
- Configurable slippage and gas limits
- Performance metrics display

**Usage:**
```bash
# Show help
cargo run --example basic_executor_demo -- --help

# Dry run - simulate selling 100 USDC
cargo run --example basic_executor_demo -- --dry-run --action sell --token USDC --amount 100

# Dry run - simulate buying UNI with 0.1 ETH
cargo run --example basic_executor_demo -- --dry-run --action buy --token UNI --eth-amount 0.1

# Real execution (BE CAREFUL - uses real funds!)
cargo run --example basic_executor_demo -- \
    --keystore ./keystore.json \
    --rpc-url https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY \
    --action buy \
    --token LINK \
    --eth-amount 0.05 \
    --slippage 5.0 \
    --priority high \
    --flashbots
```

**Command-line Options:**
- `--keystore`: Path to your encrypted wallet file
- `--rpc-url`: Ethereum RPC endpoint
- `--action`: `buy` or `sell`
- `--token`: Token to trade (USDC, USDT, DAI, UNI, LINK)
- `--amount`: Amount of tokens to sell (for sell action)
- `--eth-amount`: Amount of ETH to spend (for buy action)
- `--slippage`: Slippage tolerance in percent (default: 3%)
- `--priority`: Transaction priority (critical, high, normal)
- `--flashbots`: Enable MEV protection via Flashbots
- `--dry-run`: Simulate without executing
- `--max-gas-gwei`: Maximum gas price in gwei

### 2. `advanced_executor_demo.rs` - Advanced Features
Demonstrates advanced capabilities including MEV protection, concurrent executions, and performance monitoring.

**Features:**
- MEV protection for high-value trades
- Priority-based execution strategies
- Stress testing with concurrent trades
- Transaction confirmation monitoring
- Detailed performance analytics

**Scenarios:**
1. **MEV-Protected Sell**: Large trade using Flashbots to prevent sandwich attacks
2. **Priority Buys**: Demonstrates different priority levels and gas optimization
3. **Stress Test**: Rapid execution of multiple trades to test performance

**Usage:**
```bash
# Run all advanced scenarios
cargo run --example advanced_executor_demo

# The demo will execute:
# - MEV-protected high-value sell
# - Multiple buys with different priorities
# - Stress test with 10 rapid trades
```

## Safety Guidelines

### Testing Safely

1. **Always start with dry-run mode**:
   ```bash
   cargo run --example basic_executor_demo -- --dry-run --action sell --token USDC --amount 100
   ```

2. **Use a local fork for testing**:
   ```bash
   # Start Anvil with mainnet fork
   anvil --fork-url https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY
   
   # Run against local fork
   cargo run --example basic_executor_demo -- --rpc-url http://localhost:8545 ...
   ```

3. **Test with small amounts first**:
   - Start with 0.001 ETH for buys
   - Use test tokens on testnets

### Production Considerations

1. **Secure your keystore**:
   - Never commit keystore files to git
   - Use hardware wallets for large amounts
   - Rotate keys regularly

2. **Monitor gas prices**:
   - Set reasonable `--max-gas-gwei` limits
   - Use priority settings appropriately
   - Enable Flashbots for high-value trades

3. **Handle errors gracefully**:
   - Check balance before trading
   - Implement retry logic for failures
   - Monitor nonce issues

## Performance Expectations

### Latency Targets
- **Critical Priority**: < 150ms total execution
- **High Priority**: < 200ms total execution  
- **Normal Priority**: < 300ms total execution

### Optimization Tips
1. Use local RPC nodes for lowest latency
2. Enable Flashbots for trades > 1 ETH
3. Pre-approve tokens to save gas
4. Monitor mempool congestion

## Common Issues and Solutions

### "No tokens to sell"
- Ensure you have a balance of the token
- Check the token address is correct
- Verify the token decimals

### "Insufficient ETH for gas"
- Top up your wallet with ETH
- Reduce position size
- Lower gas price limits

### "Transaction underpriced"
- Increase priority level
- Raise max gas price
- Check network congestion

### "Nonce too low"
- Wait for pending transactions
- Clear stuck transactions
- Restart with fresh nonce

## Integration Example

Here's how to integrate the executor into your own code:

```rust
use eth_kartal::{
    alert_processor::{Alert, Action, ExecutionParams, Priority},
    tx_executor::{TransactionExecutor, ExecutorConfig},
    risk::RiskConfig,
};

async fn my_trading_bot() -> Result<(), Box<dyn std::error::Error>> {
    // Configure executor
    let config = ExecutorConfig {
        keystore_path: "./my-wallet.json".into(),
        chain_id: 1,
        rpc_url: "https://eth-mainnet.g.alchemy.com/v2/KEY".to_string(),
        flashbots_enabled: true,
        // ... other config
    };
    
    // Create executor
    let executor = TransactionExecutor::new(config).await?;
    executor.unlock_wallet(password).await?;
    
    // Create trading alert
    let alert = Alert {
        id: "my_trade_001".to_string(),
        timestamp: current_timestamp(),
        token_address: USDC_ADDRESS.parse()?,
        pool_address: USDC_ETH_POOL.parse()?,
        action: Action::Sell,
        params: ExecutionParams {
            amount: parse_units("1000", 6)?, // 1000 USDC
            slippage: 0.02, // 2%
            max_gas_price: Some(parse_units("100", "gwei")?),
            deadline_seconds: 300,
            priority: Priority::High,
        },
    };
    
    // Execute trade
    let result = executor.execute_alert(alert).await?;
    println!("Trade executed: {:?}", result.tx_hash);
    println!("Total time: {}ms", result.metrics.total_ms);
    
    Ok(())
}
```

## Additional Resources

- [Transaction Executor Module Documentation](../../src/tx_executor/README.md)
- [ETH Kartal Main Documentation](../../README.md)
- [Risk Management Guide](../../src/risk/README.md)
- [Wallet Security Guide](../../src/wallet/README.md)

## Support

For issues or questions:
1. Check the error messages and solutions above
2. Review the main documentation
3. Enable debug logging: `RUST_LOG=eth_kartal=debug`
4. Open an issue with details and logs