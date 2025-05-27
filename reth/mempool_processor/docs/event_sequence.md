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

The system currently uses HTTP RPC polling for transaction discovery. While the architecture supports WebSocket subscription as a primary method with HTTP polling as fallback, our current implementation focuses on the polling approach for simplicity and reliability.

### Current Method: HTTP RPC Polling

```
┌─────────────────────────┐      1. Poll        ┌─────────────────────┐
│                         │      mempool        │                     │
│  MempoolFetcher         ├────────────────────►│  Ethereum Node      │
│  (Main Loop)            │     (every 100ms)   │  (HTTP RPC)         │
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
│  process_transaction()  │◄────────────────── │  Vec<TransactionView>│
│  (Main Loop)            │                    │                     │
│                         │                    │                     │
└─────────────┬───────────┘                    └─────────────────────┘
              │
              │ 5. Simulate & detect scams
              │
              ▼
┌─────────────────────────┐
│                         │
│  StateDiffTracker +     │
│  ScamDetectionService   │
│                         │
└─────────────────────────┘
```

### Future Enhancement: WebSocket Subscription (Designed but not implemented)

The architecture was originally designed to support WebSocket subscription as the primary method:

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

The core of the system is the scam detection flow that integrates all components. **Note**: Our current implementation differs from the original design - we use a simpler, more direct approach:

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
│  (process_transaction)  │                             │
│                         │                             │
└─────────────┬───────────┘                             │
              │                                         │
              │ For each transaction:                   │
              ▼                                         │
┌─────────────────────────┐     1. Simulate    ┌───────▼─────────────┐
│                         │     transaction    │                     │
│  StateDiffTracker       ├───────────────────►│  REVM Simulation    │
│  (REVM-based)           │                    │  (State Changes)    │
│                         │                    │                     │
└─────────────┬───────────┘                    └─────────────────────┘
              │                                          │
              │ 2. State changes                         │
              ▼                                          │
┌─────────────────────────┐     3. Prepare     ┌───────▼─────────────┐
│                         │     simulation     │                     │
│  prepare_simulation_    │     result         │  Pool Effects       │
│  result()               ├───────────────────►│  (ETH deltas)       │
│                         │                    │                     │
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

**Current Architecture Differences:**

1. **No WebSocket Subscription**: Our current implementation uses HTTP RPC polling via `MempoolFetcher` rather than WebSocket subscription for mempool monitoring.

2. **Simplified Transaction Flow**: Instead of separate transaction fetcher workers, we have a single main loop that:
   - Fetches transactions via HTTP RPC
   - Simulates each transaction with REVM
   - Checks for pool effects
   - Runs scam detection
   - Logs alerts to database

3. **Concurrent Pool Updates**: The `PoolSubscriber` runs in a separate async task, continuously updating the shared `PoolStateCache` while the main loop processes transactions.

4. **Direct State Simulation**: We use `StateDiffTracker` with REVM directly in the main loop, not as a separate service.

### Actual Processing Flow

```
Main Loop:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  1. fetcher.get_transactions() → Vec<TransactionView>           │
│                                                                 │
│  2. For each transaction:                                       │
│     ├─ tracker.simulate_transaction(tx)                        │
│     ├─ state_cache.add_transaction(changes)                    │
│     ├─ prepare_simulation_result(tx, changes, pool_cache)      │
│     ├─ service.process_transaction(simulation_result)          │
│     └─ Log alerts if any detected                              │
│                                                                 │
│  3. Sleep 100ms and repeat                                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Concurrent Pool Updates:
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  pool_subscriber.start_listening() (async task)                │
│  ├─ Receive ZeroMQ messages                                     │
│  ├─ Parse JSON pool updates                                     │
│  └─ Update shared PoolStateCache                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Current Implementation Status

### Fully Implemented Components

1. ✅ **Pool Subscriber (ZeroMQ Client)**
   - ZeroMQ subscription from Python service
   - JSON message deserialization (`PoolUpdatesMessage`)
   - Thread-safe pool state cache (`PoolStateCache`)
   - Async task for continuous pool updates

2. ✅ **Mempool Transaction Fetcher**
   - HTTP RPC polling via `MempoolFetcher`
   - Transaction caching with configurable size
   - Batch processing for efficiency
   - *Note: WebSocket subscription designed but not implemented*

3. ✅ **Transaction Simulator**
   - REVM-based transaction execution via `StateDiffTracker`
   - State diff tracking for ETH balances and storage changes
   - In-memory state cache (`StateCache`)
   - Integration with main processing loop

4. ✅ **Scam Detection Service**
   - Detection rules for ETH reserve depletion below threshold
   - Detection rules for large percentage withdrawals
   - Configurable thresholds (ETH amount and percentage)
   - Integration with pool cache for real-time pool state

5. ✅ **Database Logger**
   - PostgreSQL integration via `tokio-postgres`
   - Alert persistence in `mempool_scam_predictions` table
   - Graceful handling of foreign key constraints (missing tokens)
   - Proper error logging and service continuity

6. ✅ **Integrated Service Binary**
   - `scam_detection_service.rs` integrating all components
   - Command-line configuration options via `clap`
   - Statistics reporting and monitoring
   - Concurrent processing (pool updates + transaction analysis)

7. ✅ **Comprehensive Testing Suite**
   - Unit tests for individual components
   - Integration tests for end-to-end flow
   - Live monitoring test (`test_live_pool_monitoring.rs`)
   - Database connectivity and scam detection verification

8. ⚠️ **System Launch Script** 
   - `run_mempool_monitor.sh` designed but may need updates
   - Environment configuration templates
   - Process management for both Python and Rust components

### Verified Functionality

**✅ Live System Integration:**
- Successfully connects to live Python pool tracking service via ZeroMQ
- Receives and processes real pool state updates
- Monitors actual Ethereum mempool transactions
- Detects pool state changes and significant ETH movements

**✅ Scam Detection Logic:**
- Correctly identifies pools depleted below ETH threshold
- Generates proper scam alerts with detailed information
- Handles edge cases (very low reserves, percentage-based detection)
- Logs alerts with transaction hash, pool address, and depletion amounts

**✅ Database Integration:**
- Connects to PostgreSQL database successfully
- Handles foreign key constraints gracefully (missing tokens)
- Continues service operation even when database writes fail
- Provides clear error messages for debugging

**✅ Performance & Reliability:**
- Processes live data efficiently (100ms polling interval)
- Handles ZeroMQ connection timeouts gracefully
- Maintains service stability during database errors
- Provides comprehensive logging and monitoring

### Recent Test Results

**Live Monitoring Test (Latest Run):**
- **Duration**: 2.1 minutes monitoring 10 blocks (22561096-22561106)
- **Pools Discovered**: 12 unique pools with reserves from 0.01 to 38+ ETH
- **Pool Updates**: 32 total updates received from live system
- **Significant Changes**: 10 pool changes detected (>10% or >0.1 ETH)
- **Scam Detection**: 1 successful detection on low-reserve pool

**Specific Scam Detection Example:**
- **Token Address**: `0x6Ae82F23C593b520f90822D6A0bA29ce5f7b06f8`
- **Pool Address**: `0xBCac3A7cA9385F141469f2dE2bfFb1d18C7A67d8`
- **Detection**: Pool depletion from 0.01 ETH → 0.001 ETH (90% depletion)
- **Result**: Scam alert generated correctly, database write failed due to missing token entry (expected behavior)

This test confirms that our scam detection system works correctly with live data and will log alerts to the database once tokens are properly populated by other system components.

## Performance Characteristics

- The system is designed for low-latency operation
- Batch processing and caching optimize transaction handling
- Multi-threaded architecture ensures responsive processing
- ZeroMQ provides efficient, non-blocking communication between components

This integrated architecture provides comprehensive monitoring of both liquidity pool states and mempool transactions, enabling proactive detection of potential scams before they can execute.
