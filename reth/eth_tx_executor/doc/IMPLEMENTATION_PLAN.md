# ETH Kartal Implementation Plan

## Current State Assessment

### What We Have
1. **Mempool Processor**: 
   - Successfully detects scam transactions with <1ms latency
   - Logs detailed alerts to `market_events_full_tx_*.log`
   - Publishes alerts via ZMQ (not yet implemented)

2. **Detected Patterns**:
   - **Critical**: 100% liquidity drains (complete pool drainage)
   - **High Risk**: 50-80% liquidity removal
   - **Warning**: 20-50% liquidity changes
   - Average detection latency: 0.2-2.3ms via IPC

3. **ETH Kartal Repository**:
   - Basic project structure initialized
   - Development plan exists but no implementation
   - Multiple branches created but empty

### What We Need
1. **Alert Reception**: ZMQ subscriber to receive scam alerts
2. **Strategy Engine**: Decision logic for protective actions
3. **Transaction Executor**: Fast transaction submission
4. **Risk Management**: Position sizing and safety controls

## Priority Implementation Order

### Step 1: Alert Integration (Critical Path)
**Goal**: Establish communication between mempool processor and eth_kartal

#### 1.1 Add ZMQ Publisher to Mempool Processor
```rust
// In mempool_processor/src/signal_engine/publisher.rs
pub struct AlertPublisher {
    socket: zmq::Socket,
    endpoint: String,
}

impl AlertPublisher {
    pub async fn publish_alert(&self, event: &MarketEvent) -> Result<()> {
        let alert = ScamAlert::from_market_event(event);
        let json = serde_json::to_string(&alert)?;
        self.socket.send(&json, 0)?;
        Ok(())
    }
}
```

#### 1.2 Create Alert Receiver in ETH Kartal
```rust
// In eth_kartal/src/alert_processor/receiver.rs
pub struct AlertReceiver {
    socket: zmq::Socket,
    tx: mpsc::Sender<ScamAlert>,
}

impl AlertReceiver {
    pub async fn start_listening(&mut self) -> Result<()> {
        loop {
            let msg = self.socket.recv_string(0)??;
            let alert: ScamAlert = serde_json::from_str(&msg)?;
            self.tx.send(alert).await?;
        }
    }
}
```

### Step 2: Basic Transaction Execution
**Goal**: Execute simple sell transactions for detected scams

#### 2.1 Transaction Builder
```rust
// In eth_kartal/src/tx_executor/builder.rs
pub struct TransactionBuilder {
    router_address: Address,
    wallet_address: Address,
}

impl TransactionBuilder {
    pub fn build_emergency_sell(
        &self,
        token: Address,
        amount: U256,
        min_output: U256,
    ) -> TransactionRequest {
        // Build swap transaction for Uniswap V2/V3
        let data = self.encode_swap_exact_tokens_for_eth(
            amount,
            min_output,
            vec![token, WETH_ADDRESS],
            self.wallet_address,
            deadline(),
        );
        
        TransactionRequest::new()
            .to(self.router_address)
            .data(data)
            .value(0)
    }
}
```

#### 2.2 Transaction Submitter
```rust
// In eth_kartal/src/tx_executor/submitter.rs
pub struct TransactionSubmitter {
    provider: Provider<Http>,
    wallet: LocalWallet,
}

impl TransactionSubmitter {
    pub async fn submit_transaction(
        &self,
        tx_request: TransactionRequest,
        gas_config: GasConfig,
    ) -> Result<TxHash> {
        // Add gas parameters
        let tx = tx_request
            .gas(gas_config.gas_limit)
            .gas_price(gas_config.gas_price);
            
        // Sign and send
        let pending_tx = self.provider
            .send_transaction(tx, None)
            .await?;
            
        Ok(pending_tx.tx_hash())
    }
}
```

### Step 3: Strategy Engine
**Goal**: Make intelligent decisions based on alert severity

#### 3.1 Strategy Types
```rust
// In eth_kartal/src/strategy/types.rs
pub enum TradingStrategy {
    EmergencySell {
        token: Address,
        amount: U256,
        max_slippage: f64,
    },
    PartialExit {
        token: Address,
        percentage: f64,
        max_slippage: f64,
    },
    MonitorOnly {
        reason: String,
    },
}
```

#### 3.2 Decision Engine
```rust
// In eth_kartal/src/strategy/decision.rs
pub struct DecisionEngine {
    risk_params: RiskParameters,
}

impl DecisionEngine {
    pub fn decide_strategy(&self, alert: &ScamAlert) -> TradingStrategy {
        match (alert.severity, alert.eth_change_percent) {
            (Severity::Critical, p) if p < -0.8 => {
                TradingStrategy::EmergencySell {
                    token: alert.token_address,
                    amount: self.get_token_balance(&alert.token_address),
                    max_slippage: 0.3, // Accept 30% slippage
                }
            },
            (Severity::High, p) if p < -0.5 => {
                TradingStrategy::PartialExit {
                    token: alert.token_address,
                    percentage: 0.75,
                    max_slippage: 0.15,
                }
            },
            _ => TradingStrategy::MonitorOnly {
                reason: "Below action threshold".to_string(),
            },
        }
    }
}
```

### Step 4: Risk Management
**Goal**: Prevent losses from failed trades or system errors

#### 4.1 Position Manager
```rust
// In eth_kartal/src/risk/position.rs
pub struct PositionManager {
    max_position_eth: f64,
    current_positions: HashMap<Address, Position>,
}

impl PositionManager {
    pub fn can_trade(&self, token: &Address, eth_value: f64) -> bool {
        let current_exposure = self.get_total_exposure();
        current_exposure + eth_value <= self.max_position_eth
    }
}
```

#### 4.2 Circuit Breaker
```rust
// In eth_kartal/src/risk/circuit_breaker.rs
pub struct CircuitBreaker {
    max_daily_loss: f64,
    max_failures: u32,
    state: CircuitState,
}

impl CircuitBreaker {
    pub fn check_can_execute(&self) -> Result<(), CircuitBreakerError> {
        match self.state {
            CircuitState::Open => Err(CircuitBreakerError::CircuitOpen),
            CircuitState::Closed => Ok(()),
        }
    }
}
```

## Development Timeline

### Week 1: Foundation
- [ ] Set up project structure
- [ ] Implement ZMQ alert publisher in mempool processor
- [ ] Create alert receiver in eth_kartal
- [ ] Add basic logging and configuration

### Week 2: Core Execution
- [ ] Implement transaction builder for Uniswap V2
- [ ] Add transaction submitter with gas optimization
- [ ] Create wallet management
- [ ] Test on testnet

### Week 3: Strategy & Intelligence
- [ ] Build decision engine
- [ ] Add strategy types
- [ ] Implement profitability calculations
- [ ] Add simulation before execution

### Week 4: Risk & Safety
- [ ] Add position management
- [ ] Implement circuit breakers
- [ ] Add comprehensive error handling
- [ ] Create monitoring dashboard

### Week 5: Testing & Optimization
- [ ] Integration testing with mempool processor
- [ ] Performance optimization
- [ ] Mainnet fork testing
- [ ] Security review

## Quick Start Commands

```bash
# 1. Update mempool processor to publish alerts
cd /home/nima/code/crypto/rust/mempool_processor
cargo build --bin mempool_signal_detection_full_tx_ipc

# 2. Build eth_kartal alert receiver
cd /home/nima/code/crypto/rust/eth_kartal
cargo build --bin kartal_receiver

# 3. Run integrated system
# Terminal 1: Mempool processor with ZMQ publishing
cargo run --bin mempool_signal_detection_full_tx_ipc -- --enable-publisher

# Terminal 2: ETH Kartal receiver
cargo run --bin kartal_receiver -- --zmq-endpoint tcp://localhost:5558

# 4. Monitor logs
tail -f /home/nima/code/crypto/logs/mempool/market_events_*.log
tail -f /home/nima/code/crypto/logs/kartal/executions_*.log
```

## Testing Strategy

### 1. Unit Tests
- Alert parsing and validation
- Transaction building correctness
- Strategy decision logic
- Risk calculations

### 2. Integration Tests
- End-to-end alert flow
- Transaction simulation accuracy
- Multi-component coordination

### 3. Mainnet Fork Tests
- Real pool interactions
- Gas estimation accuracy
- Profitability validation

### 4. Performance Tests
- Alert processing latency
- Transaction submission speed
- Concurrent alert handling

## Configuration Template

```toml
# config/kartal.toml
[alerts]
zmq_endpoint = "tcp://localhost:5558"
max_queue_size = 1000

[wallet]
private_key_env = "ETH_KARTAL_PRIVATE_KEY"
max_gas_price_gwei = 300

[strategies]
emergency_sell_threshold = -0.8  # -80% liquidity
partial_exit_threshold = -0.5    # -50% liquidity
min_profit_eth = 0.05

[risk]
max_position_eth = 10.0
max_daily_loss_eth = 5.0
circuit_breaker_enabled = true

[execution]
slippage_tolerance = 0.02  # 2% default
gas_price_multiplier = 1.2
use_flashbots = true
```

## Next Immediate Steps

1. **Create feature branch**: `git checkout -b feature/alert-receiver`
2. **Add ZMQ to mempool processor dependencies**
3. **Implement basic alert publisher**
4. **Create alert receiver skeleton in eth_kartal**
5. **Test alert flow end-to-end**

This implementation plan provides a clear path from our current detection-only system to a fully automated trading protection system.