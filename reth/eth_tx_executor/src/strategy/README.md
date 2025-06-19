# Strategy Module

## Overview
The Strategy module implements the decision-making logic for responding to scam alerts. It analyzes market conditions, evaluates risks, and selects appropriate trading strategies to protect users from losses.

## Components

### `engine.rs`
- Main decision engine that processes alerts
- Evaluates market conditions and risk factors
- Selects optimal strategy based on alert severity
- Coordinates with risk management for position sizing

### `strategies.rs`
- Implements specific trading strategies:
  - `EmergencySell`: Immediate liquidation for critical threats
  - `PartialExit`: Gradual position reduction
  - `Arbitrage`: Profit from price discrepancies
  - `LiquidityProvision`: Add liquidity to stabilize pools
  - `MonitorOnly`: Track without action

### `analyzer.rs`
- Calculates profitability of potential trades
- Estimates gas costs and slippage
- Analyzes market impact of trades
- Provides risk/reward assessments

## Strategy Decision Matrix

| Alert Type | Liquidity Loss | Pool Size | Strategy | Max Slippage |
|------------|----------------|-----------|----------|--------------|
| Critical | >80% | Any | EmergencySell | 30% |
| Critical | 50-80% | >10 ETH | EmergencySell | 20% |
| High | 30-50% | >5 ETH | PartialExit | 15% |
| Medium | 20-30% | Any | Arbitrage/Monitor | 10% |

## Strategy Types

### Emergency Sell
```rust
pub struct EmergencySellStrategy {
    pub urgency: Priority::Critical,
    pub sell_percentage: 1.0, // 100%
    pub max_slippage: 0.30,   // 30%
    pub gas_multiplier: 3.0,   // 3x base gas
}
```

### Partial Exit
```rust
pub struct PartialExitStrategy {
    pub urgency: Priority::High,
    pub sell_percentage: 0.75, // 75%
    pub max_slippage: 0.15,   // 15%
    pub gas_multiplier: 2.0,   // 2x base gas
}
```

### Arbitrage
```rust
pub struct ArbitrageStrategy {
    pub min_profit_eth: 0.1,
    pub max_hops: 3,
    pub include_cex: false,
}
```

## Configuration
- `emergency_threshold`: Liquidity loss % triggering emergency sell (default: 80%)
- `high_risk_threshold`: Liquidity loss % for high priority (default: 50%)
- `min_pool_size_eth`: Minimum pool size to consider (default: 0.5 ETH)
- `max_slippage_emergency`: Maximum acceptable slippage for emergency (default: 30%)

## Usage Example
```rust
use eth_kartal::strategy::{DecisionEngine, ScamAlert};

let engine = DecisionEngine::new(config);

// Process alert
let alert = ScamAlert { /* ... */ };
let strategy = engine.decide_strategy(&alert).await?;

match strategy {
    Strategy::EmergencySell(params) => {
        // Execute immediate sell
        tx_executor.emergency_sell(params).await?;
    },
    Strategy::PartialExit(params) => {
        // Execute gradual exit
        tx_executor.partial_exit(params).await?;
    },
    Strategy::MonitorOnly => {
        // Log and continue monitoring
    }
}
```

## Integration Points
- **Input**: `ScamAlert` from alert processor
- **Output**: Selected `Strategy` to transaction executor
- **Dependencies**: Risk manager for position limits, market data feeds

## Decision Factors
1. **Alert Severity**: Critical > High > Medium > Low
2. **Liquidity Impact**: Percentage of pool being drained
3. **Pool Size**: Larger pools may warrant different strategies
4. **Gas Costs**: Must be profitable after gas
5. **Market Conditions**: Network congestion, volatility
6. **Position Size**: Current exposure to token
7. **Historical Performance**: Past success with similar alerts

## Performance Requirements
- Strategy decision: <50ms
- Profitability calculation: <20ms
- Risk assessment: <10ms
- Total processing: <100ms per alert

## Safety Features
- Never exceed position limits
- Always simulate before execution
- Respect slippage tolerances
- Circuit breaker integration
- Profit validation before trade