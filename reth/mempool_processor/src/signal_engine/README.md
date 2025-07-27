# Signal Engine

The Signal Engine is a simplified binary signal detection system that identifies clear, actionable market events in Ethereum mempool transactions. It focuses on three critical signals with straightforward threshold-based detection.

## Overview

The signal detection system provides **binary signals** (detected/not detected) through:

1. **Function Detection**: Identifies specific function calls in transactions
2. **Transaction Routing**: Categorizes transactions and assigns simulation priorities
3. **Signal Processing**: Coordinates detection pipeline with TokenCache integration
4. **Binary Signal Detection**: Three simple signals with clear thresholds
5. **Signal Publishing**: Publishes signals via ZMQ and log files

## Three Binary Signals

### 1. Trading Enabled Signal
```
Mempool → enableTrading() detected → TokenCache: trading_status = true
         ↓
         Simulate buy/sell → Extract taxes → Check thresholds
         ↓
         If taxes ≤30% AND simulation succeeds → TRADING_ENABLED_SIGNAL
```

### 2. High Tax Warning Signal  
```
Mempool → Any transaction → Simulate buy/sell → Extract taxes
         ↓
         If buy_tax >30% OR sell_tax >30% → HIGH_TAX_WARNING_SIGNAL
```

### 3. Liquidity Removal Signal
```
Mempool → removeLiquidity() detected → Check pool has ≥0.7 ETH
         ↓
         If threshold met → LIQUIDITY_REMOVAL_SIGNAL
```

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Reth Node     │────▶│  IPC Client      │────▶│ Function        │
│   (Mempool)     │     │  (NonBlocking)   │     │ Detector        │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                           │
                                                           ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  TX Router      │────▶│ Signal Processor │────▶│ Signal          │
│ (Classification)│     │ (Coordinator)    │     │ Generator       │
└─────────────────┘     └──────────────────┘     └─────────────────┘
           │                       │                       │
           ▼                       ▼                       ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Token Cache    │     │ Signal Detectors │     │   Publishers    │
│ (Trading Status)│     │ (Binary Signals) │     │  (ZMQ/Logs)     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## SignalProcessor - The Main Coordinator

The **SignalProcessor** is the central component that coordinates the entire signal detection pipeline:

### Key Responsibilities:
1. **Transaction Routing**: Uses TransactionRouter to classify transactions
2. **Trading Status**: Queries TokenCache for `trading_status: bool`
3. **Simulation Coordination**: Triggers buy/sell simulation when needed
4. **Context Passing**: Provides signal detectors with all necessary context
5. **Signal Collection**: Aggregates binary signals from all detectors
6. **Publishing**: Sends signals to SignalGenerator for formatting

### Processing Flow:
```rust
impl SignalProcessor {
    pub async fn process_transaction(&self, tx: MempoolTransaction) -> Result<Vec<Signal>> {
        // 1. Route transaction to get classification
        let routing = self.tx_router.classify(&tx).await;
        
        // 2. Extract token address from routing or transaction
        let token_address = self.extract_token_address(&tx, &routing).await;
        
        // 3. Query TokenCache for trading status
        let trading_status = if let Some(token_addr) = &token_address {
            self.token_cache.is_trading_enabled(token_addr).await.unwrap_or(false)
        } else {
            false
        };
        
        // 4. Run buy/sell simulation if needed
        let simulation_result = if routing.requires_simulation {
            self.buy_sell_simulator.simulate(&tx).await
        } else { None };
        
        // 5. Check all signal detectors with context
        let mut signals = Vec::new();
        
        // TradingEnabledDetector - only triggers if trading_status == true
        if trading_status && simulation_result.is_some() {
            if let Some(signal) = self.check_trading_enabled(&tx, &sim_result) {
                signals.push(Signal::TradingEnabled(signal));
            }
        }
        
        // HighTaxDetector - triggers on high taxes regardless of trading status
        if simulation_result.is_some() {
            if let Some(signal) = self.check_high_tax(&tx, &sim_result) {
                signals.push(Signal::HighTaxWarning(signal));
            }
        }
        
        // LiquidityRemovalDetector - no simulation needed
        if let Some(signal) = self.check_liquidity_removal(&tx) {
            signals.push(Signal::LiquidityRemoval(signal));
        }
        
        // 6. Send signals to generator for publishing
        for signal in &signals {
            self.signal_generator.process_and_publish(signal).await;
        }
        
        Ok(signals)
    }
}
```

### Binary Signal Detection Logic:

#### 1. Trading Enabled Signal
- **Trigger Condition**: `trading_status == true` AND `simulation_result.can_buy && simulation_result.can_sell` AND `taxes ≤ 30%`
- **Purpose**: Confirms token is tradeable with reasonable taxes
- **Output**: `TradingEnabledSignal { token_address, buy_tax, sell_tax, creator_address, ... }`

#### 2. High Tax Warning Signal  
- **Trigger Condition**: `buy_tax > 30%` OR `sell_tax > 30%`
- **Purpose**: Warns of excessive taxes that may indicate honeypot
- **Output**: `HighTaxWarningSignal { warning_type: HighBuyTax | HighSellTax | PotentialHoneypot, ... }`

#### 3. Liquidity Removal Signal
- **Trigger Condition**: Liquidity removal function detected AND pool has ≥ 0.7 ETH
- **Purpose**: Alerts on potential rug pulls
- **Output**: `LiquidityRemovalSignal { pool_address, function_name, remover_address, ... }`

### Configuration Integration:
```rust
// Tax thresholds from config.rs
max_acceptable_buy_tax: 30%     // Trading enabled threshold
max_acceptable_sell_tax: 30%    // Trading enabled threshold  
min_pool_eth: 0.7 ETH          // Liquidity removal threshold
```

## Components

### 1. Transaction Classifier
Categorizes incoming transactions:
- Contract Creation
- Creator Transaction (from tracked addresses)
- DEX Interaction
- Unknown/Regular Transfer

### 2. Function Detector (`function_detector.rs`)

The FunctionDetector is a real-time transaction categorization system that identifies specific function calls by analyzing 4-byte function selectors.

#### Key Features

- **Detection Latency**: Average 0.005ms, Maximum 0.256ms
- **Throughput**: 600-700 transactions per minute
- **Three Specialized Detectors**:
  - `LiquidityRemovalDetector`: 11 DEX liquidity removal functions
  - `TradingEnabledDetector`: 4 trading activation functions
  - `SwapDetector`: 24 swap functions from major protocols

#### Function Signatures Detected

**Liquidity Removals (11 signatures):**
- Uniswap V2: `removeLiquidityETH`, `removeLiquidity`, `removeLiquidityETHSupportingFeeOnTransferTokens`
- Uniswap V3: `decreaseLiquidity`
- Balancer: `exitPool`
- Curve: `remove_liquidity`, `remove_liquidity_one_coin`, `remove_liquidity_imbalance`

**Trading Enabled (4 signatures):**
- `setTradingEnabled` (0x8a8c523c)
- `enableTrading` (0x8ee88c53) 
- `openTrading` (0xc9567bf9)
- `startTrading` (0xfb201b1d)

**Swaps (24 signatures):**
- Uniswap V2/V3: 12 functions
- 1inch: 3 functions
- 0x Protocol: 2 functions
- Curve: 2 exchange functions
- Balancer: 2 swap functions

### 3. Creator Tracker (TokenCreatorCache)

Maintains real-time cache of:
- Token → Creator mapping (`get_token_creator()`)
- Creator → Active tokens (`get_creator_tokens()`)
- Creator transaction history (`record_creator_transaction()`)
- Private mempool usage patterns (`is_token_creator_private()`)

**Available Methods**:
```rust
// Check if address is a known creator
is_creator(address: &str) -> bool

// Get all tokens created by an address
get_creator_tokens(creator_address: &str) -> Vec<String>

// Get creator info for a token
get_token_creator(token_address: &str) -> Option<TokenCreatorState>

// Track creator transactions for pattern detection
record_creator_transaction(creator_address: &str, tx_hash: &str, function_name: &str, seen_in_mempool: bool)
```

**Python Integration**:
- Receives creator updates via ZMQ from Python
- Python tracks contract creations and identifies creators
- Updates flow through `TokenTrackingSubscriber`

### 4. Sequential Simulator (BuySellSimulator)

Provides definitive honeypot detection through actual transaction simulation:

#### Key Features

- **Three-Step Simulation**: Original tx → Buy → Sell sequence
- **Honeypot Detection**: Identifies tokens that prevent selling
- **Tax Calculation**: Accurate buy/sell tax from actual transfers
- **State Verification**: Ensures trading is actually enabled

#### Simulation Flow

```rust
1. Simulate original transaction (e.g., enableTrading)
2. Simulate buy transaction:
   - Transfer 0.1 ETH to token
   - Approve max tokens to router
   - Calculate tokens received
3. Simulate sell transaction:
   - Sell all received tokens
   - Verify transaction succeeds
   - Calculate actual tax from ETH received
```

#### Detection Capabilities

- **Honeypot Detection**: Identifies tokens that prevent selling
- **Tax Calculation**: Measures actual buy/sell taxes from transfers
- **Trading Verification**: Ensures trading is actually enabled
- **State Changes**: Detects changes before/after transactions

### 5. Pool Analyzer (`pool_analyzer.rs`)

Analyzes simulation results to detect pool-level events like liquidity drains and scams.

#### Key Features

- **State Change Analysis**: Processes address state changes from simulations
- **Pool Cache Integration**: Uses real-time pool reserves from TokenTrackingCache
- **Drain Detection**: Configurable thresholds (default 60% or < 0.3 ETH)
- **Event Generation**: Creates structured MarketEvent objects

#### Detection Criteria

**Liquidity Drain**:
- ETH reserve drops > 60% in single transaction
- OR final ETH reserve < 0.3 ETH
- Severity based on drain percentage

**Scam Detection**:
- Same criteria as liquidity drain
- Additional context from pool age and volume

### 6. Signal Generator

Combines all inputs to generate actionable signals:
```rust
pub enum Signal {
    NewToken {
        address: Address,
        creator: Address,
        has_trading_enabled: bool,
        initial_tax: TaxInfo,
        is_honeypot: bool,
        risk_score: u8, // 0-100
    },
    TaxManipulation {
        token: Address,
        creator: Address,
        function: String,
        tax_before: TaxInfo,
        tax_after: TaxInfo,
        is_honeypot_after: bool,
    },
    TradingStatusChange {
        token: Address,
        enabled: bool,
        tax_info: TaxInfo,
        liquidity: U256,
    },
    LiquidityChange {
        token: Address,
        pool: Address,
        change_type: LiquidityChangeType,
        amount_eth: U256,
        new_total_eth: U256,
    },
    OwnershipChange {
        token: Address,
        old_owner: Address,
        new_owner: Address,
        is_renounced: bool,
    },
}
```

## Simulation Priority System

### Highest Priority (Always Simulate)
1. **Known Creator Transactions**
   - Any transaction from an address in `creator_storage`
   - Especially those with multiple tokens
   - Double priority if private mempool user

2. **High-Risk Functions from Creators**
   - `setTaxes()`, `updateFees()`
   - `setSwapEnabled()`, `disableTrading()`
   - `transferOwnership()`
   - `setMaxWallet()`, `setMaxTx()`

### Medium Priority (Selective Simulation)
3. **Contract Creation with Trading**
   - If `enableTrading()` in same transaction
   - If liquidity added in same block

4. **Unknown Address + Risk Function**
   - Tax/fee modifications
   - Trading status changes

### Low Priority (Function Detection Only)
5. **Regular Swaps**
   - Unless from creator address
   
6. **Transfers**
   - Unless large amount from creator

## Signal Priority

1. **CRITICAL** (Immediate Action):
   - Honeypot detected (can't sell)
   - Tax > 50% after change
   - Liquidity removed > 90%

2. **HIGH** (Rapid Response):
   - Tax increased > 20%
   - Trading disabled
   - Ownership transferred

3. **MEDIUM** (Monitor):
   - New token with high tax (>10%)
   - Max wallet < 2%
   - Creator with history of scams

4. **LOW** (Information):
   - Trading enabled
   - Tax decreased
   - Renounced ownership

## Data Flow

1. **Transaction Reception**: 
   - NonBlockingIpcClient receives transactions from Reth IPC
   - Sub-millisecond detection latency

2. **Function Detection**:
   - FunctionDetector analyzes 4-byte selectors
   - Categorizes into liquidity, trading, swap, or other
   - Logs to dedicated files

3. **Enrichment**:
   - TokenTrackingCache provides pool state and creator info
   - Transactions matched against known creators

4. **Simulation**:
   - SimulatorProcessor batches transactions
   - Uses reth_tx_simulator for state changes
   - Handles nonce adaptation automatically

5. **Analysis**:
   - CreatorAnalyzer flags creator actions
   - PoolAnalyzer detects drains and scams
   - SignalDetector aggregates all signals

6. **Publishing**:
   - Alerts published via ZMQ
   - Separate channels for different alert types
   - JSON formatted for easy consumption

## Performance Characteristics

- **IPC Detection**: < 1ms average latency
- **Function Detection**: ~0.005ms per transaction
- **Cache Lookups**: < 1μs
- **Full Simulation**: < 50ms for buy/sell sequence
- **Batch Processing**: 50 transactions per batch
- **Memory Usage**: Minimal with lazy static initialization
- **Throughput Target**: 1000+ transactions/second

## Configuration

Key configuration parameters:

```rust
// SimulatorProcessorConfig
batch_size: 50
batch_timeout: 100ms
max_queue_size: 1000

// PoolAnalysisConfig  
liquidity_drain_threshold: 0.6 (60%)
min_eth_threshold: 0.3 ETH
enable_logging: true
```

## State Management

```rust
pub struct TokenState {
    // Basic info
    address: Address,
    creator: Address,
    creation_block: u64,
    
    // Trading state
    trading_enabled: bool,
    current_tax: TaxInfo,
    is_honeypot: bool,
    
    // Liquidity info
    main_pool: Address,
    eth_liquidity: U256,
    
    // Risk metrics
    creator_risk_score: u8,
    tax_change_count: u32,
    last_simulation: Instant,
}
```

## Integration Points

### Input Sources
- Reth IPC socket for mempool transactions
- TokenTrackingCache for pool state (ZMQ port 5558)
- Python token cache updates for creator info

### Output Channels
- Log files in `logs/` directory
- ZMQ publishers:
  - 5556: Function alerts
  - 5557: Pool analysis (scams, drains)
  - 5559: Creator alerts

## Signal Examples

### Example 1: Safe Token Launch
```json
{
  "type": "NEW_TOKEN",
  "timestamp": "2024-01-27T10:30:00Z",
  "data": {
    "address": "0x...",
    "creator": "0x...",
    "has_trading_enabled": true,
    "initial_tax": {
      "buy": 5,
      "sell": 5
    },
    "is_honeypot": false,
    "risk_score": 15,
    "details": "Standard token launch with reasonable tax"
  }
}
```

### Example 2: Honeypot Detection
```json
{
  "type": "TAX_MANIPULATION",
  "severity": "CRITICAL",
  "timestamp": "2024-01-27T10:35:00Z",
  "data": {
    "token": "0x...",
    "creator": "0x...",
    "function": "setSwapEnabled",
    "tax_before": {
      "buy": 5,
      "sell": 5
    },
    "tax_after": {
      "buy": 5,
      "sell": -1
    },
    "is_honeypot_after": true,
    "details": "Token now prevents all sells - HONEYPOT"
  }
}
```

## Statistics and Monitoring

The system tracks comprehensive statistics:

- Total transactions processed
- Function detection counts by type
- Creator alerts by severity
- Pool drains and scams detected
- Performance metrics (latency, throughput)
- Simulation success rates

Statistics are logged every 60 seconds and available via component-specific methods.

## Future Improvements

1. **Enhanced Creator Detection**: ML-based pattern recognition
2. **Cross-pool Analysis**: Detect coordinated attacks
3. **MEV Integration**: Identify sandwich attacks
4. **Parameter Decoding**: Full function parameter analysis
5. **WebSocket Support**: Alternative to ZMQ for alerts
6. **Persistent Storage**: Database for historical analysis
7. **Multi-block Simulation**: Predict future manipulation
8. **Gas Optimization Detection**: Identify efficient routing
9. **Slippage Analysis**: Detect abnormal price impacts