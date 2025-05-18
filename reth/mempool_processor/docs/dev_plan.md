# Ethereum Mempool Processor Development Plan

## Project Overview
The Ethereum Mempool Processor is a high-performance Rust application that monitors Ethereum's mempool for liquidity removal transactions that can be front-run or back-run for profit.

## Requirements and Performance Targets
- **Core (Rust)**: ≤100ms processing time, captures transactions, simulates execution
- Capture new transactions: ≤5ms from mempool appearance
- Simulation time: ≤50ms per transaction
- Tx Submission to Flashbots: ≤10ms
- Total end-to-end latency: ≤100ms

# Architecture
1. **mempool_processor**:
   - Types module (shared data structures)
   - Fetcher module (retrieves mempool transactions)
   - Processor module (filters transactions)
   - Pool Subscriber module (receives pool updates from Python)
   - DB Logger module (records scam alerts) ✅
 
2. **tx_simulator**
   - Simulator module (tracks addresses and simulates transactions)
   - State Diff module (extracts per-address balance changes using REVM)
   - State Cache module (in-memory, limited-size queue with FIFO behavior)

3. **scam_detection**
   - Engine module (compares state diffs to pool levels)
   - Types module (shared data structures for scam detection)


## Development Milestones

### Phase 1: Core Rust Hot‑Path  — *"Don't get scammed"*

| Goal | Component | Success Criteria | Status |
|------|-----------|------------------|--------|
| Capture every pending tx within ≤20 ms | **Mempool Listener** (Rust, dev‑p2p) | >99 % of tx hashes seen before they land on‑chain | devp2p remains |
| Simulate tx locally | **revm Simulator** | Extract per‑address balance / storage deltas in ≤50 ms | ✅ Completed |
| Track state changes | **State Diff Tracker** | Extract per-address balance/storage changes | ✅ Completed |
| Aggregate state diffs | **State Cache** | Maintain per-address aggregated state changes | ✅ Completed |

### Current Bottleneck

The current bottleneck is the transaction fetching. We are using the devp2p protocol to fetch transactions. This is a slow process. We need to switch to the RPC protocol.


---

### Phase 2: Python-Rust Integration Flow

1.  **Bidirectional Communication & Data Handling**:
    *   Implement ZeroMQ PUB/SUB for push notifications:
        *   Python Publisher: ✅ (Confirmed running)
        *   Rust `PoolSubscriber`: ✅ (Connection, message reception, and deserialization working)
    *   **Implement serialization/deserialization for pool updates (JSON):**
        *   Python (Serialization): ✅ (Publishing JSON confirmed)
        *   Rust (Deserialization in `PoolSubscriber`): ✅ (Successfully parsing JSON messages)
    *   Test communication channels:
        *   Python Publisher to standalone Python Subscriber: ✅
        *   Python Publisher to standalone Rust `PoolSubscriber`: ✅ (Verified with `test_pool_subscriber.rs`)

2.  **Pool Data Processing & State Management in Rust**:
    *   Process deserialized pool updates from `PoolSubscriber`: ✅ (Implemented & tested)
    *   Create/Refine data structures for pool state storage: ✅ (Implemented in `types.rs`)
    *   Implement thread-safe `PoolStateCache`: ✅ (Implemented in `cache.rs`)
    *   Implement pruning and maintenance logic for the pool state cache: ✅ (Implemented)
    *   Test pool state cache with incoming live data: ✅ (Verified with `test_pool_subscriber.rs`)

3.  **Mempool Processor Core Integration**: 🔄 **IN PROGRESS**
    *   Create a `ScamDetectionEngine` component: 🔄 In Progress
        *   Define interfaces to access both mempool state diffs and pool levels ✅
        *   Implement scam detection rules (comparing simulated levels to current levels) ✅
        *   Add configuration options for thresholds and detection parameters 🔄
    *   Integrate `PoolSubscriber` and `PoolStateCache` into main application: 📝 To Do
        *   Create service to run as a concurrent task ✅
        *   Connect service to main application flow 📝 To Do
    *   Implement PostgreSQL database logging for scam alerts: ✅ Completed
        *   Create DbLogger component for database interactions ✅
        *   Add error handling and connection management ✅
        *   Implement test utilities for verification ✅
    *   Integrate scam detection with database logging: 📝 **NEXT STEP**
        *   Connect ScamDetectionEngine output to DbLogger 📝 To Do
        *   Implement service for continuous monitoring 📝 To Do
        *   Add monitoring and metrics 📝 To Do

4.  **Full System Integration & Refinement**:
    *   Create a unified service that connects:
        *   PoolSubscriber (ZeroMQ) 📝 To Do
        *   ScamDetectionEngine 📝 To Do
        *   DbLogger 📝 To Do
    *   Add configuration system for all components 📝 To Do
    *   End-to-end testing with live network data 📝 To Do
    *   Performance optimization to meet latency targets 📝 To Do
    *   Address compiler warnings for a clean build 📝 To Do

---

### Phase 3: Tx Submission to Flashbots

Once phases 1–2 are stable.
* Build Flashbots bundle in Rust and send with `eth_sendBundle`.

---


## Implementation Progress

### Phase 1: Transaction Processing Flow
1. **Mempool Monitoring**:
   - Connect to Ethereum node via WebSocket ✅
   - Subscribe to pending transactions ✅
   - Implement fallback polling mechanism ✅

2. **Transaction Simulation**:
   - Use revm to simulate transaction execution ✅
   - Configure EVM environment with proper transaction parameters ✅
   - Handle both success and revert cases ✅
   
3. **State Diff Tracking**:
   - Extract balance changes for all affected addresses ✅
   - Track storage slot modifications ✅
   - Calculate ETH value changes ✅
   
4. **State Cache Implementation**:
   - Design in-memory state cache similar to Python's MempoolTxCache ✅
   - Implement limited-size transaction queue with FIFO behavior ✅
   - Aggregate state diffs by address for cumulative change tracking ✅
   - Track first-seen timestamps for transactions ✅

### Phase 2: Python-Rust Integration Progress
1. **Pool State Monitoring**:
   - Implement ZeroMQ subscriber for pool updates ✅
   - Create thread-safe pool state cache ✅
   - Process and store pool level information ✅

2. **Scam Detection Logic**:
   - Implement engine to analyze pool state ✅
   - Add comparison between current and simulated state ✅
   - Set up configurable thresholds for detection ✅

3. **Database Integration**:
   - Implement PostgreSQL logger for scam alerts ✅
   - Add connection management and error handling ✅
   - Create testing utilities for verification ✅

4. **Next Steps**:
   - Connect ScamDetectionEngine to DbLogger
   - Create a service that integrates pool updates, scam detection, and database logging
   - Implement monitoring and metrics for the integrated system

---

# Performance 

## Current Performance Metrics
- **Transaction Fetching**: ~178.58ms (bottleneck)
- **Transaction Simulation**: ~0.06ms
- **State Diff Calculation**: ~0.01ms
- **Success Rate**: 99.60% of transactions processed correctly

## Optimization Results

### Comprehensive Benchmark (1,000 transactions)
| Metric | Batch Mode | Non-Batch Mode | Improvement |
|--------|------------|----------------|-------------|
| Fetch Time (ms) | 402.00 | 732.00 | 45.08% |
| Simulation Time (ms) | 0.51 | 0.73 | 29.75% |
| Total Processing Time (ms) | 0.55 | 0.75 | 26.76% |
| Throughput (tx/sec) | 1818.18 | 1331.56 | 36.55% |

### Detailed Fetch Time Statistics
| Statistic | Batch Mode | Non-Batch Mode |
|-----------|------------|----------------|
| Median | 402.00 | 732.00 |
| Std Dev | 0.00 | 0.00 |
| Min | 402.00 | 732.00 |
| Max | 402.00 | 732.00 |
| P95 | 402.00 | 732.00 |
| P99 | 402.00 | 732.00 |

### Performance Analysis
- **Transaction Fetching**: Batch mode is 45.08% faster for fetching transactions, confirming our optimization approach is effective
- **Simulation Time**: Batch mode is 29.75% faster for transaction simulation
- **Total Processing Time**: Batch mode is 26.76% faster for overall transaction processing
- **Throughput**: Batch mode achieves 36.55% higher throughput
- **Consistency**: Both approaches show very consistent performance with minimal variance (standard deviation near zero)

### Key Findings
1. **Batch Processing Superiority**: The batch approach consistently outperforms the non-batch method across all key metrics:
   - Faster fetching (45.08% improvement)
   - Faster simulation (29.75% improvement)
   - Higher overall throughput (36.55% improvement)

---

# Current Challenges and Solutions

## State Synchronization Challenge

### Problem Observed
We've identified a critical issue with transaction simulation: some transactions fail in our simulator with `LackOfFundForMaxFee` errors but successfully execute on-chain. For example:

```
EVM transaction error for tx c42bc2aec5458d4d35f0762765021fbbbabc30a4e562bb42a66a38a3075b9d59: 
Transaction(LackOfFundForMaxFee { fee: 562363792966396000, balance: 3664167499823000 })
```

When examining this transaction's history:
1. The account received 1.16431821 ETH at 9:20:47
2. Sent 0.598241 ETH at 9:23:35
3. Received a small amount (0.00000598 ETH) at 9:24:59
4. Sent 0.562309 ETH at 9:25:47 (this transaction was successfully mined but our simulator rejected it)

This pattern occurs because the simulation is working with stale or incorrect state data.


### Simulator Assessment

#### Current State
- Simulator uses REVM for execution environment
- State is fetched at the time of simulation, not cached in advance
- Simulation takes ~0.06ms per transaction (excellent performance)
- Successfully identifies state changes in most cases (>99%)

#### Limitations
1. **Stale State Data**: The simulator uses a point-in-time state snapshot that can be outdated by several blocks
2. **No Pending Transaction Context**: Doesn't consider other pending transactions affecting the same accounts
3. **Fee Calculation Issues**: Incorrectly interprets transaction value as part of fee requirement
4. **State Timing Mismatch**: Can't account for rapid account balance changes (common in MEV-heavy environments)

### Fetcher Assessment

#### Current State
- Uses RPC batch processing for efficiency
- Achieves ~178ms fetch time (still our bottleneck)
- Successfully retrieves >99% of transactions
- Has been optimized by 45% through batch mode
