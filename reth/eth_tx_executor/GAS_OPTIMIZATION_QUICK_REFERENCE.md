# Gas Optimization Quick Reference 🚀

## Data Sources & Usage

### 1. **Mempool Gas Metrics** (100ms updates)
```rust
// SOURCE: mempool_processor via HTTP API
let metrics = gas_client.get_gas_metrics().await;

// DATA PROVIDED:
- gas_price_percentiles (P10-P99)  → Base price selection
- pending_tx_count                  → Congestion assessment  
- gas_price_histogram              → Exact position calculation
- avg_gas_price                    → Market baseline
```

### 2. **Congestion Metrics**
```rust
// SOURCE: mempool_processor calculations
let congestion = gas_client.get_congestion_metrics().await;

// DATA PROVIDED:
- congestion_level     → Safety margin multiplier
- arrival_rate/second  → Volatility indicator
- growth_rate         → Trend direction
```

### 3. **MEV Activity**
```rust
// SOURCE: High gas price detection
let mev = gas_client.get_mev_activity().await;

// DATA PROVIDED:
- mev_percentage      → Flashbots routing decision
- avg_mev_gas_price  → Competition baseline
- high_priority_count → Market heat indicator
```

## Optimization Algorithm

### Step-by-Step Process

```rust
1. Strategy Selection
   Priority::Critical → Aggressive (25% margin)
   Priority::High    → Targeted (15% margin)
   Priority::Normal  → Economic (5% margin)

2. Base Gas Calculation
   Position 10 → Query histogram → 85 gwei

3. Safety Margin
   Base margin × Congestion multiplier
   15% × 1.2 (Medium) = 18%

4. Final Gas Price
   85 gwei × 1.18 = 100.3 gwei → 101 gwei

5. Position Validation
   Check: Will 101 gwei achieve position ≤10?
   Result: Position 8, Confidence 72%
```

## Key Formulas

### Position → Gas Price
```rust
gas_price = gas_client.get_gas_price_for_position(target_pos).await;
// Adds 1 gwei buffer automatically
```

### Gas Price → Position
```rust
position = gas_client.get_position_for_gas_price(gas).await;
// Returns: position, confidence, percentile
```

### Safety Margins
| Congestion | Multiplier | Example (15% base) |
|------------|------------|-------------------|
| Low        | 1.0×       | 15%              |
| Medium     | 1.2×       | 18%              |
| High       | 1.5×       | 22.5%            |
| Extreme    | 2.0×       | 30%              |

### MEV Adjustment
```rust
if mev_percentage > 15% && strategy == Aggressive {
    gas = max(gas, avg_mev_gas × 1.1)
}
```

## Performance Metrics

| Operation | Time | Notes |
|-----------|------|-------|
| Get metrics | 0.3ms | Pre-calculated |
| Calculate position | 0.2ms | Binary search |
| Full optimization | 2ms | All steps |
| API round-trip | <1ms | HTTP overhead |

## Integration Code

### Quick Start
```rust
// 1. Create client
let client = MempoolGasClient::new("http://localhost:8088");

// 2. Get optimal gas for position
let gas = client.get_gas_price_for_position(10).await?;

// 3. Validate confidence
let position = client.get_position_for_gas_price(gas).await?;
if position.confidence < 0.7 {
    // Increase margin
}
```

### Complete Example
```rust
async fn optimize_gas(alert: &Alert) -> Result<U256> {
    let client = MempoolGasClient::new("http://localhost:8088");
    
    // Get all metrics
    let metrics = client.get_gas_metrics().await?;
    let congestion = metrics.congestion.congestion_level;
    
    // Determine target
    let target_pos = match alert.params.priority {
        Priority::Critical => 1,
        Priority::High => 10,
        Priority::Normal => 100,
    };
    
    // Get base gas
    let mut gas = client.get_gas_price_for_position(target_pos).await?;
    
    // Apply safety margin
    let margin = get_safety_margin(alert.params.priority, congestion);
    gas = gas * (100 + margin) / 100;
    
    // MEV check
    if metrics.mev_activity.mev_percentage > 15.0 {
        gas = gas.max(metrics.mev_activity.avg_mev_gas_price);
    }
    
    Ok(gas)
}
```

## Troubleshooting

### Low Confidence (<50%)
- **Cause**: Small sample size or stale data
- **Fix**: Wait for more transactions or increase margin

### Position Worse Than Expected
- **Cause**: MEV competition or rapid changes
- **Fix**: Use Aggressive strategy or Flashbots

### API Timeout
- **Cause**: Service unavailable
- **Fix**: Check mempool_processor is running

## Configuration

### Gas Metrics Config
```rust
GasMetricsConfig {
    buffer_size: 10_000,        // Transaction buffer
    update_interval_ms: 100,    // Calculation frequency
    max_data_age_secs: 60,      // Data freshness
}
```

### Client Config
```rust
GasClientConfig {
    api_url: "http://localhost:8088",
    timeout_ms: 1000,
    cache_ttl_ms: 100,  // Local cache
}
```