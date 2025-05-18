# Ethereum Mempool Processor: Event Sequence

This document explains the event sequence and data flow in the Ethereum Mempool Processor, from transaction discovery through simulation, state diff tracking, and scam detection.

## System Architecture Overview

The complete system consists of two main components:

1. **Python Pool Tracking Service**: Monitors liquidity pool states and publishes updates via ZeroMQ
2. **Rust Mempool Processor**: Monitors mempool transactions, simulates their effects, and detects potential scams

These components work together to provide comprehensive monitoring and scam detection:

```
┌───────────────────────┐                      ┌────────────────────────┐
│                       │                      │                        │
│  Python Pool Tracker  │  ZeroMQ (tcp:5557)   │  Rust Mempool Monitor  │
│  (Live pool levels)   ├─────────────────────►│  (Transaction analysis │
│                       │                      │   & scam detection)    │
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

The system employs two methods for transaction discovery, with a fallback mechanism:

### Primary Method: WebSocket Subscription

```
                                             ┌─────────────────┐
                                             │                 │
                  1. New pending tx          │   Ethereum      │
                  notification               │   Node          │
                 ┌───────────────────────────┤   (WebSocket)   │
                 │                           │                 │
                 │                           └─────────────────┘
                 ▼
┌─────────────────────────┐     2. Tx hash      ┌─────────────────────┐
│                         │     via channel     │                     │
│  Transaction            ├────────────────────►│  Transaction        │
│  Subscription           │                     │  Fetcher            │
│  Handler                │                     │  Worker             │
│                         │                     │                     │
└─────────────────────────┘                     └─────────┬───────────┘
                                                          │
                                                          │ 3. Get full tx
                                                          │ details via HTTP
                                                          ▼
                                               ┌─────────────────────┐
                                               │                     │
                                               │  Ethereum Node      │
                                               │  (HTTP RPC)         │
                                               │                     │
                                               └─────────┬───────────┘
                                                         │
                                                         │ 4. Full tx data
                                                         │
                                                         ▼
┌─────────────────────────┐     5. Process     ┌─────────────────────┐
│                         │     transaction    │                     │
│  Transaction            │◄────────────────── │  Transaction        │
│  Processor              │                    │  Fetcher            │
│                         │                    │  Worker             │
└─────────────┬───────────┘                    └─────────────────────┘
              │
              │ 6. Send to scam detection
              │
              ▼
┌─────────────────────────┐
│                         │
│  Scam Detection         │
│  Engine                 │
│                         │
└─────────────────────────┘
```

### Fallback Method: Polling (if WebSocket fails)

```
┌─────────────────────────┐      1. Poll        ┌─────────────────────┐
│                         │      mempool        │                     │
│  Polling                ├────────────────────►│  Ethereum Node      │
│  Loop                   │     (periodically)  │  (HTTP RPC)         │
│                         │                     │                     │
└─────────────┬───────────┘                     └─────────┬───────────┘
              │                                           │
              │                                           │ 2. Full mempool
              │                                           │ content
              │                                           ▼
              │                               ┌─────────────────────┐
              │                               │                     │
              │                               │  Transaction        │
              │                               │  Filtering          │
              │                               │                     │
              │                               └─────────┬───────────┘
              │                                         │
              │                                         │ 3. New txs only
              │                                         │
              ▼                                         ▼
┌─────────────────────────┐     4. Process     ┌─────────────────────┐
│                         │     transaction    │                     │
│  Transaction            │◄────────────────── │  Transaction        │
│  Processor              │                    │  Processing         │
│                         │                    │                     │
└─────────────┬───────────┘                    └─────────────────────┘
              │
              │ 5. Send to scam detection
              │
              ▼
┌─────────────────────────┐
│                         │
│  Scam Detection         │
│  Engine                 │
│                         │
└─────────────────────────┘
```

## Transaction Simulation & State Diff Tracking

Once transactions are discovered through either method, state diff tracking works as follows:

```
┌─────────────────────────┐     1. Submit tx    ┌─────────────────────┐
│                         │     for simulation  │                     │
│  TransactionSimulator   ├────────────────────►│  REVM               │
│                         │                     │  EVM Implementation │
└─────────────────────────┘                     └─────────┬───────────┘
                                                          │
                                                          │ 2. Execute
                                                          │ transaction
                                                          ▼
┌─────────────────────────┐     3. Track        ┌─────────────────────┐
│                         │     state changes   │                     │
│  StateDiffTracker       │◄────────────────────┤  REVM               │
│                         │                     │  State Access       │
└─────────────┬───────────┘                     └─────────────────────┘
              │
              │ 4. State changes
              │ (balance + storage)
              ▼
┌─────────────────────────┐
│                         │
│  StateCache             │
│  (In-memory)            │
│                         │
└─────────────┬───────────┘
              │
              │ 5. Prepare for
              │ scam detection
              ▼
┌─────────────────────────┐
│                         │
│  prepare_simulation_    │
│  result                 │
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

The core of the system is the scam detection flow that integrates all components:

```
┌─────────────────────┐                        ┌─────────────────────┐
│                     │                        │                     │
│  Python Pool        │     ZeroMQ (5557)      │  PoolSubscriber     │
│  Tracking Service   ├───────────────────────►│  (Rust)             │
│                     │                        │                     │
└─────────────────────┘                        └─────────┬───────────┘
                                                         │
                                                         │ Update
                                                         ▼
┌─────────────────────────┐                    ┌─────────────────────┐
│                         │                    │                     │
│  MempoolFetcher         │                    │  PoolStateCache     │
│  (Transaction Source)   │                    │  (Current pool      │
│                         │                    │   reserves)         │
└─────────────┬───────────┘                    └─────────┬───────────┘
              │                                          │
              │ New transaction                          │ Current pool
              ▼                                          │ state
┌─────────────────────────┐                    ┌─────────▼───────────┐
│                         │  Transaction with  │                     │
│  StateDiffTracker       │  simulated changes │  ScamDetection      │
│  (State Simulation)     ├───────────────────►│  Service            │
│                         │                    │                     │
└─────────────────────────┘                    └─────────┬───────────┘
                                                         │
                                                         │ Alerts
                                                         ▼
                                               ┌─────────────────────┐
                                               │                     │
                                               │  DbLogger           │
                                               │  (PostgreSQL)       │
                                               │                     │
                                               └─────────────────────┘
```

## Current Implementation Status

### Fully Implemented Components

1. ✅ **Pool Subscriber (ZeroMQ Client)**
   - ZeroMQ subscription from Python
   - JSON message deserialization
   - Thread-safe pool state cache

2. ✅ **Mempool Transaction Fetcher**
   - WebSocket subscription (primary method)
   - HTTP polling fallback
   - Transaction caching with configurable size
   - Batch processing for efficiency

3. ✅ **Transaction Simulator**
   - REVM-based transaction execution
   - State diff tracking for ETH balances and storage
   - In-memory state cache

4. ✅ **Scam Detection Service**
   - Detection rules for ETH reserve depletion
   - Detection rules for large withdrawals
   - Configurable thresholds

5. ✅ **Database Logger**
   - PostgreSQL integration
   - Alert persistence

6. ✅ **Integrated Service Binary**
   - `scam_detection_service.rs` integrating all components
   - Command-line configuration options
   - Statistics reporting

7. ✅ **System Launch Script**
   - `run_mempool_monitor.sh` to launch both components
   - Environment configuration
   - Process management

### Execution Flow in Production

The integrated system execution flow is as follows:

1. **System Initialization**
   - The launch script starts the Python pool tracking service
   - The launch script then starts the Rust mempool monitor

2. **Concurrent Processing**
   - The Python service continuously monitors pool states and publishes updates
   - The Rust PoolSubscriber receives and processes pool updates
   - The Rust MempoolFetcher continuously fetches new transactions
   - The main processing loop simulates transactions and checks for potential scams

3. **Scam Detection**
   - For each new transaction, it's simulated to predict state changes
   - The system checks if the transaction would affect known pools
   - If a transaction would deplete a pool below threshold or withdraw a large percentage, it's flagged

4. **Alert Handling**
   - Detected scams are logged to the database
   - Statistics are reported periodically

## Performance Characteristics

- The system is designed for low-latency operation
- Batch processing and caching optimize transaction handling
- Multi-threaded architecture ensures responsive processing
- ZeroMQ provides efficient, non-blocking communication between components

This integrated architecture provides comprehensive monitoring of both liquidity pool states and mempool transactions, enabling proactive detection of potential scams before they can execute.
