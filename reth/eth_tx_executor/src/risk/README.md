# Risk Management Module

This module handles risk assessment for transaction execution in eth_kartal. It evaluates whether trades should proceed or be blocked based on essential risk factors.

## Core Risk Scenarios

### Scenario 1: Buy 0.01 ETH Worth of Token

**Flow**:
1. Alert arrives: "Buy TOKEN with 0.01 ETH"
2. Check wallet balance: 0.05 ETH available
3. Estimate gas cost: 0.0002 ETH
4. Total required: 0.0102 ETH
5. **Decision**: ALLOW (sufficient funds)

**What we track**:
- Available balance vs required amount
- Gas cost estimation
- Total ETH needed

### Scenario 2: Sell Tokens Back to ETH

**Flow**:
1. Alert arrives: "Sell 1000 TOKEN for ETH"
2. Check token balance: 800 TOKEN available
3. **Decision**: BLOCK (insufficient tokens)

**What we track**:
- Token balance vs sell amount
- Clear reason for blocking

### Scenario 3: High Gas Price Environment

**Flow**:
1. Alert arrives: "Buy TOKEN with 0.01 ETH"
2. Check gas price: 200 gwei (high congestion)
3. Estimate gas cost: 0.003 ETH (30% of trade value!)
4. **Decision**: BLOCK (gas cost exceeds threshold)

**What we track**:
- Gas cost as percentage of trade
- Configurable gas threshold

### Scenario 4: Slippage Protection

**Flow**:
1. Alert arrives: "Buy TOKEN, expect 1000 tokens"
2. Get quote: Only 850 tokens available (15% slippage)
3. Max acceptable slippage: 5%
4. **Decision**: BLOCK (excessive slippage)

**What we track**:
- Expected vs actual output
- Slippage percentage
- Configured tolerance


## Risk Factors We Consider

### 1. **Fund Availability**
- Do we have enough ETH/tokens for the trade?
- Do we have enough ETH for gas costs?
- Buffer for failed transactions?

### 2. **Transaction Cost**
- Is gas price reasonable for current market?
- Does gas cost exceed a percentage of trade value?
- Should we wait for better gas prices?

### 3. **Market Conditions**
- Is slippage within acceptable bounds?
- Is there enough liquidity?
- Are we getting a fair price?

## Risk Decision Types

```rust
pub enum RiskDecision {
    Allow,                    // Proceed with trade
    Block { reason: String }  // Do not execute
}
```

Note: We intentionally keep this simple. No partial executions or complex adjustments.

## Configuration

Risk parameters that can be configured:

```rust
pub struct RiskConfig {
    // Maximum gas cost as percentage of trade value
    pub max_gas_cost_percent: f64,  // e.g., 5% max
    
    // Maximum acceptable slippage
    pub max_slippage_percent: f64,  // e.g., 3% max
    
    // Minimum ETH balance to maintain (for gas)
    pub min_eth_balance: f64,       // e.g., 0.01 ETH
}
```

## What We DON'T Track

1. **Portfolio metrics** - No position sizing or exposure limits
2. **Historical performance** - No P&L tracking or daily limits
3. **Complex market analysis** - No volatility or correlation checks
4. **Time-based restrictions** - No trading hours or cooldowns

## Integration with Executor

The risk manager is called before transaction submission:

```rust
// In executor.rs
let risk_decision = self.risk_manager.evaluate_trade(
    eth_balance,
    required_amount,
    estimated_gas_cost,
    token_address,
);

match risk_decision {
    RiskDecision::Allow => {
        // Proceed with transaction
    }
    RiskDecision::Block { reason } => {
        // Log reason and return error
        return Err(format!("Trade blocked: {}", reason));
    }
}
```

## Monitoring

Key metrics to track:
- Block rate by reason (insufficient funds, high gas, etc.)
- Average gas costs
- Slippage events
- Failed transactions that passed risk checks

This helps identify if risk parameters need adjustment.