# Reth Chain Query Examples

This directory contains practical examples demonstrating how to use `reth_chain_query` for high-performance blockchain queries.

## 🚀 Quick Start

If you're new to reth_chain_query, start here:

1. **[quickstart/setup.rs](quickstart/setup.rs)** - Initialize the provider and run your first queries
2. **[quickstart/common_patterns.rs](quickstart/common_patterns.rs)** - Learn resource sharing, error handling, and batch operations

## 📚 Examples by Category

### 💰 Accounts & Balances
- **[accounts/balances.rs](accounts/balances.rs)** - Query ETH and token balances, historical balances
- **[accounts/portfolio.rs](accounts/portfolio.rs)** - Build complete portfolio views with multiple tokens

### 📝 Transactions
- **[transactions/lookup_and_receipts.rs](transactions/lookup_and_receipts.rs)** - Find transactions, get receipts and logs
- **[transactions/block_transactions.rs](transactions/block_transactions.rs)** - Get all transactions, receipts, events, traces, and gas metrics from blocks

### 🪙 Tokens
- **[tokens/token_info.rs](tokens/token_info.rs)** - Get token metadata (name, symbol, decimals, supply)

### 💾 Storage & State
- **[storage/read_storage.rs](storage/read_storage.rs)** - Direct storage slot access, proxy detection

### 💱 DeFi
- **[defi/dex_pools.rs](defi/dex_pools.rs)** - Query Uniswap V2/V3 pools, prices, and liquidity

### 🏢 Entity Monitoring
- **[entities/cex_monitoring.rs](entities/cex_monitoring.rs)** - Track exchange balances and holdings

## 🛠️ Prerequisites

1. **Reth Node**: You need a synced Reth node with the database accessible:
   ```bash
   # Default location
   /home/nima/.local/share/reth/mainnet
   ```

2. **Dependencies**: Add to your `Cargo.toml`:
   ```toml
   [dependencies]
   reth_chain_query = { path = "../path/to/reth_chain_query" }
   tokio = { version = "1.0", features = ["full"] }
   eyre = "0.6"
   ```

## 🎯 Key Concepts

### Provider Initialization
```rust
// Basic setup
let provider = RethQueryProvider::new("/path/to/reth/mainnet")?;

// With optional features
let provider = RethQueryProvider::new("/path/to/reth/mainnet")?
    .with_rpc_endpoint("http://localhost:8545")?  // For traces
    .with_reth_index("/path/to/index")?;          // For fast lookups
```

### Resource Sharing
```rust
// Share provider across async tasks
let provider = Arc::new(RethQueryProvider::new(reth_dir)?);
let provider_clone = provider.clone();

tokio::spawn(async move {
    // Use provider_clone in async task
});
```


## 🚀 Performance Tips

1. **Use batch operations** - Query multiple items in parallel
2. **Share the provider** - Use `Arc` to share across tasks
3. **Use specific block numbers** - Avoid repeated `latest` queries

## 📖 Learn More

- See the [main README](../README.md) for architecture details
- Check [CLAUDE.md](../CLAUDE.md) for development notes
- Review the [src/provider](../src/provider) module for implementation
