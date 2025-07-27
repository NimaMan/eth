# Mempool Signal Detection Architecture

This document explains the complete architecture of the mempool signal detection system, from transaction ingestion through signal generation and publishing, including the data transformation at each stage.

## High-Level Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Reth Node     │────▶│  IPC Client      │────▶│ Function        │
│   (Mempool)     │     │  (NonBlocking)   │     │ Detector        │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                           │
                                                           ▼
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ Signal Engine   │────▶│   Publishers     │     │   Simulator     │
│ (Orchestrator)  │     │  (ZMQ/Logs)      │     │   Processor     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Data Flow and Transformation

### 1. IPC Transaction Data Format

The IPC client (`NonBlockingIpcClient`) receives full transaction data as JSON from Reth:

```json
{
  "hash": "0x123...",
  "from": "0xabc...",
  "to": "0xdef...",
  "value": "0x1234",
  "gas": "0x5208",
  "gasPrice": "0x3b9aca00",
  "nonce": "0x0",
  "input": "0xa9059cbb000000000000000000000000...", // Function call data
  "type": "0x2",
  "maxFeePerGas": "0x...",
  "maxPriorityFeePerGas": "0x...",
  "v": "0x1",
  "r": "0x...",
  "s": "0x..."
}
```

This is wrapped in a `MempoolTransaction` struct:
```rust
pub struct MempoolTransaction {
    pub hash: String,
    pub data: serde_json::Value,  // The JSON above
    pub detection_ns: u64,         // Detection latency in nanoseconds
    // Pre-parsed fields for fast access
    pub from: Vec<u8>,
    pub to: Option<Vec<u8>>,
    pub input: Vec<u8>,
    pub value: U256,
    pub gas_price: Option<U256>,
    pub functions: Vec<String>,    // Detected function names (populated by FunctionDetector)
}
```

**Transaction Types** (based on `type` field):
- `0x0` or missing: Legacy transaction
- `0x1`: EIP-2930 (access list)
- `0x2`: EIP-1559 (dynamic fees with maxFeePerGas)

**Performance**: 2-7μs detection latency from mempool to struct

### 2. Function Detection

The `FunctionDetector` analyzes the `input` field to identify function calls:

```rust
// Input: Vec<MempoolTransaction> with calldata
let mut transactions = vec![tx1, tx2, tx3];

// Process batch - modifies functions field in-place
let transactions_with_functions = function_detector.detect_batch(transactions);

// Each transaction now has populated functions field
for tx in transactions_with_functions {
    println!("TX {} has functions: {:?}", tx.hash, tx.functions);
    // tx.functions = ["transfer", "liquidity_removal", etc.]
}
```

**Detected Functions**:
- **Liquidity**: `removeLiquidityETH` (0x02751cec), `removeLiquidity` (0xbaa2abde)
- **Trading**: `enableTrading` (0x8ee88c53), `openTrading` (0xc9567bf9)
- **Tax**: `setTaxes` (0x032dc6a2), `setBuyTax` (0x2f2ff15d)
- **Swaps**: Various swap functions from DEX routers

### 3. Transaction Classification & Routing

The classifier combines function detection with additional context:

```rust
// Input: MempoolTransaction with populated functions field + TokenCache data
let tx = MempoolTransaction { 
    functions: vec!["enableTrading"],
    from: creator_address_bytes,
    ... 
};

// Classification process
let is_creator = token_cache.is_creator(&tx.from);
let is_contract_creation = tx.to.is_none();
let has_risky_functions = tx.functions.iter().any(|f| 
    f.contains("Trading") || f.contains("Tax") || f.contains("Liquidity")
);

// Output: ClassificationResult
pub struct ClassificationResult {
    pub category: TransactionCategory,
    pub priority: SimulationPriority,
    pub requires_simulation: bool,
    pub requires_buy_sell_test: bool,
}
```

**Categories**:
- `ContractCreation`: New deployments (check for ERC20 bytecode patterns)
- `CreatorTransaction`: From known token creators (regardless of function)
- `DexInteraction`: To Uniswap/Sushiswap routers
- `Regular`: Everything else

**Priority Assignment**:
- `Critical`: Creator + tax/trading functions, liquidity removals
- `High`: Trading enables, new tokens with liquidity
- `Normal`: Regular swaps
- `Low`: Simple transfers

### 4. Simulation Processing

The `SimulatorProcessor` executes transactions against current blockchain state:

```rust
// Input: Classified transaction batch
let simulation_request = SimulationRequest {
    tx: MempoolTransaction { ... },
    category: TransactionCategory::CreatorTransaction { ... },
    priority: SimulationPriority::Critical,
    simulation_type: SimulationType::TransactionWithBuySell,
};

// Simulation process (using Reth DB directly)
// 1. Load current state from DB
// 2. Execute transaction in EVM
// 3. Track state changes

// Output: SimulationResult
pub struct SimulationResult {
    pub request: SimulationRequest,
    pub tx_simulation: Option<TxSimulationResult>,
    pub buy_sell_result: Option<BuySellResult>,
    pub simulation_time_ms: f64,
}

// State changes tracked
pub struct StateChange {
    pub address: String,
    pub eth_change: f64,        // -10.5 ETH
    pub token_changes: HashMap<String, f64>, // {"0xtoken": 1000000.0}
}
```

**Simulation Modes**:
- `TransactionOnly`: Just the mempool tx
- `TransactionWithBuySell`: Tx + buy 0.1 ETH + sell tokens
- `BuySellOnly`: For existing token checks

**Performance**: 5-10ms average, 50ms max

### 5. Signal Detection

Specialized detectors analyze simulation results to identify patterns:

```rust
// Input: SimulationResult with state changes
let sim_result = SimulationResult {
    buy_sell_result: Some(BuySellResult {
        can_buy: true,
        can_sell: false,  // Red flag!
        buy_tax: Some(5.0),
        sell_tax: Some(99.0),
        tokens_received: Some(1000000.0),
        eth_received_on_sell: None,
    }),
    ...
};

// Each detector runs independently
let honeypot_signal = honeypot_detector.detect(&sim_result);
let liquidity_signal = liquidity_detector.detect(&sim_result);
let tax_signal = tax_change_detector.detect(&sim_result);

// Output: Specific signal types
pub struct HoneypotSignal {
    pub token_address: String,
    pub detection_method: HoneypotDetectionMethod,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub confidence: f64,
}
```

**Detector Types**:
1. **Honeypot**: Sell fails, high tax >50%, zero return
2. **Liquidity**: Drain detection (>90% removed, <0.3 ETH left)
3. **Tax Change**: Before/after comparison, honeypot conversion
4. **Trading Status**: Enable/disable with verification

### 6. Signal Generation & Risk Scoring

The signal generator combines detector outputs with risk scoring:

```rust
// Input: Multiple detector signals
let honeypot_signal = HoneypotSignal { ... };
let category = TransactionCategory::CreatorTransaction { ... };

// Risk scoring considers multiple factors
let risk_factors = RiskFactors {
    creator_history_score: 80,  // Known serial creator
    token_age_score: 90,        // Brand new token
    liquidity_score: 70,        // Low liquidity
    tax_score: 95,              // 99% sell tax
    function_score: 80,         // High-risk function
    private_mempool_score: 20,  // Public tx
    pattern_score: 90,          // Clear honeypot pattern
};

// Weighted scoring algorithm
let risk_score = calculate_weighted_score(&risk_factors); // = 85

// Output: Unified signal
let unified_signal = UnifiedSignal {
    signal_id: "0x123-honeypot-1234567890",
    signal_type: SignalType::Honeypot,
    severity: Severity::Critical,
    confidence: 0.95,
    risk_score: 85,
    tx_hash: H256::from("0x123..."),
    timestamp: 1234567890,
    data: SignalData::Honeypot(honeypot_data),
    metadata: SignalMetadata { ... },
};
```

**Risk Calculation**: Weighted average of factors with pattern score having highest weight

### 7. Signal Publishing

The final unified signals are published to multiple outputs:

```rust
// Input: UnifiedSignal ready for publishing
let signal = UnifiedSignal {
    signal_type: SignalType::Honeypot,
    severity: Severity::Critical,
    risk_score: 85,
    ...
};

// ZMQ multipart message format
let topic = signal.topic(); // "honeypot.critical"
let json_data = serde_json::to_string(&signal)?;

// Publish to ZMQ
socket.send_multipart(&[topic.as_bytes(), json_data.as_bytes()], 0)?;

// Also log to files
writeln!(honeypot_log, "[{}] {}", timestamp, json_data)?;
```

**Publishing Channels**:

#### 7.1 ZMQ Publishers
- **Port 5556**: Function detection alerts (raw detections)
- **Port 5557**: Pool analysis (scams, drains) 
- **Port 5559**: Creator action alerts
- **Port 5560**: Unified signals (new, all signal types)

**Message Format**: `[topic, json_payload]` where topic is `{signal_type}.{severity}`

#### 7.2 Log Files
- `liquidity_removals.log`: Liquidity events with amounts
- `trading_enabled.log`: Trading status changes
- `creator_actions.log`: All creator transactions
- `signal_detector.log`: Main unified signal log
- `honeypots.log`: Confirmed honeypot detections

#### 7.3 Database (Optional)
- `signals` table: All unified signals with risk scores
- `detections` table: Raw detector outputs
- `simulations` table: Simulation results for analysis

## Performance Characteristics

### Detection Latency
| Component | Average | Maximum |
|-----------|---------|---------|
| IPC Reception | 5μs | 40μs |
| Function Detection | 5μs | 256μs |
| Classification | <1ms | 2ms |
| Simulation | 5-10ms | 50ms |
| Signal Generation | <1ms | 5ms |
| **Total Pipeline** | **6-12ms** | **60ms** |

### Throughput
- **IPC**: 700+ tx/sec sustained
- **Function Detection**: 200,000+ tx/sec
- **Simulation**: 100-200 tx/sec (bottleneck)
- **Publishing**: 10,000+ signals/sec

## Configuration

### Key Environment Variables
```bash
# Required
IPC_PATH=/tmp/reth.ipc          # Reth IPC socket
RETH_DATADIR=~/.local/share/reth/mainnet

# Optional
LOG_LEVEL=info                  # Logging verbosity
ZMQ_ENDPOINT=tcp://127.0.0.1:5556
```

### Tuning Parameters
```rust
// Batch sizes
const BATCH_SIZE: usize = 100;  // Transactions per batch

// Thresholds
const LIQUIDITY_DRAIN_THRESHOLD: f64 = 0.6;  // 60%
const MIN_ETH_THRESHOLD: f64 = 0.3;           // 0.3 ETH
const HIGH_TAX_THRESHOLD: f64 = 50.0;         // 50%

// Priorities
const CREATOR_TX_PRIORITY: Priority = Critical;
const LIQUIDITY_REMOVAL_PRIORITY: Priority = High;
```

## High-Level Signal Types

The system generates various signal types based on detected patterns:

1. **Honeypot Signals**
   - Token prevents selling through various mechanisms
   - High confidence when sell simulation fails
   - Critical severity for confirmed honeypots

2. **Liquidity Signals**
   - Major liquidity removals that could crash price
   - Complete drains indicate rug pull
   - Tracks remaining ETH in pools

3. **Tax Manipulation Signals**
   - Creator changes buy/sell taxes
   - Detects conversion to honeypot
   - Risk based on tax levels and changes

4. **Trading Control Signals**
   - Trading enabled/disabled/paused
   - Verifies if trading actually works
   - Detects fake enables (honeypot risk)

5. **Creator Action Signals**
   - Any suspicious action by token creators
   - Higher risk for serial creators
   - Tracks pattern of behavior

6. **Scam/Rug Pull Signals**
   - Combination of multiple risk factors
   - Highest severity for immediate action
   - Based on liquidity drain + creator history

## Integration Points

### Input Sources
1. **Reth IPC**: Raw mempool transactions
2. **Token Cache**: Pool states and creator info (via ZMQ from Python)
3. **Database**: Historical data for analysis

### Output Consumers
1. **Trading Bots**: Subscribe to ZMQ for real-time signals
2. **Analytics Systems**: Process log files for patterns
3. **Alert Systems**: Monitor critical severity signals
4. **Research Tools**: Query historical database

## Monitoring & Operations

### Health Checks
- IPC connection status
- Queue depths (should be <1000)
- Simulation success rate (>80%)
- Publishing latency (<10ms)

### Key Metrics
```
# Prometheus-style metrics
mempool_transactions_total{status="processed"}
mempool_detection_latency_ms{percentile="p99"}
signal_detection_total{type="honeypot",severity="critical"}
simulation_success_rate
publisher_queue_depth{endpoint="tcp://127.0.0.1:5556"}
```

### Troubleshooting

#### High Latency
1. Check IPC connection quality
2. Monitor simulation queue depth
3. Verify database performance
4. Check network congestion for ZMQ

#### Missing Signals
1. Verify function detector patterns
2. Check creator cache updates
3. Confirm simulation success
4. Monitor publisher errors

## Future Enhancements

1. **Machine Learning**: Pattern recognition for new scam types
2. **Cross-Chain**: Monitor multiple chains simultaneously
3. **MEV Integration**: Detect sandwich attacks
4. **WebSocket API**: Alternative to ZMQ for web clients
5. **Distributed Processing**: Horizontal scaling for simulation