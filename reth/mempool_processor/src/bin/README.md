# Mempool Signal Detector Service

## Overview

The **mempool_signal_detector** is a production-grade service that implements a complete signal detection pipeline for Ethereum mempool transactions. It receives transactions from Reth IPC, detects function signatures, routes transactions by category, simulates relevant transactions, detects signals (trading enabled, liquidity removal, honeypots), and publishes signals via ZMQ and logs.

This service operates with strict performance targets:
- Function detection: <10μs per transaction
- TX routing: <5μs per transaction  
- Simulation: <50ms per transaction (Reth bottleneck)
- Signal detection: <1ms per result
- End-to-end: <100ms for critical signals

## High-Level Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Reth Node     │────▶│  IPC Client      │────▶│ Function        │
│   (Mempool)     │     │  (IPC Client)    │     │ Detector        │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                           │
                                                           ▼
┌─────────────────┐     ┌──────────────────┐     
│  TX Router      │────▶│ SimulationManager│     
│ (Classification)│     │ (Integrated)     │     
└─────────────────┘     └──────────────────┘     
           │                       │               
           ▼                       ▼               
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Token Cache    │────▶│ Signal Manager   │────▶│   Publishers    │
│ (Context)       │     │ (Auto-Detection) │     │  (ZMQ/Logs)     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
```

## Data Flow and Transformation

### Complete Processing Pipeline (New Integrated Architecture)

```
1. IPC Client receives transaction
   ↓
2. FunctionDetector identifies function signatures  
   ↓
3. TransactionRouter categorizes and assigns priority
   ↓
4. SimulationManager.submit() - All-in-one processing:
   - Executes transaction simulation
   - Runs buy/sell tests if needed
   - Gets token context from cache
   - Detects signals automatically
   - Publishes signals immediately
   ↓
5. Main loop continues (no result handling needed)
```

**Key Change**: Signal detection is now integrated into SimulationManager, 
eliminating the need for separate signal processing steps.

### 1. IPC Transaction Data Format

The IPC client (`MempoolFetcherIPCClient`) receives full transaction data as JSON from Reth:

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
    pub detection_ns: u64,        // Detection latency in nanoseconds
    pub detection_time: Instant,  // When it was detected
    pub latency_ns: u64,          // Alias for detection_ns
    // Pre-parsed fields for fast access
    pub from: Vec<u8>,
    pub to: Option<Vec<u8>>,
    pub input: Vec<u8>,
    pub value: U256,
    pub gas_price: Option<U256>,
    pub functions: Vec<String>,
    pub function_category: Option<CreatorFunctionType>,
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

### 3. Transaction Routing

The `TransactionRouter` categorizes transactions and assigns simulation priorities:

```rust
// Input: MempoolTransaction with populated functions field
let tx = MempoolTransaction { 
    functions: vec!["enableTrading"],
    from: creator_address_bytes,
    to: Some(token_address_bytes),
    ... 
};

// Routing process
let routing_result = tx_router.classify(&tx).await;

// Output: ClassificationResult
pub struct ClassificationResult {
    pub category: TransactionCategory,
    pub priority: SimulationPriority,
    pub requires_simulation: bool,
    pub requires_buy_sell_test: bool,
}
```

**Categories**:
- `ContractCreation`: New deployments with deployer and contract address
- `CreatorTransaction`: From known token creators with target analysis  
- `Regular`: Standard transfers and approvals

**Priority Assignment**:
- `Critical`: Creator transactions with critical functions
- `High`: Trading enabled, liquidity operations
- `Normal`: DEX interactions, regular contract calls
- `Low`: Simple transfers and standard operations

### 4. Main Loop Architecture

The main processing loop implements an asynchronous queue-based design that separates fast transaction reception from slower simulation processing. This architecture ensures the IPC client maintains sub-millisecond latency while critical transactions receive immediate simulation.

**Queue-Based Processing Flow**:
- Main loop receives transactions from IPC and routes them
- Contract Creation and Creator Actions are queued for simulation
- Separate async task processes the simulation queue by priority
- Main loop continues without blocking on simulation
- Signals are detected and published after simulation completes

**Priority System**:
- Critical priority transactions (tax changes, trading controls) are simulated within milliseconds
- High priority transactions (liquidity changes, token deployments) are processed quickly
- Normal priority transactions are batched for efficiency
- Low priority transactions are dropped when queue is full

**Focused Scope**: 
The system currently processes only Contract Creation and Creator Action transactions. DEX interactions and regular transfers are filtered out early to focus computational resources on high-value signals. This design decision simplifies the pipeline while capturing the most important events for scam detection.

### 5. Signal Processing Coordination

The `SimulationManager` now handles the entire flow internally:

```rust
impl SimulationManager {
    pub async fn process_transaction(&self, tx: MempoolTransaction) -> Vec<Signal> {
        // 1. Route the transaction
        let routing = self.tx_router.classify(&tx).await;
        
        // 2. Get trading status from token cache
        let trading_status = if let Some(token_addr) = self.extract_token_address(&tx) {
            self.token_cache.is_trading_enabled(&token_addr).await
        } else {
            false // No token = no trading status
        };
        
        // 3. Run simulation if transaction requires it
        let simulation_result = if routing.requires_simulation || routing.requires_buy_sell_test {
            Some(self.buy_sell_simulator.simulate(&tx).await?)
        } else {
            None
        };
        
        // 4. Check all signal detectors with context
        let mut signals = Vec::new();
        
        if let Some(sim_result) = &simulation_result {
            // Trading Enabled Signal
            if let Some(signal) = self.trading_enabled_detector
                .check_transaction(&tx, sim_result, trading_status) {
                signals.push(Signal::TradingEnabled(signal));
            }
            
            // High Tax Warning Signal  
            if let Some(signal) = self.high_tax_detector
                .check_transaction(&tx, sim_result, trading_status) {
                signals.push(Signal::HighTaxWarning(signal));
            }
        }
        
        // Liquidity Removal Signal (no simulation needed)
        if let Some(signal) = self.liquidity_removal_detector
            .check_transaction(&tx) {
            signals.push(Signal::LiquidityRemoval(signal));
        }
        
        // 5. Send signals to generator for formatting and publishing
        for signal in &signals {
            self.signal_generator.process_and_publish(signal).await;
        }
        
        signals
    }
}
```

**Key Coordination Functions**:
- **Token Address Extraction**: From transaction data or routing results
- **Trading Status Query**: Via TokenCache (ZMQ communication with Python)
- **Simulation Orchestration**: Only when needed based on routing
- **Context Passing**: Provides detectors with trading status
- **Signal Collection**: Aggregates all binary signals
- **Publishing**: Coordinates output to all channels

### 5. Simulation Processing

The simulation system uses a priority queue and batch processing to efficiently handle high-value transactions. The `SimulationManager` coordinates transaction execution and signal detection in a single integrated flow.

**Architecture Overview**:
```rust
pub struct SimulationManager {
    // Simulation components
    tx_simulator: Arc<TxSimulator>,
    buy_sell_simulator: Arc<SequentialBuySellSimulator>,
    queue: Arc<Mutex<SimulationQueue>>,
    
    // Signal detection (NEW)
    signal_manager: SignalManager,
    token_cache: Arc<AddressTrackingCache>,
    
    // Configuration
    max_concurrent_simulations: usize,
}
```

**Integrated Flow**:
1. Transaction submitted to SimulationManager
2. Simulation executed (transaction + buy/sell tests)
3. Signal detection runs immediately using token cache context
4. Signals published automatically
5. Main loop doesn't need to handle results

**Queue Management**:
- Maximum queue size of 10,000 transactions prevents memory issues
- Critical priority transactions bypass normal batching for immediate processing
- Transactions are grouped into batches of up to 50 for efficiency
- Queue automatically drops low priority transactions when full

**Simulation Types**:
- Contract Creation simulations extract token parameters and check initial state
- Creator Action simulations test the effects of function calls (tax changes, trading controls)
- Buy/Sell testing validates that tokens remain tradeable after changes

The `BuySellSimulator` executes transactions against current blockchain state:

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

### 5. Signal Detection (Integrated with Simulation)

Signal detection is now integrated directly within the SimulationManager, using token cache for context-aware decisions:

```rust
// Inside SimulationManager::simulate_request()
async fn simulate_request(&self, request: SimulationRequest) -> SimulationResult {
    // 1. Run simulation
    let result = self.execute_simulation(request).await;
    
    // 2. Get token context from cache
    let token_info = match &request.category {
        TransactionCategory::ContractCreation { contract_address, .. } => {
            self.token_cache.get_token_info(contract_address).await
        }
        TransactionCategory::CreatorTransaction { token_address, .. } => {
            self.token_cache.get_token_info(token_address).await
        }
        _ => None
    };
    
    // 3. Detect signals with context
    let signals = self.signal_manager.process_simulation_result(
        &result,
        token_info.as_ref(),
    ).await;
    
    // 4. Publish signals immediately
    for signal in signals {
        self.publish_signal(signal).await;
    }
    
    result
}
```

**Context-Aware Detection Examples**:

```rust
// Contract Creation: Check if trading is enabled
if let Some(buy_sell) = &result.buy_sell_result {
    if buy_sell.can_buy && buy_sell.can_sell {
        // New token with trading enabled!
        emit_signal(TradingEnabledSignal { ... });
    }
}

// Creator Transaction: Compare with cached state
if let Some(token_info) = token_info {
    if !token_info.trading_enabled && buy_sell.can_buy && buy_sell.can_sell {
        // Trading just got enabled!
        emit_signal(TradingEnabledSignal { ... });
    }
    
    if token_info.trading_enabled && !buy_sell.can_sell {
        // Honeypot - was tradeable, now can't sell!
        emit_signal(HoneypotSignal { ... });
    }
}
```

**Simple Signal Types**:
1. **Trading Enabled**: Token tradeable with reasonable taxes (≤25%)
2. **High Tax Warning**: Taxes exceed thresholds (>25% or >50% for honeypot)
3. **Liquidity Removal**: LP removal from pools with minimum ETH value

**Configuration** (in `/src/config.rs`):
- `max_acceptable_buy_tax: 25%`
- `max_acceptable_sell_tax: 25%`
- `honeypot_sell_threshold: 50%`
- `min_pool_eth: 0.05 ETH`

### 6. Signal Publishing (Simplified Output)

Binary signals are published immediately when detected:

```rust
// Input: Simple binary signal detection
if conditions_met {
    let signal = TradingEnabledSignal {
        tx_hash: "0x123...".to_string(),
        token_address: "0xabc...".to_string(),
        creator_address: "0xdef...".to_string(),
        buy_tax: 5,
        sell_tax: 10,
        timestamp: chrono::Utc::now().timestamp() as u64,
        block_number: latest_block,
    };
    
    // Immediate publishing - no risk scoring
    publish_to_zmq("trading_enabled", &signal);
    log_to_file("trading_enabled.log", &signal);
}
```

**No Risk Scoring**: Direct binary output - signal detected or not detected

### Main Loop Simplification

With the integrated architecture, the main processing loop is greatly simplified:

```rust
// OLD: Complex result handling
let sim_results = simulation_manager.process_queue().await;
for result in sim_results {
    let signals = signal_manager.process_result(result);
    // Handle signals...
}

// NEW: Fire and forget
simulation_manager.submit(sim_request).await;
// That's it! Signals are detected and published internally
```

The SimulationManager handles the complete flow internally:
- Executes simulation
- Checks token cache for context
- Detects signals based on results + context
- Publishes signals to ZMQ/logs
- Updates internal metrics

This encapsulation makes the system more maintainable and ensures signal detection always happens with proper context.

### 7. Output Channels

Simple binary signals are published to multiple outputs:

```rust
// Input: Simple binary signal ready for publishing
let signal = TradingEnabledSignal {
    token_address: "0xabc...".to_string(),
    buy_tax: 5,
    sell_tax: 10,
    ...
};

// ZMQ publishing with topic
let topic = "trading_enabled";
let json_data = serde_json::to_string(&signal)?;
socket.send_multipart(&[topic.as_bytes(), json_data.as_bytes()], 0)?;

// File logging with clear format
writeln!(log_file, "[{}] TRADING_ENABLED | Token: {} | BuyTax: {}% | SellTax: {}% | TxHash: {}", 
    timestamp, signal.token_address, signal.buy_tax, signal.sell_tax, signal.tx_hash)?;
```

**Publishing Channels**:

#### 7.1 ZMQ Publishers  
- **Port 5556**: All binary signals (topics: `trading_enabled`, `tax_signal`, `liquidity_removal`, `scam_detection`, `lp_approval`)

**Message Format**: `[topic, json_payload]` where topic is signal type

#### 7.2 Log Files (under the run directory's `signals/` folder)
- `trading_enabled.log`: Token becomes tradeable with reasonable taxes
- `tax_signals.log`: High tax, honeypot, or suspicious tax patterns (consolidated)
- `liquidity_removals.log`: LP removal operations (also includes ScamDetection entries)
- (scam detections merged into `liquidity_removals.log`)
- `lp_approval_signals.log`: Creator approving router to spend LP tokens
- `signal_manager.log`: Per‑TX activity summary from detectors

#### 7.3 Database (Optional)
- Simple schema for binary signals only
- No complex risk scores or confidence calculations

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
ARRIVAL_INDEX_DIR=/path/to/arrival_index   # Enable recording of arrival times
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

## Binary Signal Types

The system generates three simple binary signals:

1. **Trading Enabled Signal**
   - Token becomes tradeable with reasonable taxes (≤25%)
   - Both buy and sell transactions succeed
   - Clear indication of legitimate token launch

2. **High Tax Warning Signal** 
   - Buy tax > 25% OR sell tax > 25%
   - Sell tax > 50% indicates potential honeypot
   - Simple threshold-based detection

3. **Liquidity Removal Signal**
   - LP removal functions detected (removeLiquidity*, decreaseLiquidity)
   - Pool has minimum ETH value (≥0.05 ETH)
   - Immediate alert for potential rug pulls

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

## Command Line Arguments

The service accepts the following command-line arguments:

```bash
mempool_signal_detector [OPTIONS]

OPTIONS:
    --ipc-path <PATH>           IPC socket path [env: IPC_PATH] [default: /tmp/reth.ipc]
    --reth-db-path <PATH>       Reth database path for simulations [env: RETH_DB_PATH] 
                                [default: /home/nima/.local/share/reth/mainnet]
    --log-dir <PATH>            Log directory base path [default: /home/nima/code/crypto/logs/mempool]
    --batch-size <SIZE>         Batch size for transaction processing [default: 100]
    --sim-workers <COUNT>       Simulation worker threads [default: 10]
    -v, --verbose               Enable verbose logging
    --report-interval <SECS>    Performance report interval in seconds [default: 60]
    --database-url <URL>        Database URL for mempool timestamp tracking [env: DATABASE_URL]
```

## Service Components

### 1. Token Tracking Subscriber
- Starts immediately on service launch
- Maintains real-time cache of token pools and creators
- 0.1 ETH threshold for tracking
- Provides context for signal detection decisions

### 2. IPC Client (NonBlockingIpcClient)
- Connects to Reth node via Unix socket
- Zero-copy transaction parsing
- Auto-reconnection on failure
- Maintains sub-millisecond latency

### 3. Mempool Arrival Index (Optional)
- Records first-seen mempool timestamps and persists only when the tx is mined
- Minimal mapping: `TxNumber → first_seen_ns`
- Enable with `--arrival-index-dir <DIR>` or `ARRIVAL_INDEX_DIR` env var
- Default backend: file-backed CSV (`tx_arrivals.csv`); MDBX backend available behind feature flag `arrival_mdbx`

### 4. Function Detector
- Identifies function signatures in transaction calldata
- Maintains cache of known signatures
- Custom log directory for function detection logs

### 5. Transaction Router
- Classifies transactions into categories
- Assigns simulation priorities
- Determines which transactions require simulation

### 6. Unified Simulator
- Single database connection for all simulations
- Configurable buy/sell testing parameters
- Direct Reth database access for performance

### 7. Signal Publisher
- Publishes signals to ZMQ endpoints
- Writes structured logs to signal directory
- Integrated with simulation manager

### 8. Simulation Manager
- Manages concurrent simulation workers
- Integrates signal detection and publishing
- Priority-based queue processing
- Automatic signal detection on simulation completion

## Performance Metrics

The service tracks detailed performance metrics:

### Transaction Metrics
- Total processed transactions
- Contract creations count
- Creator actions count
- DEX interactions count
- Regular transactions count

### Simulation Metrics
- Simulations submitted
- Simulations completed
- Simulation errors
- Simulation timing (avg/max/p99)

### Signal Counts
- Trading enabled signals
- Liquidity removal signals
- Honeypot signals
- Tax change signals

### Latency Tracking
- Detection latencies (maintains last 5000 samples)
- Routing latencies (maintains last 5000 samples)
- Simulation times (maintains last 500 samples)

## Log Output Structure

The service creates a timestamped run directory with the following structure:

```
/home/nima/code/crypto/logs/mempool/signal_detector_YYYY-MM-DD_HH-MM-SS/
├── signal_detector.log          # Main service + lifecycle logs
├── simulation_results.log       # One line per simulation outcome (success/error)
├── function_detector/           # Function detector diagnostics
│   └── liquidity_removals.log   # Fast path for removal function matches
└── signals/                     # Per-signal outputs (one file per signal type)
    ├── trading_enabled.log      # TradingEnabled signals
    ├── tax_signals.log          # High tax / honeypot signals
    ├── liquidity_removals.log   # LiquidityRemoval + ScamDetection signals
    ├── lp_approval_signals.log  # LP approval (rug setup) signals
    └── signal_manager.log       # Summary + publication diagnostics
```

## Monitoring & Operations

### Health Checks
- IPC connection status
- Queue depths (should be <1000)
- Simulation success rate (>80%)
- Publishing latency (<10ms)
- Token cache population status

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

## Graceful Shutdown

The service implements graceful shutdown handling:
- Responds to SIGTERM and Ctrl+C signals
- Completes in-flight simulations
- Writes final statistics to summary log
- Properly closes all connections and file handles
- Aborts background token subscriber task

## Adaptive Backoff

The main processing loop implements adaptive backoff when no transactions are available:
- 1-10 empty batches: 100μs sleep
- 11-100 empty batches: 1ms sleep
- >100 empty batches: 10ms sleep

This ensures efficient CPU usage while maintaining low latency for new transactions.

## Future Enhancements

1. **Machine Learning**: Pattern recognition for new scam types
2. **Cross-Chain**: Monitor multiple chains simultaneously
3. **MEV Integration**: Detect sandwich attacks
4. **WebSocket API**: Alternative to ZMQ for web clients
5. **Distributed Processing**: Horizontal scaling for simulation
