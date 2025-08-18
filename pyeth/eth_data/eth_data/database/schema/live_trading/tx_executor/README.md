# Transaction Executor Enhanced Schema

This directory contains enhanced database models for execution tracking within the `live_trading_db` database. These models extend the existing trade_signals and executions tables with detailed performance metrics and tracking capabilities for the ETH Kartal high-performance transaction execution engine.

## Database Overview

**Database Name:** `live_trading_db` (shared with existing trading schema)

The enhanced execution schema is designed to provide detailed insights into transaction execution performance, with sub-200ms execution targets. It extends the existing tables with additional tracking and metrics.

## Database Architecture

### Integration with Existing Schema

The system uses two main databases:

1. **`eth_db`** - Core blockchain data (pools, tokens, transactions)
2. **`live_trading_db`** - Trading positions, strategy management, and execution tracking (includes this enhanced schema)

The enhanced execution tables within `live_trading_db` provide:
- Detailed execution metrics beyond the basic executions table
- Performance tracking and optimization data
- MEV protection statistics
- Gas optimization insights

## Core Tables

### 1. `execution_wallets`
Enhanced wallet management for execution tracking.

**Key Fields:**
- `wallet_id` - Links to existing wallets table
- `keystore_path` - Path to encrypted keystore
- `execution_enabled` - Enable/disable execution
- `max_gas_per_tx_eth` - Per-transaction gas limit
- `require_flashbots_above_eth` - Auto-enable MEV protection
- Detailed execution statistics

**Purpose:** Extends basic wallet management with execution-specific configuration and tracking.

### 2. `trade_signals` (existing table)
Trading signals queued for execution by ETH Kartal. The enhanced schema adds detailed tracking via execution_details.

**Status Flow:**
```
PENDING → SENT → CONFIRMED
           ↓        ↓
        FAILED   FAILED
```

### 3. `executions` (existing table)
Basic execution records. Extended by `execution_details` table for comprehensive metrics.

### 4. `execution_details` (new table)
Enhanced execution tracking with detailed performance metrics.

**Key Features:**
- Component-level timing breakdown (validation, gas optimization, etc.)
- MEV protection tracking
- Network congestion monitoring
- Exact amount tracking for precise P&L

**Timing Metrics (all in milliseconds):**
- `signal_received_ms` - Initial signal processing
- `validation_time_ms` - Input validation
- `position_check_ms` - Balance verification
- `gas_estimation_ms` - Gas requirement calculation
- `gas_ranking_ms` - Gas price optimization
- `route_calculation_ms` - Trading route optimization
- `price_quote_ms` - Price calculation
- `tx_build_ms` - Transaction construction
- `tx_sign_ms` - Transaction signing
- `tx_submit_ms` - Network submission
- `mempool_time_ms` - Time in mempool
- `confirmation_time_ms` - Block confirmation
- `total_execution_ms` - End-to-end latency

### 5. `execution_errors`
Detailed error tracking for debugging and improvement.

**Error Types:**
- `INSUFFICIENT_BALANCE` - Not enough tokens/ETH
- `SLIPPAGE_EXCEEDED` - Price moved beyond tolerance
- `GAS_PRICE_TOO_HIGH` - Exceeded max gas limit
- `DEADLINE_EXCEEDED` - Signal expired
- `RISK_LIMIT_EXCEEDED` - Position too large
- `NONCE_CONFLICT` - Transaction ordering issue
- `NETWORK_ERROR` - RPC/network issues

**Enhanced Features:**
- Full transaction data capture
- RPC node response logging
- Retry tracking and scheduling
- Context capture (balances at error time)

## Performance Monitoring

Instead of a separate metrics table, use the `execution_metrics_hourly` view or query `execution_details` directly:

```sql
-- Get hourly performance metrics
SELECT * FROM execution_metrics_hourly
WHERE wallet_id = 1 AND hour > NOW() - INTERVAL '24 hours';
```

## Views

### `active_executions`
Shows all in-progress executions with current status.

### `execution_performance_summary`
Wallet-level performance overview with key metrics.

## Python Models

The schema is implemented using SQLAlchemy models in:
- `tx_executor_models.py` - Transaction executor tracking models

## Integration Points

### With ETH Kartal (Rust)

ETH Kartal integration flow:

1. Poll existing `trade_signals` table for PENDING signals
2. Execute trades and update `executions` table
3. Create detailed record in `execution_details` table
4. Log any errors to `execution_errors` table

```python
# Example: Create execution details after trade
execution_detail = ExecutionDetail(
    execution_id=execution.id,
    wallet_id=execution_wallet.id,
    total_execution_ms=125,
    gas_ranking_ms=28,
    price_quote_ms=18,
    tx_build_ms=8,
    tx_submit_ms=45,
    mev_protected=True
)
```

### With Existing Trading System

The enhanced schema seamlessly integrates with existing tables:

```python
# Link to existing position
execution.position_id = live_position.id

# Track detailed metrics
execution_detail.execution_id = execution.id
```

## Usage Examples

### Initialize Enhanced Tables

```bash
# Run initialization script
python initialize_tx_executor.py

# View schema info
python initialize_tx_executor.py --info

# Drop tables (CAUTION!)
python initialize_tx_executor.py --drop
```

### Enable Execution Tracking for Wallet

```sql
-- Link existing wallet to enhanced tracking
INSERT INTO execution_wallets (wallet_id, execution_enabled, keystore_path)
SELECT id, true, '/path/to/keystore'
FROM wallets 
WHERE wallet_address = '0x123...';
```

### Query Execution Performance

```sql
-- Get detailed execution metrics
SELECT 
    ed.total_execution_ms,
    ed.gas_ranking_ms,
    ed.price_quote_ms,
    ed.mev_protected,
    e.status
FROM executions e
JOIN execution_details ed ON ed.execution_id = e.id
WHERE e.signal_id = 'uuid-here';

-- Analyze performance directly from execution_details
SELECT 
    DATE_TRUNC('hour', created_at) as hour,
    COUNT(*) as executions,
    AVG(total_execution_ms) as avg_latency,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY total_execution_ms) as p95_latency
FROM execution_details
WHERE wallet_id = 1 AND created_at > NOW() - INTERVAL '24 hours'
GROUP BY hour
ORDER BY hour DESC;
```

### Monitor Gas Optimization

```sql
-- Gas cost analysis
SELECT 
    DATE_TRUNC('day', created_at) as day,
    AVG(base_fee_gwei + priority_fee_gwei) as avg_gas_price_gwei,
    SUM(gas_cost_eth) as total_gas_cost_eth,
    COUNT(*) as executions
FROM execution_details
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY day
ORDER BY day DESC;
```

## Performance Targets

### Latency Breakdown Targets
- Signal validation: < 5ms
- Position check: < 20ms
- Gas optimization: < 30ms
- Price quote: < 25ms
- Transaction build: < 10ms
- Submission: < 50ms
- **Total target: < 200ms**

### Success Rate Targets
- Normal priority: > 95%
- High priority: > 98%
- Critical priority: > 99.5%

## Monitoring and Alerts

Key metrics to monitor:
- Average `total_execution_ms` > 200ms
- Execution success rate < 95%
- Gas optimization savings < 5%
- MEV attacks detected > threshold
- Error rate by type > 5%

## Migration Notes

This schema extends the existing `live_trading_db` tables rather than replacing them:

```sql
-- Initialize enhanced execution tables
python initialize_tx_executor.py

-- Link existing wallet to execution tracking
INSERT INTO execution_wallets (wallet_id, execution_enabled)
SELECT id, true FROM wallets WHERE is_active = true;

-- Create execution details for new executions
-- This happens automatically via ETH Kartal integration
```

## Security Considerations

1. **Wallet Security**: Keystore paths must be encrypted at rest
2. **Access Control**: Use database roles to limit access
3. **Data Retention**: Archive old execution data regularly
4. **PII Protection**: No personal data in execution records
5. **Audit Trail**: All modifications tracked with timestamps