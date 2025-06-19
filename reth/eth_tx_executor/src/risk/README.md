# Risk Management Module

## Overview
The Risk Management module provides safety controls and monitoring to prevent losses from system errors, market volatility, or adversarial conditions. It enforces position limits, implements circuit breakers, and tracks performance metrics.

## Components

### `position_manager.rs`
- Tracks current token positions and exposure
- Enforces maximum position sizes
- Calculates risk-adjusted position limits
- Manages portfolio diversification rules

### `circuit_breaker.rs`
- Implements safety mechanisms to halt trading
- Monitors loss thresholds and failure rates
- Provides cooldown periods after incidents
- Enables emergency system shutdown

### `metrics.rs`
- Tracks execution performance and profitability
- Monitors system health indicators
- Generates risk reports and alerts
- Provides real-time dashboard data

## Risk Controls

### Position Limits
```rust
pub struct PositionLimits {
    pub max_position_eth: f64,           // 10 ETH
    pub max_position_usd: f64,           // $25,000
    pub max_percent_of_pool: f64,        // 5%
    pub max_tokens_held: u32,            // 20
    pub concentration_limit: f64,        // 25% per token
}
```

### Circuit Breaker Rules
```rust
pub struct CircuitBreakerRules {
    pub max_daily_loss_eth: f64,         // 5 ETH
    pub max_consecutive_failures: u32,    // 5
    pub max_hourly_transactions: u32,     // 100
    pub cooldown_period_seconds: u64,     // 300 (5 min)
    pub emergency_shutdown_loss: f64,     // 10 ETH
}
```

### Risk Scoring
```rust
pub struct TokenRiskScore {
    pub liquidity_score: f64,      // 0-1 (1 = high liquidity)
    pub volatility_score: f64,     // 0-1 (1 = low volatility)
    pub contract_risk: f64,        // 0-1 (1 = verified safe)
    pub historical_performance: f64, // 0-1 (1 = profitable)
    pub overall_risk: f64,         // Weighted average
}
```

## Safety Features

### Pre-Trade Checks
1. **Position Size**: Ensure trade doesn't exceed limits
2. **Loss Limits**: Check daily/hourly loss thresholds
3. **System Health**: Verify all components operational
4. **Market Conditions**: Check for unusual volatility
5. **Circuit Status**: Ensure breakers not tripped

### Post-Trade Monitoring
1. **Execution Quality**: Compare actual vs expected
2. **Slippage Tracking**: Monitor price impact
3. **P&L Calculation**: Track profitability
4. **Risk Metrics Update**: Adjust scores based on results

## Risk Metrics

### Real-Time Metrics
- Current exposure (ETH and USD)
- Number of open positions
- Daily P&L
- Win/loss ratio
- Average slippage
- Gas efficiency

### Historical Analysis
- 24h/7d/30d performance
- Best/worst trades
- Risk-adjusted returns
- Maximum drawdown
- Sharpe ratio

## Usage Example
```rust
use eth_kartal::risk::{RiskManager, TradeRequest};

let risk_manager = RiskManager::new(config);

// Check if trade is allowed
let trade_request = TradeRequest {
    token: token_address,
    amount_eth: 2.5,
    strategy: Strategy::EmergencySell,
};

match risk_manager.evaluate_trade(&trade_request) {
    RiskDecision::Approved => {
        // Proceed with trade
    },
    RiskDecision::Rejected(reason) => {
        log::warn!("Trade rejected: {}", reason);
    },
    RiskDecision::Modified(new_params) => {
        // Use modified parameters (e.g., reduced size)
    },
}

// Check circuit breaker status
if risk_manager.is_circuit_breaker_open() {
    log::error!("Circuit breaker is open - trading halted");
    return;
}
```

## Circuit Breaker States

### States
1. **Closed** (Normal): All systems operational
2. **Half-Open** (Caution): Limited trading allowed
3. **Open** (Halted): No trading permitted
4. **Emergency** (Shutdown): System disabled

### State Transitions
```
Closed → Half-Open: After minor incidents
Half-Open → Closed: After successful recovery
Half-Open → Open: After continued failures
Open → Half-Open: After cooldown period
Any → Emergency: After catastrophic loss
```

## Integration Points
- **Input**: Trade requests from strategy engine
- **Output**: Risk decisions (approved/rejected/modified)
- **Dependencies**: Position tracking, P&L calculation, system monitors

## Configuration
```toml
[risk]
max_position_eth = 10.0
max_daily_loss_eth = 5.0
max_consecutive_failures = 5
circuit_breaker_enabled = true
emergency_contact = "alerts@example.com"

[risk.position_limits]
max_percent_of_pool = 0.05
concentration_limit = 0.25
max_tokens_held = 20

[risk.monitoring]
alert_on_loss_percent = 0.1
performance_log_interval = 60
dashboard_update_frequency = 5
```

## Alerts and Notifications
- **Critical**: Circuit breaker triggered, emergency shutdown
- **High**: Approaching loss limits, multiple failures
- **Medium**: Unusual slippage, degraded performance
- **Low**: Position concentration warnings

## Performance Impact
- Risk checks add <5ms to trade execution
- Continuous monitoring uses minimal resources
- Historical data kept for 30 days
- Real-time metrics updated every second