# PostgreSQL Database Query Module

## Overview
This module provides efficient querying of aggregated Ethereum trading data stored in PostgreSQL. It complements the blockchain queries by accessing pre-processed analytics data from the eth_db schema.

## Architecture

### Core Components
- **postgres_db/connection.rs**: Database connection pool management with sqlx
- **postgres_db/models.rs**: Rust structs matching PostgreSQL schema tables
- **postgres_db/queries/**: Specialized query modules organized by domain

### Query Modules
1. **address_metrics.rs**: Address-level aggregated metrics (PnL, volume, activity)
2. **trades.rs**: Trade-level queries and PnL calculations for address-token pairs
3. **tokens.rs**: Token metadata, pool information, and scam detection
4. **analytics.rs**: Complex analytical queries (rankings, network analysis)

## Performance Features
- Connection pooling (10 max, 2 min connections)
- Prepared statements for repeated queries
- Zero-copy deserialization with sqlx
- Batch query support for efficiency

## Usage Examples

### Initialize Connection
```rust
use reth_chain_query::postgres_db::PostgresQuery;

let database_url = "postgresql://postgres:postgres@localhost:5432/eth_db";
let pg_query = PostgresQuery::new(&database_url).await?;
```

### Query Top Profitable Addresses
```rust
use reth_chain_query::postgres_db::queries;

let top_addresses = queries::address_metrics::get_top_profitable_addresses(
    pg_query.db(),
    10,              // limit
    Some(10000.0),   // min volume
    true,            // exclude contracts
).await?;
```

### Get PnL for Address-Token Pair
```rust
let pnl = queries::trades::get_address_token_pnl(
    pg_query.db(),
    "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0",
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  // USDC
).await?;
```

### Find Token Pools
```rust
let pools = queries::tokens::get_token_pools(
    pg_query.db(),
    "0x6B175474E89094C44Da98b954EedeAC495271d0F",  // DAI
).await?;
```

## Examples
- **address_profitability.rs**: Top profitable addresses with rankings
- **trade_pnl_calculation.rs**: PnL analysis for trades
- **token_scam_analysis.rs**: Scam token detection and pool analysis
- **pool_discovery.rs**: Finding pools across DEX protocols

## Running Examples
```bash
DATABASE_URL="postgresql://postgres:postgres@localhost:5432/eth_db" \
cargo run --example address_profitability
```

## Integration with pyreth
This module can be integrated into pyreth for Python access to PostgreSQL queries, providing:
- Fast aggregated data access for analytics
- Pre-computed metrics without blockchain queries
- Efficient batch operations for large datasets