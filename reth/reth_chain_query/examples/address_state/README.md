# Address State Examples

This directory contains examples demonstrating the `address_state` module functionality for querying and analyzing Ethereum address balances and state.

## Examples

### Basic Operations

- **[balances.rs](balances.rs)** - Query ETH and token balances
  - Single address ETH balance
  - Multiple token balances for an address
  - Historical balance queries at specific blocks
  - Account type detection (EOA vs Contract)

- **[portfolio.rs](portfolio.rs)** - Build complete portfolio views
  - Complete portfolio for an address (ETH + multiple tokens)
  - Multi-address portfolio aggregation
  - Historical portfolio snapshots
  - Portfolio value tracking over time

### Advanced Features

- **[batch_operations.rs](batch_operations.rs)** - Efficient batch queries
  - `batch_get_eth_balances` - Multiple addresses ETH balances in parallel
  - `batch_get_token_balances` - Multiple token-holder pairs
  - `batch_get_portfolio_balances` - One address, multiple tokens
  - Performance comparison: batch vs sequential queries
  - Shows 10-50x performance improvement with batching

- **[balance_changes.rs](balance_changes.rs)** - Track balance changes
  - `get_balance_changes` - Calculate changes between two blocks
  - Identify deposits and withdrawals
  - Multi-token change tracking
  - Exchange flow monitoring
  - Net inflow/outflow analysis

- **[historical_analysis.rs](historical_analysis.rs)** - Historical balance analysis
  - `get_historical_balance` - Query balances at any block
  - Balance evolution through Ethereum milestones
  - Token balance history since launch
  - Monthly/yearly balance snapshots
  - Works for both ETH and tokens with same method

## Key Methods Demonstrated

### From `address_state.rs`

- `get_eth_balance(address, block)` - Get ETH balance
- `get_complete_balances(address, tokens, block)` - Get ETH + token balances
- `get_historical_balance(address, token, block)` - Historical balance (ETH if token=None)
- `get_balance_changes(address, tokens, from_block, to_block)` - Track changes
- `get_nonce(address, block)` - Get transaction count
- `has_code(address, block)` - Check if address is a contract

### From `batch_ops.rs`

- `batch_get_eth_balances(addresses, block)` - Multiple ETH balances
- `batch_get_token_balances(pairs, block)` - Multiple token-holder pairs
- `batch_get_portfolio_balances(address, tokens, block)` - Full portfolio
- `batch_get_nonces(addresses, block)` - Multiple nonces

## Running Examples

```bash
# Basic balance queries
cargo run --example balances

# Portfolio management
cargo run --example portfolio

# Batch operations (shows performance gains)
cargo run --example batch_operations

# Balance change tracking
cargo run --example balance_changes

# Historical analysis
cargo run --example historical_analysis
```

## Performance Notes

- Batch operations are 10-50x faster than sequential queries
- Direct database access eliminates RPC overhead
- Parallel processing for multi-item queries
- Memory-mapped database provides zero-copy access

## Common Patterns

1. **Always use batch operations for multiple queries** - Significantly faster
2. **Use `get_historical_balance` for both ETH and tokens** - Single method for both
3. **Track balance changes for flow analysis** - Useful for whale watching
4. **Check if address is contract before token queries** - Avoids unnecessary calls