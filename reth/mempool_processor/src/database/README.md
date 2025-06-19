# Database Module

This module provides database connectivity and utilities for the mempool processor.

## Components

### `db_logger`
PostgreSQL database logger for persisting mempool scam predictions and other analytics data.

**Features:**
- Connection pooling with automatic reconnection
- Async/await support using tokio-postgres
- Environment variable configuration
- Error handling with foreign key constraint support
- Transaction management for reliable data persistence

**Usage:**
```rust
use mempool_processor::database::DbLogger;

// Create with default connection (reads from env vars)
let logger = DbLogger::default().await?;

// Or create with explicit connection params
let logger = DbLogger::new("user", "password", "localhost", 5432, "eth_db").await?;

// Write a scam prediction
logger.write_mempool_scam_prediction(
    token_address,
    pool_address,
    block_number,
    current_eth,
    simulated_eth,
    threshold,
).await?;
```

## Environment Variables

- `DB_USER` - Database username (default: "postgres")
- `DB_PASSWORD` - Database password (default: "postgres")
- `DB_HOST` - Database host (default: "localhost")
- `DB_PORT` - Database port (default: 5432)
- `DB_NAME` - Database name (default: "eth_db")