# Signal Engine

The Signal Engine is a high-performance transaction analysis system that detects specific function calls, market events, and suspicious activity in Ethereum mempool transactions with sub-millisecond latency. It combines multiple detection methods to identify potential scams, trading opportunities, and market manipulation in real-time.

## Overview

The signal detection system monitors mempool transactions to provide comprehensive analysis through:

1. **Function Detection**: Identifies specific function calls in transactions (liquidity removals, trading enables, swaps)
2. **Creator Analysis**: Tracks known token creators/owners for suspicious pre-market activity
3. **Sequential Simulation**: Simulates transactions and their effects on trading (buy/sell tests)
4. **Pool Analysis**: Analyzes state changes to detect liquidity drains and scams
5. **Alert Publishing**: Publishes alerts via ZMQ for real-time consumption

## Real-World Scenarios

### 1. New Token Launch
```
Mempool → Contract Creation → Extract Token Info → Monitor Creator
         ↓
         Trading Enabled? → Simulate Buy/Sell → Detect Initial Tax
         ↓
         Signal: NEW_TOKEN_LAUNCH {safe: true/false, tax: X%}
```

### 2. Creator Manipulation
```
Mempool → Transaction from Known Creator → Identify Function
         ↓
         Simulate Transaction → Simulate Buy/Sell After
         ↓
         Compare Before/After → Detect Changes
         ↓
         Signal: TAX_CHANGE {before: X%, after: Y%, honeypot: bool}
```

### 3. Trading Status Change
```
Mempool → enableTrading() or disableTrading() → Simulate
         ↓
         Verify Trading Works → Check Tax Changes
         ↓
         Signal: TRADING_ENABLED {token: address, safe: bool, tax: X%}
```

## Architecture

```
mempool_signal_detector.rs (main binary)
    ↓
NonBlockingIpcClient → FunctionDetector → SimulatorProcessor → SignalDetector
    ↓                                           ↓                    ↓
TokenTrackingSubscriber                   CreatorAnalyzer      PoolAnalyzer
    ↓                                           ↓                    ↓
TokenTrackingCache                         ZMQ :5559           ZMQ :5557
```

## Detection Flow

```
                    Mempool Transaction
                           |
                    ┌──────┴──────┐
                    │ Classifier  │
                    └──────┬──────┘
                           |
        ┌─────────────────┼─────────────────┐
        |                 |                 |
   Contract          Creator Tx        DEX/Other
   Creation              |                 |
        |                |                 |
   Extract          Function           (monitor)
   Token Info       Detector
        |                |
        └────────┬───────┘
                 |
          ┌──────┴──────┐
          │  Simulator  │
          └──────┬──────┘
                 |
          - Simulate Tx
          - Buy Simulation
          - Sell Simulation
                 |
          ┌──────┴──────┐
          │   Signal    │
          │ Generator   │
          └──────┬──────┘
                 |
            Risk Score
            + Signal
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