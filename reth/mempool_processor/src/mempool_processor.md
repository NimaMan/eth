# Ethereum Mempool Processor: Event Sequence

This document explains the event sequence and data flow in the Ethereum Mempool Processor, from transaction discovery through REVM simulation, state diff tracking, and scam detection.

## System Architecture Overview

The complete system consists of two main components:

1. **Python Pool Tracking Service**: Monitors liquidity pool states and publishes updates via ZeroMQ
2. **Rust Mempool Processor**: Monitors mempool transactions, simulates their effects with REVM, and detects potential scams

These components work together to provide comprehensive monitoring and scam detection:

```
┌───────────────────────┐                      ┌────────────────────────┐
│                       │                      │                        │
│  Python Pool Tracker  │  ZeroMQ (tcp:5557)   │  Rust Mempool Monitor  │
│  (Live pool levels)   ├─────────────────────►│  (REVM simulation &    │
│                       │                      │   scam detection)      │
└───────────────────────┘                      └────────────────────────┘
          │                                              │
          │                                              │
          ▼                                              ▼
┌───────────────────────┐                      ┌────────────────────────┐
│                       │                      │                        │
│  Pool reserve data    │                      │  PostgreSQL Database   │
│  collection           │                      │  (Alert logging)       │
│                       │                      │                        │
└───────────────────────┘                      └────────────────────────┘
```

## Transaction Discovery & Processing

The system uses HTTP RPC polling for transaction discovery, optimized for real-time scam detection.

### Current Method: HTTP RPC Polling with REVM Simulation

```
┌─────────────────────────┐      1. Poll        ┌─────────────────────┐
│                         │      mempool        │                     │
│  MempoolFetcher         ├────────────────────►│  Ethereum Node      │
│  (Main Loop)            │     (every 50ms)    │  (HTTP RPC)         │
│                         │                     │                     │
└─────────────┬───────────┘                     └─────────┬───────────┘
              │                                           │
              │                                           │ 2. Pending txs
              │                                           │ from mempool
              │                                           ▼
              │                               ┌─────────────────────┐
              │                               │                     │
              │                               │  Transaction        │
              │                               │  Filtering &        │
              │                               │  Caching            │
              │                               └─────────┬───────────┘
              │                                         │
              │                                         │ 3. New txs only
              │                                         │
              ▼                                         ▼
┌─────────────────────────┐     4. Process     ┌─────────────────────┐
│                         │     each tx        │                     │
│  process_transaction_   │◄────────────────── │  Vec<TransactionView>│
│  with_revm()            │                    │                     │
│                         │                    │                     │
└─────────────┬───────────┘                    └─────────────────────┘
              │
              │ 5. REVM simulate & detect scams
              │
              ▼
┌─────────────────────────┐
│                         │
│  TransactionSimulator + │
│  ScamDetectionService   │
│                         │
└─────────────────────────┘
```

## REVM Transaction Simulation & State Diff Tracking

The core simulation pipeline uses REVM for accurate transaction execution:

```
┌─────────────────────────┐     1. Submit tx    ┌─────────────────────┐
│                         │     for simulation  │                     │
│  TransactionSimulator   ├────────────────────►│  REVM Engine        │
│  (revm_tx_simulator)    │                     │  (Local execution)  │
└─────────────────────────┘                     └─────────┬───────────┘
                                                          │
                                                          │ 2. Execute
                                                          │ transaction
                                                          ▼
┌─────────────────────────┐     3. Account      ┌─────────────────────┐
│                         │     changes         │                     │
│  CalculatedAccount      │◄────────────────────┤  SimulationOutput   │
│  Changes                │                     │  (gas, logs, state) │
└─────────────┬───────────┘                     └─────────────────────┘
              │
              │ 4. ETH balance changes
              │ (per account)
              ▼
┌─────────────────────────┐
│                         │
│  Pool Effect Analysis   │
│  (Cross-reference with  │
│   pool cache)           │
└─────────────┬───────────┘
              │
              │ 5. Prepare for
              │ scam detection
              ▼
┌─────────────────────────┐
│                         │
│  SimulationResult       │
│  (affected pools)       │
│                         │
└─────────────────────────┘
```

## Pool Data Flow

The Python component collects and publishes pool data as follows:

```
┌─────────────────────────┐      1. Query       ┌─────────────────────┐
│                         │      current        │                     │
│  Python Pool            ├────────────────────►│  Ethereum Node      │
│  Tracking Service       │      pool state     │  (HTTP RPC)         │
│                         │                     │                     │
└─────────────┬───────────┘                     └─────────────────────┘
              │
              │ 2. Process and format
              │ pool data
              ▼
┌─────────────────────────┐
│                         │
│  ZeroMQ Publisher       │
│  (tcp://localhost:5557) │
│                         │
└─────────────────────────┘
              │
              │ 3. Publish pool data
              │ as JSON messages
              ▼
┌─────────────────────────┐
│                         │
│  ZeroMQ Subscriber      │
│  (Rust PoolSubscriber)  │
│                         │
└─────────────┬───────────┘
              │
              │ 4. Update pool cache
              │
              ▼
┌─────────────────────────┐
│                         │
│  PoolStateCache         │
│  (Thread-safe in-memory)│
│                         │
└─────────────────────────┘
```

## Integrated Scam Detection Flow

The current implementation uses a direct, efficient approach:

```
┌─────────────────────┐                        ┌─────────────────────┐
│                     │                        │                     │
│  Python Pool        │     ZeroMQ (5557)      │  PoolSubscriber     │
│  Tracking Service   ├───────────────────────►│  (Rust)             │
│                     │                        │                     │
└─────────────────────┘                        └─────────┬───────────┘
                                                         │
                                                         │ Update (async task)
                                                         ▼
┌─────────────────────────┐                    ┌─────────────────────┐
│                         │                    │                     │
│  MempoolFetcher         │                    │  PoolStateCache     │
│  (HTTP RPC Polling)     │                    │  (Shared Arc)       │
│                         │                    │                     │
└─────────────┬───────────┘                    └─────────┬───────────┘
              │                                          │
              │ New transactions                         │ Pool state lookup
              ▼                                          │
┌─────────────────────────┐                             │
│                         │                             │
│  Main Processing Loop   │                             │
│  (process_transaction_  │                             │
│   with_revm)            │                             │
└─────────────┬───────────┘                             │
              │                                         │
              │ For each transaction:                   │
              ▼                                         │
┌─────────────────────────┐     1. Simulate    ┌───────▼─────────────┐
│                         │     transaction    │                     │
│  TransactionSimulator   ├───────────────────►│  REVM Simulation    │
│  (REVM-based)           │                    │  (Account Changes)  │
│                         │                    │                     │
└─────────────┬───────────┘                    └─────────────────────┘
              │                                          │
              │ 2. Account changes                       │
              ▼                                          │
┌─────────────────────────┐     3. Convert     ┌───────▼─────────────┐
│                         │     to pool        │                     │
│  prepare_simulation_    │     effects        │  CalculatedAccount  │
│  result_from_revm_      ├───────────────────►│  Changes            │
│  changes()              │                    │                     │
└─────────────────────────┘                    └─────────┬───────────┘
                                                         │
                                                         │ 4. Analyze
                                                         ▼
                                               ┌─────────────────────┐
                                               │                     │
                                               │  ScamDetection      │
                                               │  Service            │
                                               │                     │
                                               └─────────┬───────────┘
                                                         │
                                                         │ 5. Alerts
                                                         ▼
                                               ┌─────────────────────┐
                                               │                     │
                                               │  DbLogger           │
                                               │  (PostgreSQL)       │
                                               │                     │
                                               └─────────────────────┘
```

### Key Implementation Details

**Current Architecture Features:**

1. **REVM Integration**: Uses `TransactionSimulator` with `revm_tx_simulator_lib` for accurate transaction simulation
2. **Simplified Transaction Flow**: Single main loop that:
   - Fetches transactions via HTTP RPC polling
   - Simulates each transaction with REVM
   - Converts REVM account changes to pool effects
   - Runs scam detection analysis
   - Logs alerts to database

3. **Concurrent Pool Updates**: The `PoolSubscriber` runs in a separate async task, continuously updating the shared `PoolStateCache`

4. **Direct REVM State Simulation**: Uses `TransactionSimulator.process_transaction()` directly in the main loop

### Actual Processing Flow

```
Main Loop:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  1. fetcher.get_transactions() → Vec<TransactionView>           │
│                                                                 │
│  2. For each transaction:                                       │
│     ├─ simulator.process_transaction(tx, block_env)            │
│     ├─ prepare_simulation_result_from_revm_changes()           │
│     ├─ service.process_transaction(simulation_result)          │
│     └─ Log alerts if any detected                              │
│                                                                 │
│  3. Sleep 50ms and repeat                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Concurrent Pool Updates:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  pool_subscriber.start_listening() (async task)                 │
│  ├─ Receive ZeroMQ messages                                     │
│  ├─ Parse JSON pool updates                                     │
│  └─ Update shared PoolStateCache                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## REVM Simulation Components

### TransactionSimulator Integration
- **Library**: `revm_tx_simulator_lib` for core simulation logic
- **Inputs**: `TransactionView` + `BlockEnv` (current chain state)
- **Outputs**: `HashMap<Address, CalculatedAccountChanges>` with precise ETH balance changes
- **Performance**: <20ms average simulation time for typical transactions

### State Change Processing
- **Input Format**: REVM `CalculatedAccountChanges` with `eth_balance_change` field
- **Conversion**: REVM Address → checksummed string format for pool cache lookup
- **Pool Detection**: Cross-reference affected addresses with `PoolStateCache`
- **Output**: `SimulationResult` with affected pool analysis

### Scam Detection Logic
- **ETH Threshold**: Detect pools drained below 0.15 ETH (configurable)
- **Percentage Threshold**: Detect withdrawals >50% of pool reserves (configurable)
- **False Positive Prevention**: Ignore positive ETH deltas (deposits/swaps)
- **Real-time Processing**: Process transactions before they execute on-chain

## Current Implementation Status

### Fully Implemented & Tested Components

1. ✅ **REVM Transaction Simulator**
   - Accurate simulation using `revm_tx_simulator_lib`
   - Validated against Etherscan for correctness
   - Performance optimized (<20ms average simulation time)
   - Proper EIP-1559 gas handling and state changes

2. ✅ **Pool Subscriber (ZeroMQ Client)**
   - ZeroMQ subscription from Python service  
   - JSON message deserialization (`PoolUpdatesMessage`)
   - Thread-safe pool state cache (`PoolStateCache`)
   - Async task for continuous pool updates

3. ✅ **Mempool Transaction Fetcher**
   - HTTP RPC polling optimized for 50ms intervals
   - Transaction caching with duplicate prevention
   - Batch processing for efficiency
   - Adaptive timeout and batching

4. ✅ **Scam Detection Service**
   - Detection rules for ETH reserve depletion below threshold
   - Detection rules for large percentage withdrawals  
   - Configurable thresholds (ETH amount and percentage)
   - Integration with pool cache for real-time pool state

5. ✅ **Database Logger**
   - PostgreSQL integration via `tokio-postgres`
   - Alert persistence in `mempool_scam_predictions` table
   - Graceful handling of foreign key constraints
   - Proper error logging and service continuity

6. ✅ **Integrated Service Binary**
   - `scam_detection_service.rs` with full REVM integration
   - Command-line configuration via `clap`
   - Statistics reporting and monitoring
   - Concurrent processing (pool updates + transaction analysis)

### Performance Characteristics

**Validated Performance Metrics** (from recent testing):
- **Simulation Time**: 3.45ms average REVM simulation
- **Setup Time**: 7.28ms average environment preparation  
- **State Diff Time**: 8.84ms average account change calculation
- **Total Processing**: 19.57ms average end-to-end