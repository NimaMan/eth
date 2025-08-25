# PostgreSQL Database Query Module

## Overview

This module provides efficient querying of Ethereum blockchain data stored in PostgreSQL. It complements the direct blockchain queries by accessing pre-processed data from the `eth_db` schema, enabling fast analytics and aggregations without touching the blockchain.

## Architecture

```
postgres_db/
├── mod.rs                 # Module exports and PostgresQuery client
├── connection.rs          # Database connection pool management
├── models.rs              # Rust structs matching PostgreSQL schema
└── queries/
    ├── mod.rs             # Query module exports
    ├── address_metrics.rs # Address-level aggregated metrics
    ├── trades.rs          # Trade-level PnL calculations
    ├── tokens.rs          # Token metadata and pool queries
    ├── transactions.rs    # Raw transaction and participant queries
    ├── analytics.rs       # Complex analytical queries
    ├── aggregation.rs     # Data aggregation from trades to addresses
    └── population.rs      # Database population helpers
```

## Database Schema

The module expects the following PostgreSQL tables in the `eth_db` schema:

### Core Tables
- **addresses** - Wallet/contract addresses with aggregated metrics
- **transactions** - Transaction records with block numbers and status
- **tx_participants** - Many-to-many relationship between transactions and addresses
- **trades** - Aggregated trades between addresses and tokens
- **tokens** - ERC20 token metadata with scam detection
- **pools** - DEX pools across protocols (V2, V3, V4)

## Query Modules

### 1. Address Metrics (`address_metrics.rs`)
Queries for address-level aggregated data:
```rust
// Get metrics for a specific address
let metrics = get_address_metrics(db, "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0").await?;

// Get top profitable addresses
let top = get_top_profitable_addresses(db, 10, Some(10000.0), true).await?;

// Find high scam ratio addresses
let scam = get_high_scam_ratio_addresses(db, 0.8, 10).await?;
```

### 2. Transactions (`transactions.rs`)
Raw transaction and participant queries for graph building:
```rust
// Get all transactions where an address appears
let txs = get_address_transactions(db, address, 100, None).await?;

// Get all participants in a transaction
let participants = get_tx_participants(db, tx_hash).await?;

// Find transactions between two addresses
let txs = get_transactions_between_addresses(db, addr1, addr2, 10).await?;
```

### 3. Trades (`trades.rs`)
Trade-level PnL calculations:
```rust
// Get trades for an address
let trades = get_trades_for_address(db, address, None).await?;

// Calculate PnL for address-token pair
let pnl = get_address_token_pnl(db, address, token).await?;

// Get trade summary
let summary = get_address_trade_summary(db, address).await?;
```

### 4. Tokens (`tokens.rs`)
Token metadata and pool discovery:
```rust
// Get token information
let token = get_token_info(db, token_address).await?;

// Find all pools for a token
let pools = get_token_pools(db, token_address).await?;

// Get scam tokens
let scam_tokens = get_scam_tokens(db, Some(100)).await?;
```

### 5. Analytics (`analytics.rs`)
Complex analytical queries:
```rust
// Calculate address rankings
let rankings = calculate_address_ranking(db, 100).await?;

// Get network relationships
let relationships = get_network_relationships(db, address, depth).await?;

// Global statistics
let stats = get_global_statistics(db).await?;
```

### 6. Aggregation (`aggregation.rs`)
Populate addresses table from trades:
```rust
// Aggregate all metrics
let rows = aggregate_address_metrics(db).await?;

// Calculate scam ratios
let rows = calculate_scam_ratios(db).await?;

// Batch update with progress
let rows = batch_update_addresses(db, 1000, offset).await?;
```

### 7. Population (`population.rs`)
Database maintenance and population:
```rust
// Full population from trades
let result = populate_addresses_from_trades(db, full_refresh).await?;

// Find stale addresses
let stale = get_addresses_needing_update(db, 100).await?;

// Get population statistics
let stats = get_population_statistics(db).await?;
```

## Usage Examples

### Initialize Connection
```rust
use reth_chain_query::postgres_db::{PostgresQuery, queries};

let database_url = "postgresql://postgres:postgres@localhost:5432/eth_db";
let pg_query = PostgresQuery::new(&database_url).await?;
```

### Find Top Traders
```rust
let top_traders = queries::address_metrics::get_top_profitable_addresses(
    pg_query.db(),
    10,              // limit
    Some(10000.0),   // minimum volume
    true,            // exclude contracts
).await?;

for trader in top_traders {
    println!("{}: ${:.2} profit", trader.address, trader.total_profit.unwrap_or(0.0));
}
```

### Build Transaction Graph
```rust
// Get all transactions for an address
let txs = queries::transactions::get_address_transactions(
    pg_query.db(),
    &address,
    100,  // limit
    None, // max_block
).await?;

// For each transaction, get all participants
for tx in txs {
    let participants = queries::transactions::get_tx_participants(
        pg_query.db(),
        &tx.tx_hash,
    ).await?;
    
    // Build graph edges between participants
    for participant in participants {
        // Add edge to graph
    }
}
```

### Populate Database
```rust
// Run full aggregation
let result = queries::population::populate_addresses_from_trades(
    pg_query.db(),
    true,  // full refresh
).await?;

println!("Updated {} addresses", result.addresses_updated_profit);
```

## Performance

- Connection pooling with 10 max connections
- Prepared statements for repeated queries
- Zero-copy deserialization with sqlx
- Batch operations for large updates
- Indexed queries for fast lookups

## Running Examples

```bash
# Basic examples
DATABASE_URL="postgresql://..." cargo run --example address_profitability
DATABASE_URL="postgresql://..." cargo run --example trade_pnl_calculation

# Population scripts
DATABASE_URL="postgresql://..." cargo run --example populate_address_metrics
DATABASE_URL="postgresql://..." BATCH_SIZE=5000 cargo run --example calculate_pnl_metrics
```

## Integration with Other Modules

This module is used by:
- **fundflownetwork** - For building transaction graphs and fund flow analysis
- **pyreth** - Python bindings for accessing PostgreSQL data
- **qarqa_analytics** - Analytics and insights generation