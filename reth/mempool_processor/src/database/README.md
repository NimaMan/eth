# Database Module

This module provides PostgreSQL database connectivity for persisting scam detections and market events.

## Components

### `scam_prediction_writer`
Specialized PostgreSQL writer for persisting mempool scam predictions to the `eth_db.mempool_scam_predictions` table.

**Features:**
- Automatic connection management with reconnection
- Async/await support using tokio-postgres
- Environment variable configuration
- Graceful handling of foreign key constraints (missing tokens)
- Detailed logging of scam events with transaction hashes
- Thread-safe with Arc<Mutex<>> wrapper

**Usage:**
```rust
use mempool_processor::database::ScamPredictionWriter;

// Create with default connection (reads from env vars)
let writer = ScamPredictionWriter::default().await?;

// Or create with explicit connection params
let writer = ScamPredictionWriter::new("user", "password", "localhost", 5432, "eth_db").await?;

// Write a scam prediction with transaction hash
writer.write_mempool_scam_prediction_with_tx(
    token_address,
    pool_address,
    block_number,
    current_eth_level,
    simulated_eth_level,
    eth_threshold,
    Some(tx_hash),
).await?;
```

**Logged Information:**
When a scam is detected, the writer logs:
- Transaction hash
- Token and pool addresses
- ETH drained (before → after)
- Percentage loss
- Database write status

## Environment Variables

- `DB_USER` - Database username (default: "postgres")
- `DB_PASSWORD` - Database password (default: "postgres")
- `DB_HOST` - Database host (default: "localhost")
- `DB_PORT` - Database port (default: 5432)
- `DB_NAME` - Database name (default: "eth_db")