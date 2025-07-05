# Trade Logging and Slippage Validation

## Overview

This document describes the trade logging and slippage validation features added to eth_kartal for production trading.

## Slippage Validation

### Module: `src/common/validation.rs`

- **Bounds**: 0.1% (min) to 10% (max), default 3%
- **Input handling**: Accepts both decimal (0.05) and percentage (5.0) formats
- **Validation**: Enforced before any trade execution
- **Error handling**: Returns `KartalError::Validation` with descriptive message

### Usage:
```rust
let validated_slippage = validate_slippage(alert.params.slippage)?;
let min_amount_out = apply_slippage(amount_out, validated_slippage);
```

## Trade Logging

### Module: `src/logging/trade_logger.rs`

Comprehensive logging system that tracks all trading activity with PostgreSQL integration.

### Key Features:

1. **Alert Logging**: Records incoming alerts with signal_id generation
2. **Risk Decision Logging**: Tracks risk manager decisions (Allow/Reduce/Block/Halt)
3. **Transaction Submission**: Logs tx hash, nonce, gas price, execution path
4. **Execution Results**: Records success/failure with detailed performance metrics

### Database Integration:

- Uses dynamic SQL queries (not compile-time checked) for flexibility
- Gracefully handles missing database connection
- Integrates with existing schema in `/home/nima/code/crypto/py/sarigoz/sarigoz/data/db/schema/live_trading`

### Tables Updated:

1. **trade_signals**: Main signal tracking with status state machine
2. **executions**: Detailed execution metrics and performance data

### Performance Metrics Tracked:

- Alert to start latency
- Position check time
- Gas ranking optimization time
- Price quote time
- Transaction building time
- Transaction submission time
- Total execution time

## Integration Points

### Transaction Executor (`src/tx_executor/executor.rs`)

1. **Input Validation**: Added `validate_alert_inputs()` method
2. **Signal Tracking**: Generates UUID for each alert to link all events
3. **Risk Logging**: Logs risk decisions before execution
4. **Submission Logging**: Records transaction submission details
5. **Result Logging**: Captures final execution outcome

### Execution Flow:

1. Alert received → Generate signal_id
2. Validate inputs (token, pool, slippage)
3. Check position and calculate trade
4. Apply slippage validation
5. Log risk decision
6. Build and submit transaction
7. Log submission details
8. Log final result

## Configuration

### Environment Variables:

- `DATABASE_URL`: PostgreSQL connection string (optional)
- If not set, logging continues to console only

### Example:
```bash
export DATABASE_URL="postgres://user:pass@localhost/eth_db"
```

## Usage Example:

```rust
// Initialize trade logger
let trade_logger = TradeLogger::new(
    database_url.as_deref(),
    wallet_address
).await?;

// Log alert
let signal_id = trade_logger.log_alert_received(&alert).await;

// Log risk decision
trade_logger.log_risk_decision(
    signal_id,
    &alert.id,
    &risk_decision,
    original_amount
).await;

// Log transaction submission
trade_logger.log_tx_submitted(
    signal_id,
    &alert.id,
    tx_hash,
    nonce,
    gas_price,
    "PublicMempool"
).await;
```

## Production Readiness

✅ **Slippage Protection**: Prevents excessive slippage losses
✅ **Audit Trail**: Complete record of all trading decisions
✅ **Performance Tracking**: Detailed latency metrics for optimization
✅ **Database Integration**: Works with existing live trading schema
✅ **Graceful Degradation**: Functions without database connection