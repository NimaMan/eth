# Signal Engine Module

## Overview

The Signal Engine is the brain of the mempool processor, analyzing simulated transaction effects to detect market signals, risks, and opportunities in real-time. It categorizes pool state changes into actionable signals for automated trading systems.

## Purpose

Transform raw state changes from transaction simulation into categorized, confidence-scored market events that can trigger automated actions. The engine goes beyond simple scam detection to identify:
- Liquidity risks and opportunities
- Token supply anomalies (hidden mints)
- Volume spikes and market manipulation
- Price impact events for arbitrage

## Architecture

```
Transaction Simulation Results
         │
         ▼
┌─────────────────────────┐
│   Signal Engine         │
├─────────────────────────┤
│                         │
│  1. State Analysis      │──── Compare simulated vs current state
│  2. Threshold Check     │──── Apply dynamic thresholds
│  3. Event Categorize    │──── Classify into event types
│  4. Confidence Score    │──── Calculate reliability
│  5. Signal Publish      │──── Emit to multiple channels
│                         │
└─────────────────────────┘
         │
         ▼
    Market Events
```

## Event Categories

### 1. ScamAlert (Critical Severity)
**Purpose**: Detect rugpulls and liquidity drains
```rust
pub struct ScamAlert {
    pub pool: Address,
    pub drain_amount: f64,      // ETH drained
    pub drain_percentage: f64,   // % of pool drained
    pub recipient: Address,      // Who received the funds
}
```
**Triggers**:
- ETH reserve drops > 50%
- Pool liquidity < 0.1 ETH after TX
- Known scam patterns detected

### 2. LiquidityWarning (High Severity)
**Purpose**: Monitor significant liquidity changes
```rust
pub struct LiquidityWarning {
    pub pool: Address,
    pub eth_change: f64,         // Negative for withdrawals
    pub percent_change: f64,     // 20-50% range
    pub new_reserve: f64,        // Remaining ETH
}
```
**Triggers**:
- ETH reserve change 20-50%
- Large single-transaction impact
- Approaching critical thresholds

### 3. TokenSupplyAlert (High Severity)
**Purpose**: Detect hidden mints and supply manipulation
```rust
pub struct TokenSupplyAlert {
    pub pool: Address,
    pub token: Address,
    pub supply_increase: f64,    // New tokens appeared
    pub percent_increase: f64,   // % increase
}
```
**Triggers**:
- Token reserve increases > 10% in single TX
- Mismatch between expected and actual token flow
- Supply expansion without corresponding ETH

### 4. VolumeSpike (Medium Severity)
**Purpose**: Identify unusual trading activity
```rust
pub struct VolumeSpike {
    pub pool: Address,
    pub volume_multiplier: f64,  // vs average
    pub tx_count: u32,          // In current block
}
```
**Triggers**:
- Transaction volume > 5x average
- Multiple large trades in short time
- Coordinated trading patterns

### 5. PriceImpact (Medium Severity)
**Purpose**: Arbitrage and MEV opportunities
```rust
pub struct PriceImpact {
    pub pool: Address,
    pub price_change: f64,       // % change
    pub arb_opportunity: f64,    // Potential profit
}
```
**Triggers**:
- Price movement > 15%
- Cross-DEX arbitrage detected
- Sandwich attack opportunities

## Input Data

### SimulationResult
From tx_simulator module:
```rust
pub struct SimulationResult {
    pub tx_hash: String,
    pub affected_pools: HashMap<String, PoolEffect>,
    pub simulation_successful: bool,
}

pub struct PoolEffect {
    pub pool_address: String,
    pub current_eth_reserve: f64,
    pub simulated_eth_reserve: f64,
    pub current_token_reserve: f64,
    pub simulated_token_reserve: f64,
    pub eth_delta: f64,
    pub token_delta: f64,
}
```

### PoolState
From pool_subscriber cache:
```rust
pub struct PoolState {
    pub eth_reserve: f64,
    pub token_reserve: f64,
    pub last_updated_block: u64,
    pub volume_24h: f64,         // For spike detection
    pub tx_count_24h: u32,       // For pattern analysis
}
```

## Output Format

### MarketEvent
Unified event structure for all categories:
```rust
pub struct MarketEvent {
    pub event_type: EventType,
    pub severity: Severity,
    pub confidence: f64,         // 0.0 - 1.0
    pub tx_hash: String,
    pub pool: String,
    pub token: String,
    pub metrics: EventMetrics,
    pub detection_time: f64,
    pub details: String,
}

pub enum EventType {
    ScamAlert,
    LiquidityWarning,
    TokenSupplyAlert,
    VolumeSpike,
    PriceImpact,
}

pub enum Severity {
    Critical,   // Immediate action required
    High,       // Important, monitor closely
    Medium,     // Noteworthy, may require action
    Low,        // Informational
}
```

## Threshold Configuration

### Dynamic Thresholds
Thresholds adjust based on pool characteristics:
```rust
pub struct DecisionThresholds {
    // Base thresholds
    pub scam_drain_percent: f64,        // 50%
    pub warning_drain_percent: f64,      // 20%
    pub supply_increase_percent: f64,    // 10%
    pub volume_spike_multiplier: f64,    // 5x
    pub price_impact_percent: f64,       // 15%
    
    // Pool size adjustments
    pub small_pool_eth: f64,             // < 5 ETH
    pub medium_pool_eth: f64,            // 5-50 ETH
    pub large_pool_eth: f64,             // > 50 ETH
}
```

### Confidence Scoring
Factors affecting confidence:
1. **Data Freshness** (40%)
   - Pool state age < 12s: 100%
   - Pool state age < 60s: 80%
   - Older: Decreasing score

2. **Simulation Quality** (30%)
   - Successful trace: 100%
   - Partial trace: 60%
   - Failed trace: 0%

3. **Historical Accuracy** (30%)
   - Based on past predictions
   - Self-learning adjustments

## API Usage

### Creating the Engine
```rust
use mempool_processor::signal_engine::{SignalEngine, SignalConfig};

// Default configuration
let engine = SignalEngine::new(pool_cache);

// Custom configuration
let config = SignalConfig {
    scam_threshold: 0.4,        // 40% drain
    warning_threshold: 0.15,    // 15% change
    min_pool_eth: 0.1,         // Ignore tiny pools
    enable_ml_scoring: true,    // Use ML confidence
};
let engine = SignalEngine::with_config(pool_cache, config);
```

### Processing Transactions
```rust
// Analyze single transaction
let events = engine.analyze_transaction(simulation_result).await?;

for event in events {
    match event.event_type {
        EventType::ScamAlert => {
            // Critical: Block transaction, alert users
            send_alert(&event);
        }
        EventType::LiquidityWarning => {
            // Monitor pool, prepare defensive actions
            monitor_pool(&event);
        }
        _ => {
            // Log and forward to trading systems
            publish_event(&event);
        }
    }
}
```

### Batch Processing
```rust
// Process multiple simulations efficiently
let results = vec![sim1, sim2, sim3];
let all_events = engine.analyze_batch(results).await?;

// Events are sorted by severity and confidence
for event in all_events {
    handle_event(event);
}
```

## Implementation Details

### State Comparison Logic
```rust
fn calculate_pool_impact(&self, current: &PoolState, simulated: &PoolEffect) -> Impact {
    let eth_change_percent = (simulated.eth_delta / current.eth_reserve) * 100.0;
    let token_change_percent = (simulated.token_delta / current.token_reserve) * 100.0;
    
    // Detect hidden mints
    if token_change_percent > 10.0 && simulated.token_delta > 0.0 {
        return Impact::TokenSupplyIncrease(token_change_percent);
    }
    
    // Check for liquidity drains
    if eth_change_percent < -50.0 {
        return Impact::CriticalDrain(eth_change_percent.abs());
    }
    
    // More checks...
}
```

### Event Publishing
The engine publishes events through multiple channels:
1. **Log Files**: Timestamped event logs
2. **Database**: Persistent storage with metrics
3. **ZMQ Publisher**: Real-time signals to eth_kartal
4. **Metrics**: Prometheus/Grafana integration

## Performance Characteristics

- **Processing Time**: < 1ms per transaction
- **Memory Usage**: ~100KB per active pool
- **Concurrency**: Thread-safe, supports parallel analysis
- **Throughput**: 10,000+ transactions/second

## Configuration Files

### decision_config.toml
```toml
[thresholds]
scam_drain_percent = 50.0
warning_drain_percent = 20.0
supply_increase_percent = 10.0

[pools]
min_eth_threshold = 0.1
max_tracked_pools = 2000

[confidence]
enable_ml_scoring = true
min_confidence = 0.7
```

## Monitoring & Debugging

### Metrics Exposed
- `signal_engine_events_total{type, severity}`
- `signal_engine_processing_time_ms`
- `signal_engine_confidence_score{type}`
- `signal_engine_false_positives{type}`

### Debug Logging
```bash
RUST_LOG=mempool_processor::signal_engine=debug cargo run
```

## Future Enhancements

1. **Machine Learning Integration**
   - Pattern recognition for new scam types
   - Adaptive threshold adjustment
   - Anomaly detection

2. **Cross-Pool Analysis**
   - Coordinated attacks across multiple pools
   - Cascading liquidation risks
   - Market-wide sentiment analysis

3. **Historical Analysis**
   - Backtesting engine for threshold tuning
   - Performance analytics dashboard
   - Predictive modeling

4. **Integration Expansion**
   - Direct MEV bundle creation
   - Flashloan execution triggers
   - Cross-chain event correlation