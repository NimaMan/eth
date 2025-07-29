# Token Parameter Extraction Examples

This directory contains examples for analyzing token parameters, particularly buy/sell taxes and honeypot detection using the mempool processor's simulation capabilities.

## Examples Overview

### 1. analyze_token_with_detectors.rs

**Purpose**: Comprehensive token analysis using signal detectors. Simulates buy/sell sequences and runs various detectors to identify token characteristics and potential risks.

**Usage**:
```bash
cargo run --example analyze_token_with_detectors -- <token_address> <pool_address> [block_number]
```

**Inputs**:
- `token_address`: The token contract address to analyze
- `pool_address`: The liquidity pool address for the token
- `block_number` (optional): Specific block to analyze at (defaults to latest)

**Outputs**:
- Log file at `/home/nima/code/crypto/logs/mempool/dev/token_parameter_extraction/token_analysis_[token]_[timestamp].log`
- Console output showing progress and key findings

**What it analyzes**:
1. **Token Information**: Name, symbol, decimals, total supply
2. **Buy/Sell Simulation**: 
   - Simulates buying tokens with 0.1 ETH
   - Attempts to sell tokens back
   - Calculates actual taxes from state changes
3. **Signal Detection**:
   - Trading status (enabled/restricted)
   - Honeypot detection (can't sell after buying)
   - Buy/sell tax thresholds
4. **State Change Analysis**:
   - Detailed ETH and token movements
   - Gas usage for each transaction
   - Revert reasons if transactions fail

### 2. analyze_tax_for_block_range.rs

**Purpose**: Analyzes how a token's buy/sell taxes change over a range of blocks. Useful for tracking tax modifications over time.

**Usage**:
```bash
cargo run --example analyze_tax_for_block_range -- <token_address> <pool_address> <start_block> <end_block>

# Example usage:
cargo run --example analyze_tax_for_block_range -- 0x354ee0074cd5538a1dd1fda7a13e517e11b02699 0xa1eb81db04d93b210ba934a31d794eea76007b20 23002159 23002359
```

**Inputs**:
- `token_address`: The token contract address
- `pool_address`: The liquidity pool address
- `start_block`: First block to analyze
- `end_block`: Last block to analyze

**Outputs**:
- Log file at `/home/nima/code/crypto/logs/mempool/dev/token_parameter_extraction/token_tax_[token]_[timestamp].log`
- Console output showing progress for each block

## How It Works

1. Fetches token info (name, symbol, decimals, total supply) from the blockchain
2. Simulates a 0.1 ETH buy transaction through Uniswap V2 router
3. Simulates selling 1% of the received tokens back
4. Calculates tax rates from the differences in expected vs actual amounts
5. Logs all results to `/home/nima/code/crypto/logs/test/`

## Tax Calculation Formula

### Buy Tax
When a user buys tokens with ETH:
```
Buy Tax % = (1 - tokens_received_by_buyer / tokens_sent_by_pool) × 100
```

### Sell Tax
When a user sells tokens for ETH:
```
Sell Tax % = (1 - eth_received_by_seller / eth_sent_by_pool) × 100
```

## State Changes

The transaction simulator provides a HashMap of address state changes:
```rust
state_changes: HashMap<Address, AddressStateChange>

AddressStateChange {
    token_net: HashMap<String, f64>,  // token changes
    eth_net: f64,                     // ETH changes
}
```

- **Pool Changes**: When tokens/ETH leave the pool, the change is negative
- **Recipient Changes**: When addresses receive tokens/ETH, the change is positive
- **Tax Calculation**: The difference between what left the pool and what the trader received is the tax

## Important Notes

⚠️ **THESE EXAMPLES ARE FOR EDUCATIONAL PURPOSES ONLY**

### Known Limitations

1. **Hardcoded Values**: Examples use hardcoded addresses and paths
2. **Simplified ABI Encoding**: Production code should use proper ABI libraries
3. **Floating Point Math**: Production should use fixed-point arithmetic
4. **No Error Recovery**: Examples don't handle network failures gracefully

### Production Considerations

Before using in production:
- Replace hardcoded values with configuration
- Use proper ABI encoding libraries (ethers-rs)
- Implement fixed-point math for precision
- Add comprehensive error handling
- Implement timeouts and retries
- Add input validation
- Use environment-specific configurations

## Dependencies

These examples require:
- `mempool_processor` crate with tax calculation module
- `reth_tx_simulator` for transaction simulation
- Running Reth node with synced database
- Access to historical blockchain state

## Common Issues

1. **"failed to watch path" error**: Too many file watchers, increase system limits
2. **Simulation failures**: Ensure Reth node is fully synced
3. **Zero taxes calculated**: Check that the token has tax implementation

## Further Reading

- [Token Parameter Extraction README](../../src/token_parameter_extraction/README.md)
- [Reth Transaction Simulator](https://github.com/your-org/reth_tx_simulator)
- [Uniswap V2 Documentation](https://docs.uniswap.org/contracts/v2/overview)