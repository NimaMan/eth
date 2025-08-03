# Mempool Processor - Real-Time Token Signal Detection System

## Overview

High-performance Rust system for real-time Ethereum mempool monitoring, transaction simulation, and automated signal detection. The system detects trading opportunities by analyzing mempool transactions, simulating their effects, and determining token tradability and tax rates.

## 🎯 Core Concept: Transaction Flow & Signal Detection

### **The Journey of a Transaction**

When a transaction appears in the Ethereum mempool, our system processes it through several stages to determine if it represents a trading opportunity:

```
1. Transaction Arrival (2-7μs)
   └─> IPC socket receives raw transaction data from Reth node
   
2. Function Detection (<10μs)
   └─> Identifies function calls: enableTrading(), removeLiquidity(), setTaxes(), etc.
   
3. Transaction Routing (<1ms)
   └─> Classifies transaction type and assigns priority:
       • Contract Creation → New token deployment
       • Creator Transaction → Token owner/creator actions
       • Regular Transaction → Swaps, transfers (often skipped)
       
4. Simulation & Analysis (5-10ms)
   └─> Executes transaction + buy/sell tests
   └─> Calculates actual tax rates from simulation
   └─> Determines if token is tradeable
   
5. Signal Detection (Context-Aware)
   └─> Compares simulation results with token history
   └─> Detects state changes: trading enabled, tax changes, honeypots
   └─> Emits binary signals based on thresholds
```

### **Example Data Flows**

#### **Flow 1: New Token Launch**
```
1. Contract Creation TX detected
   • From: 0xCreator123...
   • To: null (deployment)
   • Input: Token bytecode
   
2. Router classifies as "ContractCreation"
   • Priority: HIGH
   • Contract address: 0xNewToken456...
   
3. Simulation runs buy/sell test
   • Buy 0.1 ETH worth → Success, received 1M tokens
   • Sell 500K tokens → Success, received 0.045 ETH
   • Calculated buy tax: 5%
   • Calculated sell tax: 10%
   
4. Signal: TRADING_ENABLED
   • Token is tradeable
   • Taxes are reasonable (<25%)
   • Creator still owns the token
```

#### **Flow 2: Trading Enabled on Existing Token**
```
1. Function call detected
   • From: 0xTokenOwner789...
   • To: 0xToken123...
   • Function: enableTrading()
   
2. Router classifies as "CreatorTransaction"
   • Priority: CRITICAL (trading status change)
   • Token context loaded from cache
   
3. Token context from cache shows:
   • Trading was previously disabled
   • Owner matches transaction sender
   • Token has liquidity pool with 5 ETH
   
4. Simulation confirms tradability
   • Buy test → Success
   • Sell test → Success
   • No tax changes detected
   
5. Signal: TRADING_ENABLED
   • Previously untradeable token now tradeable
   • Pool has sufficient liquidity
```

#### **Flow 3: Honeypot Detection**
```
1. Tax setter function detected
   • From: 0xScammer...
   • To: 0xToken789...
   • Function: setSellTax(99)
   
2. Router identifies creator action
   • Priority: CRITICAL (tax change)
   
3. Token context shows:
   • Trading was previously enabled
   • Previous sell tax: 5%
   
4. Simulation reveals honeypot
   • Buy test → Success
   • Sell test → Fails or returns minimal ETH
   • Calculated sell tax: 99%
   
5. Signal: HIGH_TAX_WARNING / HONEYPOT
   • Token no longer sellable
   • Sell tax exceeds 50% threshold
```

### **Key Signal Types**

1. **TRADING_ENABLED**
   - Conditions: Buy succeeds AND sell succeeds AND taxes ≤ 25%
   - Context: Can be new token OR previously disabled token
   - Use case: Enter positions in newly tradeable tokens

2. **HIGH_TAX_WARNING**  
   - Conditions: Buy tax > 25% OR sell tax > 25%
   - Sub-type: HONEYPOT if sell tax > 50% or sell fails
   - Use case: Avoid tokens with excessive taxes

3. **LIQUIDITY_REMOVAL**
   - Conditions: removeLiquidity function AND pool has > 0.05 ETH
   - Context: Pool reserves decreasing significantly
   - Use case: Exit positions before rug pull

### **Context-Aware Detection**

The system uses the Token Tracking Cache to provide context:

```
Token Cache provides:
├── Current trading status (enabled/disabled)
├── Current owner and creator addresses
├── Historical tax rates
├── Pool addresses and reserves
└── Previous simulation results

This context enables detection of CHANGES:
• Was not tradeable → Now tradeable = SIGNAL
• Was 5% tax → Now 99% tax = SIGNAL  
• Had 10 ETH liquidity → Now 0.1 ETH = SIGNAL
```

### **Why Simulation Matters**

Function names can lie, but simulation reveals truth:

```
Example 1: Deceptive "enableTrading()"
• Function called: enableTrading()
• Simulation result: Buy fails
• Reality: Trading not actually enabled
• Signal: NONE (no false positive)

Example 2: Hidden tax implementation
• Function called: transfer()
• Simulation result: 90% tokens disappear
• Reality: Hidden tax in transfer function
• Signal: HIGH_TAX_WARNING

Example 3: Complex tax calculation
• Contract has dynamic tax based on holder count
• Simple contract read would miss this
• Simulation captures actual tax rate
• Signal: Accurate tax percentage
```

## 📡 Signal Detection Algorithms

### **1. Trading Enabled Detector**

**Purpose**: Detect when a token becomes tradeable with reasonable taxes.

**Algorithm**:
```
IF (simulation.can_buy AND simulation.can_sell) THEN
    IF (buy_tax ≤ 25% AND sell_tax ≤ 25%) THEN
        IF (token_cache.was_not_tradeable OR is_new_token) THEN
            EMIT TradingEnabledSignal
        END IF
    END IF
END IF
```

**Context Requirements**:
- Previous trading status from token cache
- Calculated tax rates from simulation
- Token creation block for new token detection

### **2. High Tax Warning Detector**

**Purpose**: Alert when tokens have excessive taxes that make trading unprofitable.

**Algorithm**:
```
IF (buy_tax > 25% OR sell_tax > 25%) THEN
    severity = "WARNING"
    
    IF (sell_tax > 50% OR NOT simulation.can_sell) THEN
        severity = "CRITICAL"
        type = "HONEYPOT"
    END IF
    
    IF (token_cache.previous_tax < current_tax) THEN
        EMIT HighTaxWarning(severity, type, tax_change)
    END IF
END IF
```

**Thresholds**:
- Warning: >25% tax
- Critical: >50% sell tax
- Honeypot: Cannot sell OR sell tax >50%

### **3. Liquidity Removal Detector**

**Purpose**: Detect when liquidity is being removed from pools.

**Algorithm**:
```
IF (function IN ["removeLiquidity", "removeLiquidityETH", "decreaseLiquidity"]) THEN
    pool = token_cache.get_pool(tx.to)
    
    IF (pool.eth_reserve > 0.05 ETH) THEN
        IF (tx.from IN [pool.creator, pool.owner]) THEN
            severity = "CRITICAL"
        ELSE
            severity = "HIGH"
        END IF
        
        EMIT LiquidityRemovalSignal(pool, severity)
    END IF
END IF
```

**Context Checks**:
- Pool must have minimum 0.05 ETH
- Creator/owner removals are more critical
- Pool address must be known in cache

### **4. Honeypot Detector**

**Purpose**: Identify tokens that can be bought but not sold.

**Algorithm**:
```
IF (simulation.can_buy AND NOT simulation.can_sell) THEN
    IF (token_cache.was_previously_sellable) THEN
        // Token turned into honeypot
        EMIT HoneypotSignal(severity="CRITICAL", type="CHANGED")
    ELSE IF (is_new_token) THEN
        // New honeypot token
        EMIT HoneypotSignal(severity="HIGH", type="NEW")
    END IF
END IF

// Alternative: Extreme sell tax
IF (sell_tax > 90% AND buy_tax < 25%) THEN
    EMIT HoneypotSignal(severity="HIGH", type="HIGH_TAX")
END IF
```

### **5. Tax Change Detector**

**Purpose**: Alert on significant tax rate changes.

**Algorithm**:
```
previous_buy_tax = token_cache.get_buy_tax()
previous_sell_tax = token_cache.get_sell_tax()

buy_tax_change = ABS(current_buy_tax - previous_buy_tax)
sell_tax_change = ABS(current_sell_tax - previous_sell_tax)

IF (buy_tax_change > 5% OR sell_tax_change > 5%) THEN
    IF (current_tax > previous_tax) THEN
        direction = "INCREASED"
        severity = "HIGH"
    ELSE
        direction = "DECREASED"
        severity = "MEDIUM"
    END IF
    
    EMIT TaxChangeSignal(direction, severity, old_tax, new_tax)
END IF
```

### **Signal Emission Rules**

1. **No Duplicate Signals**: Check recent signal history before emitting
2. **Context Required**: Never emit without token cache context
3. **Binary Decision**: Signal is either emitted or not (no confidence scores)
4. **Immediate Publishing**: Signals are published as soon as detected

## 🏗️ System Architecture

### **Data Flow Pipeline**

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Ethereum Node │    │  UltraFastClient │    │ Signal Detection│
│                 │    │                  │    │     Engine      │
│ ┌─────────────┐ │    │ ┌──────────────┐ │    │ ┌─────────────┐ │
│ │   Mempool   │─┼───▶│ │ IPC Socket   │─┼───▶│ │ Simulation  │ │
│ │             │ │    │ │ Non-blocking │ │    │ │ Engine      │ │
│ │  New Txs    │ │    │ │ JSON Parser  │ │    │ │             │ │
│ └─────────────┘ │    │ └──────────────┘ │    │ └─────────────┘ │
│                 │    │                  │    │        │        │
│ Reth Node       │    │ Detection Time:  │    │        ▼        │
│ /tmp/reth.ipc   │    │    2-7μs        │    │ ┌─────────────┐ │
└─────────────────┘    └──────────────────┘    │ │ Pool State  │ │
                                               │ │ Analysis    │ │
                                               │ └─────────────┘ │
                                               │        │        │
                                               │        ▼        │
                                               │ ┌─────────────┐ │
                                               │ │ Trading     │ │
                                               │ │ Signals     │ │
                                               │ └─────────────┘ │
                                               └─────────────────┘
                                                       │
                                                       ▼
               ┌─────────────────────────────────────────────────────┐
               │              Alert Distribution                      │
               │                                                     │
               │  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐ │
               │  │ PostgreSQL  │  │ ZMQ Publisher│  │ Log Files   │ │
               │  │ Database    │  │ (Trading     │  │ (Analysis)  │ │
               │  │ (Audit)     │  │ Bots)        │  │             │ │
               │  └─────────────┘  └──────────────┘  └─────────────┘ │
               └─────────────────────────────────────────────────────┘
```

### **Component Architecture (New Integrated Design)**

```
mempool_signal_detector (Single Binary)
│
├── 🔌 NonBlockingIpcClient              ──▶ /tmp/reth.ipc
│   ├── Non-blocking socket reads        ──▶ 2-7μs detection
│   ├── JSON streaming parser            ──▶ Zero-copy parsing
│   └── Auto-reconnect on failure        ──▶ Resilient connection
│
├── 🧭 TransactionRouter                 ──▶ Classification
│   ├── Function signature detection     ──▶ <10μs latency
│   ├── Priority assignment              ──▶ Critical/High/Normal
│   └── Category routing                 ──▶ Contract/Creator/DEX
│
├── 🔧 SimulationManager (Integrated)    ──▶ All-in-one processing
│   ├── TxSimulator                      ──▶ Reth DB direct access
│   ├── BuySellSimulator                 ──▶ Trading validation
│   ├── SignalManager (built-in)         ──▶ Automatic detection
│   └── TokenCache integration           ──▶ Context-aware signals
│
├── 🎯 Signal Detection (Automatic)      ──▶ Inside SimulationManager
│   ├── Trading Enabled (buy+sell OK)    ──▶ New tradeable tokens
│   ├── Honeypot Detection               ──▶ Can't sell anymore
│   ├── High Tax Warning (>25%)          ──▶ Excessive fees
│   └── Liquidity/Scam Detection         ──▶ Pool drains
│
├── 💾 TokenTrackingCache                ──▶ Comprehensive token data
│   ├── Token information + simulation   ──▶ Full token state
│   ├── Creator addresses (HashSet)      ──▶ O(1) creator lookups
│   ├── Pool states (HashMap)            ──▶ Real-time reserves
│   ├── Tax information storage          ──▶ Buy/sell tax rates
│   └── get_all_token_addresses()        ──▶ Unique token list
│
└── 📡 SignalPublisher                   ──▶ Multi-channel output
    ├── ZMQ publisher (tcp://127.0.0.1:5556) ──▶ Real-time alerts
    ├── Log files                        ──▶ Audit trail
    └── Metric counters                  ──▶ Performance tracking
```

**Key Improvement**: Signal detection is now integrated directly into the 
SimulationManager, eliminating the need for result polling and separate 
signal processing steps.

## 🎯 Core Features

### **Simplified API with Integrated Detection**
The new architecture dramatically simplifies the main processing loop:

```rust
// OLD: Complex multi-step process
let results = simulation_manager.process_queue().await;
for result in results {
    let signals = signal_manager.process_result(result);
    publisher.publish(signals);
    // Handle errors, update metrics...
}

// NEW: Single submit call - everything handled internally
simulation_manager.submit(request).await?;
// That's it! Simulation, detection, and publishing all automatic
```

### **Ultra-Fast Detection**
- **IPC Integration**: Direct Unix socket connection to Reth node
- **Non-blocking I/O**: Eliminates blocking read bottlenecks
- **Zero RPC Fallback**: Full transaction data in first request
- **Nanosecond Timing**: Precise performance measurement

### **Single-Processor Design**
- **Centralized Processing**: One main binary handles all transactions
- **Concurrent Pipeline**: tokio async runtime for parallel processing
- **Shared State**: Pool cache and signal engine across threads
- **Distributed Integration**: External services via ZMQ

### **Transaction Simulator**
- **Primary Method**: debug_traceCall RPC (~3.6ms average)
- **Alternative**: REVM simulation (~40-50ms) - not used in production
- **State Tracking**: ETH transfers and ERC20 token movements
- **Pool Analysis**: Reserve changes and percentage calculations

### **Signal Categories**
1. **TradingEnabled**: Token becomes tradeable with taxes ≤25%
2. **HighTaxWarning**: Buy or sell tax exceeds 25%
3. **Honeypot**: Sell tax >50% or cannot sell
4. **LiquidityRemoval**: Significant ETH removed from pools
5. **ScamDetected**: Pool drain >60% or <0.3 ETH remaining

## 🚀 Performance Characteristics

### **Latency Breakdown**
| Stage | Average | Maximum | Notes |
|-------|---------|---------|-------|
| IPC Reception | 5μs | 40μs | Raw transaction from mempool |
| Function Detection | 5μs | 256μs | Pattern matching on calldata |
| Transaction Routing | <1ms | 2ms | Classification and priority |
| Simulation | 5-10ms | 50ms | Transaction + buy/sell tests |
| Signal Detection | <1ms | 5ms | Context lookup + logic |
| **Total Pipeline** | **6-12ms** | **60ms** | End-to-end |

### **Throughput**
- **IPC Reception**: 700+ tx/sec
- **Function Detection**: 200,000+ tx/sec  
- **Simulation**: 100-200 tx/sec (bottleneck)
- **Signal Publishing**: 10,000+ signals/sec

### **Resource Usage**
- **Memory**: ~500MB steady state
- **CPU**: 2-4 cores utilized
- **Network**: <10 Mbps (IPC + ZMQ)

## 🚀 Quick Start

### Prerequisites
- **Rust**: 1.70+ with cargo
- **Reth Node**: Running with IPC enabled (`/tmp/reth.ipc`)
- **Reth Database**: Read access to `/home/nima/.local/share/reth/mainnet`
- **Python Token Tracker**: Publishing tokens on `tcp://localhost:5557-5558`
- **PostgreSQL**: Optional for audit logging

### Installation
```bash
# Clone and build
git clone <repository>
cd mempool_processor
cargo build --release

# Production binary
./target/release/mempool_signal_detector
```

### Configuration
```bash
# Environment variables (optional)
export ETH_RPC_URL="http://localhost:8545"
export IPC_PATH="/tmp/reth.ipc"
export RETH_DB_PATH="/home/nima/.local/share/reth/mainnet"

# Token tracking service endpoints
export TOKEN_TRACKING_PUB="tcp://localhost:5557"  # Token updates from Python
export TOKEN_TRACKING_REP="tcp://localhost:5558"  # Query endpoint

# Signal publishing
export SIGNAL_ZMQ_ENDPOINT="tcp://127.0.0.1:5556"

# Run the main service
./target/release/mempool_signal_detector \
  --ipc-path /tmp/reth.ipc \
  --reth-db-path /home/nima/.local/share/reth/mainnet \
  --log-dir /home/nima/code/crypto/logs/mempool \
  --batch-size 100 \
  --sim-workers 10
```

## 📊 Performance Monitoring

### **Real-time Metrics**
```bash
# View timing reports (every 1000 transactions)
tail -f /home/nima/code/crypto/logs/mempool/timing_reports_full_tx_*.log

# Monitor scam detections
tail -f /home/nima/code/crypto/logs/mempool/scam_alerts_full_tx_*.log

# Watch market events
tail -f /home/nima/code/crypto/logs/mempool/market_events_full_tx_*.log
```

### **Sample Timing Report**
```
[2025-07-02 17:19:57.424] ============================================================
[2025-07-02 17:19:57.424] IPC Full: 2000 transactions, avg latency: 12μs, queue: 0/50000
[2025-07-02 17:19:57.424] ⚡ TIMING REPORT (last 1101 transactions):
[2025-07-02 17:19:57.424]    📡 IPC Detection:   avg=0.01ms  max=0.04ms
[2025-07-02 17:19:57.424]    🔬 Simulation:      avg=6.55ms  max=864.83ms
[2025-07-02 17:19:57.424]    🔍 Pool Check:      avg=0.00ms  max=0.02ms
[2025-07-02 17:19:57.424]    🛡️  Scam Detection: avg=0.00ms  max=0.10ms
[2025-07-02 17:19:57.424]    📊 Total Pipeline:  avg=6.63ms  max=864.93ms
[2025-07-02 17:19:57.424]    🎯 Pools Affected:  0 (0.0%)
[2025-07-02 17:19:57.424]    🚀 Throughput:      150.7 tx/sec
[2025-07-02 17:19:57.424] ============================================================
```

## 📡 Trading Signal Integration

### **ZMQ Signal Publishing**

Signals are published to `tcp://127.0.0.1:5556` as topic-prefixed JSON:

**Trading Enabled Signal:**
```json
{
  "tx_hash": "0x7ea69e87...",
  "token_address": "0x8390a1DA...",
  "creator_address": "0x97dC7F34...",
  "buy_tax": 5,
  "sell_tax": 10,
  "timestamp": 1705123456,
  "block_number": 22832374
}
```

**High Tax Warning Signal:**
```json
{
  "tx_hash": "0x7ea69e87...",
  "token_address": "0x8390a1DA...",
  "creator_address": "0x97dC7F34...",
  "buy_tax": 30,
  "sell_tax": 99,
  "warning_type": "PotentialHoneypot",
  "timestamp": 1705123456,
  "block_number": 22832374
}
```

### **Python Subscriber Example**
```python
import zmq
import json

context = zmq.Context()
subscriber = context.socket(zmq.SUB)
subscriber.connect("tcp://localhost:5556")
subscriber.setsockopt_string(zmq.SUBSCRIBE, "")

while True:
    # Receive topic and message
    topic = subscriber.recv_string()
    message = subscriber.recv_string()
    signal = json.loads(message)
    
    if topic == "trading_enabled":
        print(f"✅ Trading Enabled: {signal['token_address']} (buy: {signal['buy_tax']}%, sell: {signal['sell_tax']}%)")
    elif topic == "high_tax_warning":
        print(f"⚠️ High Tax: {signal['token_address']} - {signal['warning_type']}")
    elif topic == "liquidity_removal":
        print(f"💧 Liquidity Removal: {signal['pool_address']} - {signal['eth_change']:.2f} ETH")
```

## 📁 Project Structure

```
src/
├── bin/
│   ├── mempool_signal_detector.rs                # 🎯 Main production binary
│   └── README.md                                  # Binary-specific documentation
│
├── mempool_fetcher/                               # 🔌 Transaction detection
│   ├── mod.rs                                     # Module exports
│   ├── nonblocking_ipc_client.rs                 # ⚡ Ultra-fast IPC client
│   └── types.rs                                   # Transaction types
│
├── function_detector/                             # 🔍 Function signature detection
│   └── mod.rs                                     # Signature matching logic
│
├── tx_router/                                     # 🧭 Transaction routing
│   └── mod.rs                                     # Classification & priority
│
├── simulator/                                     # 🔬 Transaction simulation
│   ├── mod.rs                                     # Module coordination
│   ├── tx_simulator.rs                            # Reth DB simulator
│   ├── buy_sell_sequence_simulator.rs            # Trading validation
│   ├── simulation_manager.rs                      # Integrated processing
│   └── simulation_queue.rs                        # Priority queue
│
├── signal_detector/                               # 🎯 Signal detection
│   ├── mod.rs                                     # Module exports
│   ├── signal_manager.rs                          # Signal coordination
│   ├── liquidity_detector.rs                      # Pool drain detection
│   ├── stablecoin_detector.rs                     # USDC/USDT activity
│   ├── trading_status_detector.rs                 # Trading enabled signals
│   ├── tax_change_detector.rs                     # Tax modification alerts
│   ├── honeypot_detector.rs                       # Scam detection
│   └── types.rs                                   # Signal definitions
│
├── token_tracking/                                # 💾 Token state management
│   ├── mod.rs                                     # ZMQ subscriber setup
│   ├── cache.rs                                   # TokenTrackingCache impl
│   └── types.rs                                   # Token data structures
│
├── signal_publisher/                              # 📡 Signal distribution
│   └── mod.rs                                     # ZMQ publisher
│
├── token_parameter_extraction/                    # 📊 Token analysis
│   ├── mod.rs                                     # Module exports
│   └── tax_calculator.rs                          # Tax calculation logic
│
└── common/                                        # 🛠️ Shared utilities
    ├── address.rs                                 # Address formatting
    └── types.rs                                   # Common data types
```

## 🔧 Configuration Reference

### **Detection Thresholds**
```rust
// Trading signals
max_acceptable_buy_tax: 25%          // Trading enabled if ≤25%
max_acceptable_sell_tax: 25%         // Trading enabled if ≤25%

// Honeypot detection
honeypot_sell_threshold: 50%         // Honeypot if sell tax >50%

// Liquidity/scam detection  
scam_drain_threshold: 60%            // Scam if >60% drained
min_eth_threshold: 0.3 ETH           // Scam if <0.3 ETH remaining
major_removal_threshold: 50%         // Major removal signal
significant_removal_threshold: 20%   // Significant removal signal
min_pool_eth: 0.05 ETH              // Minimum pool size to track

// Cache limits (hardcoded - TODO: make configurable)
max_pools: 100_000                   // Pool state cache
max_creators: 50_000                 // Token creator cache
```

### **Performance Tuning**
```rust
// Ultra-fast client settings
const BUFFER_SIZE: usize = 8192;           // Socket read buffer
const QUEUE_CAPACITY: usize = 50_000;      // Transaction queue
const TIMEOUT_MS: u64 = 100;               // Non-blocking timeout

// Pool cache settings  
const CACHE_TTL_BLOCKS: u64 = 5;           // Cache refresh interval
const MAX_CACHED_POOLS: usize = 10_000;    // Memory limit
```

## 🛠️ Development

### **Build & Test**
```bash
# Development build
cargo build

# Production build (optimized)
cargo build --release

# Run tests
cargo test

# Lint and check
cargo clippy
cargo check
```

### **Adding New Signal Types**
1. Define event in `src/signal_engine/types.rs`
2. Implement detection in `src/signal_engine/engine.rs`
3. Add threshold to `SignalThresholds` struct
4. Update publisher filtering if needed

### **Performance Analysis**
```bash
# Run with detailed timing
RUST_LOG=debug ./target/release/mempool_signal_detector

# Profile memory usage
valgrind --tool=massif ./target/release/mempool_signal_detector

# Benchmark detection latency
cargo run --example test_buy_sell_simulator
```

## 📈 Production Deployment

### **System Requirements**
- **CPU**: 2+ cores, <1% sustained usage
- **RAM**: 64MB+ (27MB baseline + buffer)  
- **Network**: <10ms RTT to Reth node
- **Storage**: 1GB+ for logs and database

### **Deployment Checklist**
- [ ] Reth node running with IPC enabled at `/tmp/reth.ipc`
- [ ] Reth database accessible at `/home/nima/.local/share/reth/mainnet`
- [ ] Python token tracking service running on ports 5557-5558
- [ ] Log directory `/home/nima/code/crypto/logs/mempool/` exists
- [ ] ZMQ signal publisher port 5556 available
- [ ] Sufficient disk space for logs (1GB+)

### **Production Optimizations**
- Use `--release` build for maximum performance
- Enable `--enable-publisher` for trading integration
- Monitor log files for performance degradation
- Set up alerting on detection latency >10ms

## 🔗 Integration

### **External Systems**
- **[eth_kartal](../eth_kartal)**: Transaction execution engine
- **[revm_tx_simulator](../revm_tx_simulator)**: Alternative simulator
- **[sarigoz](../../py/sarigoz)**: Web analytics dashboard

### **Data Sources**
- **Reth Node**: Primary mempool and RPC data
- **Python Pool Service**: Real-time pool state via ZMQ
- **PostgreSQL**: Persistent storage and audit trail

### **Output Consumers**
- **Trading Bots**: Real-time alerts via ZMQ
- **Analytics Systems**: Log file processing
- **Monitoring**: HTTP metrics endpoint

## 📜 License

Proprietary - Internal use only