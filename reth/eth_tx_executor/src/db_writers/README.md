# Logging Module

Comprehensive trade logging and audit trail system for eth_kartal with PostgreSQL integration and structured performance metrics.

## Purpose

The logging module provides:
- **Complete audit trails** for all trading decisions
- **Performance metrics** tracking for optimization
- **Database integration** with existing live trading schema
- **Structured logging** with correlation IDs
- **Real-time monitoring** capabilities

## Components

### `TradeLogger`
Central logging service that tracks the complete trading lifecycle:

```rust
pub struct TradeLogger {
    pool: Option<Arc<PgPool>>,
    wallet_address: Address,
}
```

### `TradeEvent`
Structured events for different stages of execution:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeEvent {
    AlertReceived { alert: Alert, received_at: DateTime<Utc> },
    RiskDecision { alert_id: String, decision: String, ... },
    TxSubmitted { alert_id: String, tx_hash: H256, ... },
    TxConfirmed { alert_id: String, tx_hash: H256, ... },
    TxFailed { alert_id: String, error: String, ... },
}
```

## Usage Examples

### Initialization

**With Database Connection:**
```rust
use eth_kartal::logging::TradeLogger;

let database_url = std::env::var("DATABASE_URL").ok();
let trade_logger = TradeLogger::new(
    database_url.as_deref(),
    wallet_address
).await?;
```

**Console-Only Mode:**
```rust
// Works without database - logs to console only
let trade_logger = TradeLogger::new(None, wallet_address).await?;
```

### Complete Trading Lifecycle Logging

```rust
// 1. Log incoming alert
let signal_id = trade_logger.log_alert_received(&alert).await;

// 2. Log risk assessment
trade_logger.log_risk_decision(
    signal_id,
    &alert.id,
    &risk_decision,
    original_amount
).await;

// 3. Log transaction submission
trade_logger.log_tx_submitted(
    signal_id,
    &alert.id,
    tx_hash,
    nonce,
    gas_price,
    "FlashbotsBundle"
).await;

// 4. Log final execution result
trade_logger.log_execution_result(
    signal_id,
    &alert.id,
    &execution_result
).await;
```

## Database Schema Integration

### Tables Updated

#### **trade_signals**
Main signal tracking with status state machine:

```sql
CREATE TABLE trade_signals (
    signal_id UUID PRIMARY KEY,
    alert_id VARCHAR NOT NULL,
    wallet_id UUID REFERENCES wallets(wallet_id),
    token_address VARCHAR NOT NULL,
    pool_id UUID REFERENCES pools(pool_id),
    action VARCHAR NOT NULL, -- 'BUY', 'SELL'
    amount_tokens VARCHAR NOT NULL,
    max_slippage_percent DECIMAL,
    priority VARCHAR NOT NULL,
    deadline_timestamp TIMESTAMP,
    status VARCHAR NOT NULL, -- 'PENDING', 'SUBMITTED', 'CONFIRMED', 'FAILED'
    tx_hash VARCHAR,
    risk_assessment JSONB,
    error_message TEXT,
    signal_timestamp TIMESTAMP DEFAULT NOW(),
    submission_timestamp TIMESTAMP,
    confirmation_timestamp TIMESTAMP,
    updated_at TIMESTAMP DEFAULT NOW()
);
```

#### **executions**
Detailed execution metrics and performance data:

```sql
CREATE TABLE executions (
    execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    signal_id UUID REFERENCES trade_signals(signal_id),
    attempt_number INTEGER DEFAULT 1,
    tx_hash VARCHAR NOT NULL,
    nonce INTEGER,
    gas_price VARCHAR,
    mev_protected BOOLEAN DEFAULT FALSE,
    success BOOLEAN,
    alert_to_start_ms INTEGER,
    position_check_ms INTEGER,
    gas_ranking_ms INTEGER,
    price_quote_ms INTEGER,
    tx_build_ms INTEGER,
    tx_submit_ms INTEGER,
    total_execution_ms INTEGER,
    error_details JSONB,
    execution_timestamp TIMESTAMP DEFAULT NOW()
);
```

## Logging Output Examples

### Console Logging
```
2024-01-15T10:30:15Z INFO  📨 Alert received: buy_signal_001 | Token: 0xA0b8... | Action: BUY | Amount: 500000000000000000 | Priority: HIGH
2024-01-15T10:30:15Z INFO  ⚖️ Risk decision: ALLOW | Alert: buy_signal_001 | Original: 500000000000000000 | Final: 500000000000000000
2024-01-15T10:30:15Z INFO  📤 Transaction submitted: 0x1a2b... | Alert: buy_signal_001 | Nonce: 42 | Gas: 25000000000 | Path: FlashbotsBundle
2024-01-15T10:30:15Z INFO  ✅ Execution successful: 0x1a2b... | Alert: buy_signal_001 | Latency: 187ms
2024-01-15T10:30:15Z INFO    📊 Performance breakdown:
2024-01-15T10:30:15Z INFO      • Alert→Start: 1ms
2024-01-15T10:30:15Z INFO      • Position check: 12ms
2024-01-15T10:30:15Z INFO      • Gas ranking: 8ms
2024-01-15T10:30:15Z INFO      • Price quote: 15ms
2024-01-15T10:30:15Z INFO      • TX build: 6ms
2024-01-15T10:30:15Z INFO      • TX submit: 145ms
```

### Error Logging
```
2024-01-15T10:30:16Z ERROR ❌ Execution failed: Alert: sell_signal_002 | Error: Insufficient liquidity
2024-01-15T10:30:16Z WARN  ⚖️ Risk manager reduced trade: Position limit exceeded, reducing from 1.0 ETH to 0.5 ETH
2024-01-15T10:30:16Z ERROR ❌ Event: Transaction failed for alert buy_signal_003: Transaction reverted
```

## Performance Metrics Tracking

### Execution Timing Breakdown

The logger tracks detailed performance metrics across all execution phases:

```rust
pub struct ExecutionMetrics {
    pub alert_to_start_ms: u64,     // Time from alert receipt to execution start
    pub position_check_ms: u64,     // Time to check current positions
    pub gas_ranking_ms: u64,        // Time for gas optimization calculations  
    pub price_quote_ms: u64,        // Time to get pool quotes
    pub tx_build_ms: u64,           // Time to build transaction
    pub tx_submit_ms: u64,          // Time to submit to network
    pub total_ms: u64,              // Total execution time
}
```

### Latency Targets vs Actual

| Phase | Target | Typical | Warning Threshold |
|-------|--------|---------|-------------------|
| Alert→Start | < 5ms | ~1ms | > 10ms |
| Position Check | < 20ms | ~12ms | > 50ms |
| Gas Ranking | < 15ms | ~8ms | > 30ms |
| Price Quote | < 25ms | ~15ms | > 50ms |
| TX Build | < 10ms | ~6ms | > 25ms |
| TX Submit | < 150ms | ~100ms | > 300ms |
| **Total** | **< 200ms** | **~140ms** | **> 400ms** |

## Analytics and Reporting

### Wallet Statistics

```rust
// Get 24-hour trading statistics
let stats = trade_logger.get_wallet_stats().await?;

// Returns JSON:
{
  "24h_stats": {
    "total_trades": 45,
    "successful": 42,
    "failed": 3,
    "success_rate": 0.933,
    "avg_execution_time_sec": 0.187
  }
}
```

### Custom Queries

**Top Performance Bottlenecks:**
```sql
SELECT 
    AVG(gas_ranking_ms) as avg_gas_ranking,
    AVG(tx_submit_ms) as avg_tx_submit,
    AVG(total_execution_ms) as avg_total
FROM executions 
WHERE execution_timestamp > NOW() - INTERVAL '1 hour'
AND success = true;
```

**Failure Analysis:**
```sql
SELECT 
    error_details->>'error' as error_type,
    COUNT(*) as failure_count
FROM executions 
WHERE success = false
AND execution_timestamp > NOW() - INTERVAL '24 hours'
GROUP BY error_details->>'error'
ORDER BY failure_count DESC;
```

## Integration with Monitoring Systems

### Structured Logging Format

All logs use structured JSON format for easy parsing:

```json
{
  "timestamp": "2024-01-15T10:30:15.123Z",
  "level": "INFO",
  "target": "eth_kartal::logging",
  "message": "Alert received",
  "fields": {
    "alert_id": "buy_signal_001",
    "token_address": "0xA0b86a33E6417aFb8D3c5C61308B6fCFa36C6a00b",
    "action": "BUY",
    "amount": "500000000000000000",
    "priority": "HIGH",
    "signal_id": "123e4567-e89b-12d3-a456-426614174000"
  }
}
```

### Metrics Export

**Prometheus Metrics:**
```rust
// Export execution metrics to Prometheus
use prometheus::{Counter, Histogram, register_counter, register_histogram};

static EXECUTION_COUNTER: Counter = register_counter!(
    "eth_kartal_executions_total",
    "Total number of trade executions"
);

static EXECUTION_DURATION: Histogram = register_histogram!(
    "eth_kartal_execution_duration_ms",
    "Trade execution duration in milliseconds"
);
```

### Log Aggregation

**ELK Stack Integration:**
```yaml
# logstash.conf
input {
  file {
    path => "/var/log/eth_kartal.log"
    codec => json
  }
}

filter {
  if [fields][alert_id] {
    mutate {
      add_field => { "alert_id" => "%{[fields][alert_id]}" }
    }
  }
}

output {
  elasticsearch {
    hosts => ["localhost:9200"]
    index => "eth-kartal-%{+YYYY.MM.dd}"
  }
}
```

## Error Handling and Resilience

### Database Connection Failures

```rust
// Graceful degradation when database is unavailable
if let Err(e) = result {
    error!("Failed to log to database: {}", e);
    // Continue with console logging only
}
```

### Log Rotation

```rust
// Automatic log file rotation
use tracing_appender::rolling::{RollingFileAppender, Rotation};

let file_appender = RollingFileAppender::new(
    Rotation::daily(),
    "/var/log/eth_kartal",
    "eth_kartal.log"
);
```

## Security and Compliance

### Sensitive Data Handling

```rust
// Never log private keys or passwords
info!("Wallet unlocked for address: {}", wallet_address); // ✅ Safe
// warn!("Failed to unlock wallet: {}", password); // ❌ Never do this
```

### Audit Trail Requirements

- **Immutable logs** - all trade decisions permanently recorded
- **Correlation IDs** - signal_id links all related events
- **Timestamp precision** - microsecond accuracy for ordering
- **Data integrity** - cryptographic checksums for log files

### Regulatory Compliance

- **MiFID II compliance** - detailed execution reporting
- **Best execution** - performance metrics for regulatory review
- **Data retention** - configurable retention periods
- **Privacy protection** - no personal data in logs

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_alert_logging() {
    let logger = TradeLogger::new(None, Address::zero()).await.unwrap();
    let alert = create_test_alert();
    
    let signal_id = logger.log_alert_received(&alert).await;
    assert!(!signal_id.is_nil());
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_database_integration() {
    let db_url = std::env::var("TEST_DATABASE_URL").unwrap();
    let logger = TradeLogger::new(Some(&db_url), Address::zero()).await.unwrap();
    
    // Test complete logging lifecycle
    let signal_id = logger.log_alert_received(&alert).await;
    logger.log_execution_result(signal_id, &alert.id, &result).await;
    
    // Verify data in database
    let stats = logger.get_wallet_stats().await.unwrap();
    assert_eq!(stats["24h_stats"]["total_trades"], 1);
}
```

## Best Practices

### **Structured Logging**
- Use consistent field names across all log entries
- Include correlation IDs (signal_id) in all related logs
- Log at appropriate levels (INFO for normal flow, WARN for recoverable issues, ERROR for failures)

### **Performance Logging**
- Always log execution timing metrics
- Include context for slow operations
- Monitor and alert on performance degradation

### **Error Logging** 
- Include full error context and recovery actions
- Log errors at the point of occurrence
- Use structured error types for consistent formatting

### **Database Logging**
- Use transactions for related database updates
- Handle database unavailability gracefully
- Implement retry logic for transient failures