# Signal Detectors - Simplified Signal System

## Overview

This module implements a **simplified binary signal system** that replaces complex risk scoring with clear, actionable signals based on configurable thresholds. Each detector emits simple YES/NO signals when specific conditions are met.

## Design Philosophy

- **No risk scores** - only binary signals (detected/not detected)
- **Clear thresholds** - all parameters configurable in `/src/config.rs`
- **Simple logic** - straightforward if/then conditions
- **Actionable signals** - each signal has a clear meaning and response

## Signal Types

We detect 4 simple binary signals:

### 1. Trading Enabled Signal

**Purpose**: Detect when a token becomes tradeable with reasonable taxes

**Trigger Conditions**:
- Trading enabled function detected (`enableTrading`, `openTrading`, etc.)
- Buy tax ≤ `max_acceptable_buy_tax` (default: 30%)
- Sell tax ≤ `max_acceptable_sell_tax` (default: 30%)
- Both buy and sell transactions succeed in simulation

**Signal Data**:
```rust
pub struct TradingEnabledSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub creator_address: String,
    pub buy_tax: u8,
    pub sell_tax: u8,
    pub timestamp: u64,
    pub block_number: u64,
}
```

**Configuration Parameters** (in `config.rs`):
- `tax_detection.max_acceptable_buy_tax` (default: 30%)
- `tax_detection.max_acceptable_sell_tax` (default: 30%)

### 2. High Tax Warning Signal

**Purpose**: Alert when a token has excessive taxes that indicate potential honeypot

**Trigger Conditions**:
- Buy tax > `max_acceptable_buy_tax` OR
- Sell tax > `max_acceptable_sell_tax`

**Signal Data**:
```rust
pub struct HighTaxWarningSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub creator_address: Option<String>,
    pub buy_tax: u8,
    pub sell_tax: u8,
    pub warning_type: TaxWarningType, // HighBuyTax | HighSellTax | PotentialHoneypot
    pub timestamp: u64,
    pub block_number: u64,
}

pub enum TaxWarningType {
    HighBuyTax,
    HighSellTax, 
    PotentialHoneypot, // sell tax > honeypot_sell_threshold
}
```

**Configuration Parameters** (in `config.rs`):
- `tax_detection.max_acceptable_buy_tax` (default: 30%)
- `tax_detection.max_acceptable_sell_tax` (default: 30%)
- `tax_detection.honeypot_sell_threshold` (default: 50%)

### 3. Liquidity Removal Signal

**Purpose**: Detect when liquidity is being removed from a pool

**Trigger Conditions**:
- Liquidity removal function detected (`removeLiquidity*`, `decreaseLiquidity`, etc.)
- Pool has minimum ETH value > `min_pool_eth` (default: 0.7 ETH)

**Signal Data**:
```rust
pub struct LiquidityRemovalSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub token_address: Option<String>,
    pub remover_address: String,
    pub function_name: String,
    pub estimated_eth_removed: Option<f64>,
    pub timestamp: u64,
    pub block_number: u64,
}
```

**Configuration Parameters** (in `config.rs`):
- `signal_detection.min_pool_eth` (default: 0.7 ETH)

### 4. Scam Detection Signal (TO BE IMPLEMENTED)

**Purpose**: Detect when a transaction drains significant liquidity from a tracked pool

**Trigger Conditions**:
- Transaction simulation shows state changes
- One or more addresses in state changes match tracked pools (from TokenCache)
- Pool ETH balance after transaction:
  - Drops by more than 60% (configurable) OR
  - Falls below 0.3 ETH (configurable)

**Signal Data**:
```rust
pub struct ScamDetectionSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub token_address: String,
    pub scammer_address: String,
    pub eth_drained: f64,
    pub eth_remaining: f64,
    pub drain_percentage: f64,
    pub timestamp: u64,
    pub block_number: u64,
}
```

**Configuration Parameters** (in `config.rs`):
- `scam_detection.drain_percentage_threshold` (default: 60%)
- `scam_detection.min_eth_remaining` (default: 0.3 ETH)

**Implementation Notes**:
- Requires transaction simulation to get state changes
- Cross-references state change addresses with TokenCache pool addresses
- Binary signal: scam detected (meets threshold) or not detected

## Signal Detection Flow

```
1. Transaction arrives via IPC
   ↓
2. FunctionDetector identifies function signatures
   ↓
3. TransactionRouter categorizes transaction
   ↓
4. If creator/owner transaction → Run simulation sequence:
   a. Simulate creator transaction → state changes
   b. Simulate buy transaction → state changes
   c. Simulate approve transaction → state changes
   d. Simulate sell transaction → state changes
   ↓
5. Pass simulation results to signal detectors:
   - LiquidityDetector uses creator tx state changes
   - TaxDetector uses buy/sell state changes
   - TradingEnabledDetector uses buy/sell success
   - ScamDetector uses all state changes
   ↓
6. If conditions met → Emit binary signal
   ↓
7. Publish to ZMQ + Log to file
```

## Simulation Sequence

For every creator/owner transaction, we run a full simulation sequence using the BuySellSimulator:

### The Standard Sequence:
1. **Creator Transaction** - Simulate the actual transaction from creator
   - State changes show liquidity operations, parameter updates
   
2. **Buy Transaction** - Simulate a buy from a test wallet
   - State changes show token balance increase, ETH decrease
   - Used to calculate buy tax
   
3. **Approve Transaction** - Simulate approval for DEX router
   - Required for ERC20 tokens to allow router to spend tokens
   - State changes show approval set
   
4. **Sell Transaction** - Simulate selling the bought tokens
   - State changes show token balance decrease, ETH increase  
   - Used to calculate sell tax

### Why This Sequence?

- **Approve is always required** for ERC20 tokens before selling on DEX
- Running the full sequence gives us complete picture of token behavior
- Single simulation run provides data for all detectors
- State changes from each step reveal different aspects:
  - Creator tx → liquidity/parameter changes
  - Buy/Sell → tax rates and trading status
  - All combined → scam detection

## Detector Implementations

### How Detectors Use Simulation Results

Each detector receives specific parts of the simulation sequence:

#### TradingEnabledDetector
- **Input**: Buy/Sell simulation results
- **Checks**: Both buy and sell succeed, taxes within limits
- **Output**: Trading enabled signal

```rust
pub fn detect(
    &self,
    buy_result: &SimulationResult,
    sell_result: &SimulationResult,
) -> Option<TradingEnabledSignal>
```

#### HighTaxDetector
- **Input**: Buy/Sell simulation results
- **Checks**: Tax rates from token balance changes
- **Output**: High tax warning if above threshold

```rust
pub fn detect(
    &self,
    buy_result: &SimulationResult,
    sell_result: &SimulationResult,
) -> Option<HighTaxWarningSignal>
```

#### LiquidityDetector
- **Input**: Creator transaction simulation result
- **Checks**: ETH balance changes in pool addresses
- **Output**: Liquidity removal signal

```rust
pub fn detect(
    &self,
    creator_tx_result: &SimulationResult,
) -> Option<LiquidityRemovalSignal>
```

#### ScamDetector
- **Input**: All simulation results (full sequence)
- **Checks**: Pool drain across all state changes
- **Output**: Scam detection signal

```rust
pub fn detect(
    &self,
    simulation_sequence: &SimulationSequence,
) -> Option<ScamDetectionSignal>
```

## Signal Output

### ZMQ Publishing

All signals are published to ZMQ endpoints:
- **Endpoint**: `tcp://127.0.0.1:5556` (configurable)
- **Format**: JSON-serialized signal structs
- **Topics**: Signal type (e.g., "trading_enabled", "high_tax", "liquidity_removal")

### File Logging

Signals are logged to dedicated files:
- `/home/nima/code/crypto/logs/mempool/dev/trading_enabled.log`
- `/home/nima/code/crypto/logs/mempool/dev/high_tax_warnings.log`
- `/home/nima/code/crypto/logs/mempool/dev/liquidity_removals.log`
- `/home/nima/code/crypto/logs/mempool/dev/scam_detections.log`

### Log Format

```
[2025-07-27 14:45:36.123] TRADING_ENABLED | Token: 0x123... | Creator: 0xabc... | BuyTax: 5% | SellTax: 10% | TxHash: 0x456...
[2025-07-27 14:45:42.456] HIGH_TAX | Token: 0x789... | SellTax: 75% | Type: PotentialHoneypot | TxHash: 0xdef...
[2025-07-27 14:45:48.789] LIQUIDITY_REMOVAL | Pool: 0x111... | Function: removeLiquidityETH | Remover: 0x222... | TxHash: 0x333...
[2025-07-27 14:45:52.012] SCAM_DETECTED | Pool: 0x444... | Token: 0x555... | Scammer: 0x666... | Drained: 5.2 ETH (85%) | Remaining: 0.9 ETH | TxHash: 0x777...
```

## Configuration

All detector thresholds are centralized in `/src/config.rs`:

```rust
pub struct TaxDetectionConfig {
    pub enabled: bool,
    pub max_acceptable_buy_tax: u8,      // 25% - signals above this
    pub max_acceptable_sell_tax: u8,     // 25% - signals above this  
    pub honeypot_sell_threshold: u8,     // 50% - potential honeypot
    // ... other settings
}

pub struct SignalDetectionConfig {
    pub min_pool_eth: f64,               // 0.7 - minimum pool size to track
    // ... other settings
}
```

## Testing Strategy

### Unit Tests
- Test each detector with known transaction patterns
- Verify threshold boundary conditions
- Test edge cases (zero taxes, contract creation, etc.)

### Integration Tests  
- Full pipeline test with mock IPC transactions
- Verify signal publishing to ZMQ
- Test configuration loading and overrides

### Test Data
Create test transactions for each signal type:
- `test_trading_enabled_low_tax.json` - should trigger signal
- `test_trading_enabled_high_tax.json` - should NOT trigger signal
- `test_honeypot_tax.json` - should trigger high tax warning
- `test_liquidity_removal.json` - should trigger liquidity signal

## Performance Requirements

- **Latency**: < 1ms per detector check
- **Memory**: Minimal state - detectors should be stateless
- **CPU**: Simple threshold comparisons only
- **Throughput**: Handle 1000+ transactions/second

## Error Handling

- **Invalid simulation results**: Skip signal detection, log warning
- **Missing tax data**: Skip tax-based signals, log debug
- **ZMQ publish failure**: Log error, continue processing
- **Configuration errors**: Fail fast on startup

## Migration from Current System

1. **Replace** complex HoneypotDetector with simple HighTaxDetector
2. **Replace** TradingStatusDetector with TradingEnabledDetector  
3. **Simplify** LiquidityDetector to LiquidityRemovalDetector
4. **Remove** risk scoring, confidence calculations, pattern matching
5. **Keep** function detection and simulation integration

## Future Extensions

Additional simple signals can be added:
- **Contract Creation Signal**: New token contracts with specific patterns
- **Volume Spike Signal**: Unusual trading volume detected
- **Price Manipulation Signal**: Suspicious price movements
- **Blacklist Signal**: Interaction with known malicious addresses

Each new signal follows the same pattern:
1. Clear trigger conditions
2. Simple threshold-based logic
3. Binary output (detected/not detected)
4. Configurable parameters in `config.rs`