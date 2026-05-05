# Mempool Processor - Real-Time Token Signal Detection System

## Overview

High-performance Rust system for real-time Ethereum mempool monitoring, transaction simulation, and automated signal detection. The system detects trading opportunities by analyzing mempool transactions, simulating their effects, and determining token tradability and tax rates.

## System Objective

Our end-to-end objective is straightforward:

> *Fuse the freshest canonical token/pool context (produced by the Python data
> pipeline) with in-flight mempool intelligence so we can buy the moment trading
> is enabled and exit the moment someone tries to scam us.*

To achieve that we:

1. **Continuously ingest canonical data** – the Python `eth_data` services watch
   newly mined blocks, derive token + pool metadata, and push those snapshots
   into the shared `TokenTrackingCache`.
2. **Continuously watch mempool deltas** – this Rust crate consumes the Reth
   mempool feed, classifies creator actions, simulates their effects on each
   tracked pool, and emits semantic signals.
3. **Act on convergence** – trading components listen to the signals
   (`TradingEnabled`, `HighTax`, `LiquidityRemoval`). When a signal lines up with
   the strategy we immediately execute the corresponding buy or sell.

Everything else in this repository (function detection, simulators, caches) is
in service of that control loop.

## Environment

Live block headers and processed transactions are sourced from a Redis instance
whose URL is provided via `LIVE_BLOCKCHAIN_DATA_REDIS_URL` (defaults to
`redis://localhost:6379/0`). Export this variable before launching the Python
publishers and Rust consumers so every component shares the same live chain
feed.

## 🚧 Next Steps (Live Scam Response)

### 1. Tip-State Mirror (block feed ➜ in-memory cache)

- Subscribe to the existing live block feed (Python publisher or Reth `CanonStateNotification`).
- For every canonical block event, hydrate a persistent `CacheDB<StateProviderDatabase>` using the block’s execution bundle. 
- Store the accompanying header (timestamp, base fee, gas limit, state root) alongside the cache. This combination becomes the authoritative “tip snapshot” without waiting for MDBX writes.

### 2. Pending-Tx Layer (mempool ➜ overlay replay)

- Maintain a lightweight queue of unmined transactions in canonical order (per sender nonce). 
- As new mempool transactions arrive, replay them into the same `CacheDB` overlay so approvals/allowances/transfer effects are visible immediately. 
- Provide an API to snapshot/rollback this overlay so simulations don’t permanently mutate the shared cache.

### 3. Header Injection (skip `header_by_number` hot path)

- Extend the tx simulator so callers can provide a `BlockHeader` directly. When present, skip the `header_by_number` lookup and instead build the EVM environment from the injected header.
- Use the tip snapshot header for mempool simulations; this guarantees a consistent base fee and parent hash even if MDBX is a block behind.

### 4. State Reset (new canonical block ➜ overlay prune)

- Upon the next canonical block notification: 
  - Drop any pending‑tx overlays that were layered on top of the previous tip. 
  - Rebuild the cache from the new block outcome. 
  - Re-apply outstanding mempool transactions whose nonces are still valid.

### 5. Integration Checklist

- [ ] Expose a `TipStateManager` struct responsible for block subscription, cache hydration, and overlay management.
- [ ] Update `MempoolSimulator` to request state/env from `TipStateManager` instead of `get_latest_block()/header_by_number`.
- [ ] Add metrics (tip-lag, overlay size, replay latency) to ensure the new path stays performant.
- [ ] Provide fallbacks (retry with canonical MDBX) if the tip snapshot becomes unavailable.

## ✅ Implementation Status & Known Gaps

- **Live head snapshots in place**: the `TxSimulator` itself now hydrates headers/state via the shared `LiveChainCache`. Simulations transparently replay ahead-of-MDBX blocks without wiring a separate canonical head tracker.
- **Only the latest header is cached**: we currently overwrite the snapshot on every new head. If callers need `tip-1`, we must extend the cache (for example, keep a short deque) because the previous header is not retained yet.
- **Contract-creation flow still stubbed**: `SimulationManager` warns and exits early for deployments. There is no deterministic address derivation, helper replay, or post-deploy per-pool viability check.
- **State-change extraction missing**: `simulate_mempool_tx_with_state_changes` returns an empty map; integrating the richer `tx_processor` diffs is still a TODO.
- **Signal payload tax fields rely solely on buy/sell probes**: we do not compute before/after deltas from state changes, so downstream consumers cannot see tax adjustments unless the probe succeeds.
- **Pool metadata in token cache is partially hardcoded**: V2 pools are tagged with `"V2"` and `last_update_block = 0` because the provenance supplied by the Python publisher is not persisted yet.
- **Pending sequence buffer only keeps unmined helpers**: creator transactions are tracked per `(creator, token)` while they remain in the mempool. A contract-creation tx is only retained if a trading-enablement helper lands in the same block; otherwise the Python token-tracking feed populates the token on the next block and the creation is dropped.

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
   
5. Signal: TaxSignal (HighTaxOrHoneypot)
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

### **2. Tax Signal Detector (Consolidated)**

**Purpose**: Emit a per‑pool tax signal when taxes are excessive or selling fails.

**Logic (as implemented):**
```
let cant_sell = !simulation.can_sell
let buy_exceeds = buy_tax >= max_acceptable_buy_tax
let sell_exceeds = sell_tax >= max_acceptable_sell_tax

if buy_exceeds || sell_exceeds || cant_sell:
    emit TaxSignal(HighTaxOrHoneypot)
else if (buy_tax <= 5%) and (sell_tax >= buy_tax + 20%):
    emit TaxSignal(SuspiciousPattern)
```

Notes:
- Honeypot is determined primarily by “cannot sell”.
- “Tax change” signaling exists in types but is not currently emitted.

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

### **4. Honeypot (via Tax Signal)**

Handled by the Tax Signal detector as `HighTaxOrHoneypot` when selling fails (cannot sell).

### **5. Tax Change Detector**

Reserved (types include `TaxChange`, but current implementation does not emit it).

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
│ reth.ipc        │    │    2-7μs        │    │ ┌─────────────┐ │
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

### End‑to‑End Pipeline Flow

```
Reth Node (IPC @ /home/nima/storage/samsung8tb/ethereum/reth/reth.ipc)
        │
        ▼
┌──────────────────────────────┐
│ MempoolFetcherIPCClient      │  Non‑blocking try_read() + JSON parse
│ - Produces MempoolTransaction│  (2–7µs socket read)
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ FunctionDetector              │  Populate tx.functions, function_category
└──────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐
│ TransactionRouter             │  Classify + priority
│ - ContractCreation            │
│ - CreatorTransaction          │  (e.g., enableTrading, liquidity removal)
│ - Dex/Other                   │
└──────────────────────────────┘
        │
        ├── (LP Approval, no simulation) ──────────────────────────────────────────────┐
        │                                                                             │
        │                                        ┌──────────────────────────────────┐  │
        │                                        │  SignalManager (direct path)     │  │
        │                                        │  • LpApprovalDetector            │  │
        │                                        │  • Publish via SignalPublisher   │  │
        │                                        └──────────────────────────────────┘  │
        │                                                                             │
        ▼                                                                             │
┌────────────────────────────────────────────────────────────────────────────────────┘
│ SimulationManager (integrated)
│  submit(request)
│  - Uses MempoolSimulator (shared Arc<TxSimulator>)
│  - LiquidityRemovalSimulator for removal flows
│  - Discovers pools via TokenTrackingCache for CreatorTransaction
│  - Runs per‑pool simulations concurrently
│  - Immediately dispatches EACH SimulationResult to SignalManager
└────────────────────────────────────────────────────────────────────────────────────
        │
        ▼
Per‑pool simulation (CreatorTransaction)
  ┌──────────────────────────────────────────────────────────────────────────────┐
  │ 1) Optional tx simulation (mempool) with nonce retry on “too high”         │
  │ 2) Build PoolBuySellParameters { token, pool, at_block?, … }                 │
  │ 3) Pool buy/sell via tx_processor (check_can_buy_sell_pool)                │
  │ 4) Build SimulationResult { token, pool, pool_viability_result, … }        │
  │ 5) Send to SignalManager                                                   │
  └──────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│ SignalManager                                                                     │
│  • TaxDetector                → HighTax / Honeypot signals                        │
│  • TradingStatusDetector      → TradingEnabled signals                            │
│  • LiquidityDetector          → Pool drain / scam detections                      │
│  • LpApprovalDetector         → LP approvals (direct path or from results)       │
│  → Publishes every signal via SignalPublisher                                     │
└──────────────────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────┐    ┌──────────────────────────────┐
│ SignalPublisher (logs)       │    │ SignalPublisher (ZMQ)        │
│  • signals/trading_enabled   │    │  tcp://127.0.0.1:5556        │
│  • signals/tax_signals       │    └──────────────────────────────┘
│  • signals/liquidity_removals│
│  • signals/scam_detections   │    (DB writer enabled by default)
│  • signals/lp_approval       │    • Toggle via SignalPublisherConfig.enable_database
└──────────────────────────────┘

Support services
  • TokenTrackingSubscriber → populates TokenTrackingCache from Python feeds (optional)
  • MempoolSimulator shares a single TxSimulator to avoid DB writer locks
  • At‑block simulation preferred when available to avoid nonce/basefee drift
```

### **Component Architecture (New Integrated Design)**

```
mempool_signal_detector (Single Binary)
│
├── 🔌 MempoolFetcherIPCClient           ──▶ /home/nima/storage/samsung8tb/ethereum/reth/reth.ipc
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
│   ├── MempoolSimulator                 ──▶ Shared TxSimulator (no DB locks)
│   ├── Buy/Sell Simulator               ──▶ Trading validation
│   ├── SignalManager (built-in)         ──▶ Automatic detection
│   └── TokenCache integration           ──▶ Context-aware signals
│
├── 🎯 Signal Detection (Automatic)      ──▶ Inside SimulationManager
│   ├── Trading Enabled (buy+sell OK)    ──▶ New tradeable tokens
│   ├── Tax Signal (high tax/honeypot)   ──▶ Excessive fees or cannot sell
│   ├── Liquidity/Scam Detection         ──▶ Pool drains (incl. dedicated removal)
│   └── LP Approval Detection            ──▶ Creator approving router to spend LP tokens
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
- **Non-blocking I/O**: Eliminates blocking read bottlenecks (via MempoolFetcherIPCClient)
- **Zero RPC Fallback**: Full transaction data in first request
- **Nanosecond Timing**: Precise performance measurement

### **Single-Processor Design**
- **Centralized Processing**: One main binary handles all transactions
- **Concurrent Pipeline**: tokio async runtime for parallel processing
- **Shared State**: Pool cache and signal engine across threads
- **Distributed Integration**: External services via ZMQ

### **Transaction Simulator**
- **MempoolSimulator**: wraps a single shared `Arc<TxSimulator>` used across components
- **DB Locks**: avoided by sharing one connection (no -30778 LMDB write lock)
- **Nonce Handling**: mempool tx path retries on "nonce too high"; at-block simulation preferred
- **State Tracking**: ETH transfers and ERC20 token movements
- **Pool Analysis**: Reserve changes and percentage calculations

### **Signal Categories**
1. **TradingEnabled**: Token becomes tradeable on a specific pool with acceptable taxes
2. **TaxSignal**: HighTaxOrHoneypot (incl. cannot sell) or SuspiciousPattern
3. **LiquidityRemoval**: ETH removed from pool (minor/significant/major)
4. **ScamDetected**: Pool drain >60% or <0.3 ETH remaining
5. **LpApproval**: Creator approves router to spend LP tokens

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
- **Reth Node**: Running with IPC enabled (`/home/nima/storage/samsung8tb/ethereum/reth/reth.ipc`)
- **Reth Database**: Read access to `/home/nima/storage/samsung8tb/ethereum/reth`
- **Shared Config**: `/home/nima/code/crypto/blockchains/eth/config.env`
- **Python Token Tracker**: Optional; when offline the cache is sparse but pipeline still runs
- **PostgreSQL**: Optional for audit logging (set `SignalPublisherConfig.enable_database = false` to skip)

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
# Shared Ethereum workspace config
export ETH_CONFIG_PATH="/home/nima/code/crypto/blockchains/eth/config.env"

# Token tracking service inputs
export TOKEN_TRACKING_PUB="tcp://localhost:5557"  # Token updates from Python
export REDIS_URL="redis://127.0.0.1:6379/0"      # Startup snapshots/index

# Signal publishing
export SIGNAL_ZMQ_ENDPOINT="tcp://127.0.0.1:5556"

# Run the main service
./target/release/mempool_signal_detector \
  --log-dir mempool_processor/logs \
  --batch-size 100 \
  --sim-workers 10
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

**TaxSignal (HighTaxOrHoneypot):**
```json
{
  "tx_hash": "0x7ea69e87...",
  "token_address": "0x8390a1DA...",
  "pool_address": "0xPool...",
  "pool_type": "V2",
  "creator_address": "0x97dC7F34...",
  "signal_type": "HighTaxOrHoneypot",
  "signal_details": "High sell tax: 99.0%",
  "confidence": 0.90,
  "buy_tax": 30.0,
  "sell_tax": 99.0,
  "buy_tax_exceeds_threshold": true,
  "sell_tax_exceeds_threshold": true,
  "cant_sell": false,
  "timestamp": 1705123456
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
        print(f"✅ Trading Enabled: token={signal['token_address']} pool={signal['pool_address']} (buy: {signal['buy_tax']}%, sell: {signal['sell_tax']}%)")
    elif topic == "tax_signal":
        print(f"⚠️ TaxSignal: token={signal['token_address']} pool={signal['pool_address']} type={signal['signal_type']} details={signal['signal_details']}")
    elif topic == "liquidity_removal":
        print(f"💧 Liquidity Removal: pool={signal['pool_address']}")
    elif topic == "scam_detection":
        print(f"🚨 Scam Detected: pool={signal['pool_address']} drained={signal['drain_percentage']}% remaining={signal['eth_remaining']} ETH")
    elif topic == "lp_approval":
        print(f"⚠️ LP Approval: creator={signal['creator']} lp_token={signal['lp_token_address']} router={signal['router_address']}")
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
│   ├── mempool_fetcher_ipc_client.rs              # ⚡ Ultra-fast IPC client
│   └── types.rs                                   # Transaction types
│
├── function_detector/                             # 🔍 Function signature detection
│   └── mod.rs                                     # Signature matching logic
│
├── tx_router/                                     # 🧭 Transaction routing
│   └── mod.rs                                     # Classification & priority
│
├── simulator/                                     # 🔬 Transaction simulation (per‑pool)
│   ├── mod.rs                                     # Module coordination
│   ├── mempool_simulator.rs                       # Reth DB simulator
│   ├── pool_buy_sell_simulator.rs                 # Buy/Sell per‑pool testing
│   ├── simulation_manager.rs                      # Integrated processing + signal dispatch
│   ├── simulation_queue.rs                        # Priority queue
│   └── liquidity_removal_simulator.rs             # Dedicated removal simulator
│
├── signal_detector/                               # 🎯 Signal detection (per‑pool)
│   ├── mod.rs                                     # Module exports
│   ├── signal_manager.rs                          # Signal coordination
│   ├── liquidity_detector.rs                      # Drains/removals + scams
│   ├── stablecoin_detector.rs                     # USDC/USDT activity
│   ├── trading_status_detector.rs                 # Trading enabled signals
│   ├── tax_signal_detector.rs                     # Consolidated tax signals
│   ├── lp_approval_detector.rs                    # LP approval detection
│   └── types.rs                                   # Signal definitions
│
├── token_tracking/                                # 💾 Token state management
│   ├── mod.rs                                     # ZMQ subscriber setup
│   ├── cache.rs                                   # TokenTrackingCache impl
│   └── types.rs                                   # Token data structures
│
├── signal_publisher.rs                            # 📡 Signal distribution (ZMQ/logs/DB)
│
├── token_parameter_extraction/                    # 📊 Token analysis
│   ├── mod.rs                                     # Module exports
│   └── tax_calculator.rs                          # Tax calculation logic
│
└── common/                                        # 🛠️ Shared utilities
    ├── address.rs                                 # Address formatting
    └── types.rs                                   # Common data types
```
