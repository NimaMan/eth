# Ethereum Mempool Processor Development Plan

## Project Objective
Build a high-performance Rust mempool processor that:
1. **Monitors Ethereum mempool** for pending transactions in real-time
2. **Receives pool level updates** from Python via ZeroMQ
3. **Simulates transaction effects** on pool balances using REVM
4. **Detects scam transactions** that would drain pools below safety thresholds
5. **Publishes sell signals** immediately for other modules to execute trades

## Architecture Overview

### Core Components
1. **Mempool Fetcher** - Retrieves pending transactions from Ethereum node
2. **Transaction Simulator** - Simulates transaction effects using REVM
3. **Pool Subscriber** - Receives pool level updates from Python via ZeroMQ
4. **Scam Detection Engine** - Analyzes simulated effects against pool thresholds
5. **Signal Publisher** - Publishes sell signals for immediate execution
6. **Database Logger** - Records scam alerts and system metrics

### Data Flow
```
Ethereum Node → Mempool Fetcher → Transaction Simulator → Scam Detection Engine
                                                                    ↓
Python Pools → Pool Subscriber → Pool State Cache → Scam Detection Engine
                                                                    ↓
                                Signal Publisher → Sell Signal → Trading Module
                                                                    ↓
                                Database Logger → PostgreSQL
```

## Performance Requirements
- **Mempool Monitoring**: <100ms from transaction appearance to processing
- **Transaction Simulation**: <50ms per transaction using REVM
- **Scam Detection**: <10ms analysis time per transaction
- **Signal Publishing**: <5ms from detection to signal broadcast
- **Total Latency**: <200ms end-to-end for critical scam detection

## Current Implementation Status

### ✅ Completed Components

#### 1. Mempool Fetcher (`src/mempool_processor/fetcher.rs`)
- **Algorithm**: Polls `txpool_content` with optimized timeout handling and exponential backoff
- **Features**:
  - Circuit breaker pattern for RPC failures
  - Dynamic timeout adjustment (1.5s-30s)
  - Exponential backoff retry logic
  - Connection pooling and keepalive
  - Performance metrics tracking
- **Performance**: 100% success rate, <200ms average response time

#### 2. Pool Subscriber (`src/mempool_processor/pool_subscriber.rs`)
- **Algorithm**: ZeroMQ subscriber receiving JSON pool updates from Python
- **Features**:
  - Thread-safe pool state cache
  - Automatic reconnection handling
  - JSON deserialization of pool data
  - Real-time pool level tracking
- **Status**: Successfully receiving and processing pool updates

#### 3. Database Logger (`src/db_logger/mod.rs`)
- **Algorithm**: PostgreSQL connection with prepared statements for scam alerts
- **Features**:
  - Connection pooling and error handling
  - Structured scam alert logging
  - Performance metrics storage
  - Test utilities for verification
- **Status**: Fully functional with connection management

#### 4. Transaction Simulator (`src/tx_simulator/mod.rs`)
- **Algorithm**: REVM-based transaction simulation with state diff extraction
- **Features**:
  - EVM environment setup with proper gas and block parameters
  - ETH balance change tracking for all affected addresses
  - Success/revert handling
  - State diff aggregation
- **Status**: Basic simulation working, needs enhancement for scam detection

### 🔄 In Progress Components

#### 5. Scam Detection Engine (`src/scam_detection/engine.rs`)
- **Current**: Basic threshold comparison logic
- **Needed**: 
  - Pool sampling strategy (process subset of pools per cycle)
  - Aggregated state diff analysis
  - Configurable detection thresholds
  - Performance optimization for real-time processing

### ❌ Missing Components

#### 6. Signal Publisher
- **Purpose**: Broadcast sell signals immediately when scams detected
- **Requirements**:
  - ZeroMQ publisher for sell signals
  - Signal format: `{pool_address, severity, timestamp, recommended_action}`
  - <5ms publishing latency
  - Reliable delivery guarantees

#### 7. Mempool State Aggregation
- **Purpose**: Track cumulative effects of multiple pending transactions on same pools
- **Requirements**:
  - Address-based state diff aggregation
  - Time-based cleanup of old pending transactions
  - Memory-efficient storage for high transaction volumes

## Development Roadmap

### Phase 1: Enhanced Scam Detection (Current Priority)
**Goal**: Implement reliable scam detection with pool sampling

1. **Pool Sampling Strategy**
   - Sample 20-50 pools per detection cycle
   - Rotate through all pools to ensure coverage
   - Prioritize pools with recent activity

2. **State Diff Aggregation**
   - Aggregate multiple pending transactions affecting same pools
   - Track cumulative ETH balance changes
   - Implement cleanup for processed/expired transactions

3. **Detection Logic Enhancement**
   - Compare aggregated simulated levels vs current pool levels
   - Apply configurable safety thresholds
   - Generate severity scores for detected scams

### Phase 2: Signal Publishing System
**Goal**: Immediate sell signal broadcasting for detected scams

1. **ZeroMQ Publisher Setup**
   - Create publisher socket for sell signals
   - Define signal message format
   - Implement reliable delivery

2. **Signal Generation Logic**
   - Trigger signals when pools drop below thresholds
   - Include severity and recommended actions
   - Add rate limiting to prevent signal spam

3. **Integration Testing**
   - End-to-end testing with mock trading module
   - Latency measurement and optimization
   - Error handling and recovery

### Phase 3: Production Optimization
**Goal**: Meet performance requirements for live trading

1. **Performance Tuning**
   - Optimize REVM simulation parameters
   - Implement parallel transaction processing
   - Memory usage optimization

2. **Monitoring and Metrics**
   - Real-time performance dashboards
   - Alert system for component failures
   - Detailed logging for debugging

3. **Reliability Enhancements**
   - Graceful degradation during high load
   - Automatic recovery from failures
   - Data persistence for critical state

## Configuration System

### Environment Variables
```bash
# Ethereum Node
ETH_RPC_URL=http://localhost:8545
ETH_RPC_TIMEOUT_MS=8000

# ZeroMQ Communication
POOL_UPDATES_ENDPOINT=tcp://localhost:5555
SELL_SIGNALS_ENDPOINT=tcp://localhost:5556

# Database
DATABASE_URL=postgresql://user:pass@localhost/mempool_db

# Detection Parameters
SCAM_THRESHOLD_ETH=0.1
POOL_SAMPLE_SIZE=20
DETECTION_CYCLE_MS=100
```

### Performance Targets
- **Mempool Processing**: 1000+ transactions/second
- **Scam Detection**: <200ms end-to-end latency
- **Signal Publishing**: <5ms from detection to broadcast
- **System Uptime**: >99.9% availability
- **Memory Usage**: <1GB for 24/7 operation

## Testing Strategy

### Unit Tests
- Individual component functionality
- Error handling and edge cases
- Performance benchmarks

### Integration Tests
- End-to-end transaction flow
- ZeroMQ communication reliability
- Database operations under load

### Load Testing
- High transaction volume scenarios
- Network failure recovery
- Memory leak detection

## Success Metrics

### Functional Requirements
- ✅ Successfully process mempool transactions
- ✅ Receive pool updates from Python
- ✅ Log scam alerts to database
- 🔄 Detect scam transactions reliably
- ❌ Publish sell signals immediately

### Performance Requirements
- ✅ <200ms transaction processing latency
- ✅ 100% RPC success rate with retry logic
- ✅ Thread-safe concurrent operations
- 🔄 <10ms scam detection analysis
- ❌ <5ms signal publishing latency

### Reliability Requirements
- ✅ Graceful handling of RPC timeouts
- ✅ Automatic reconnection to data sources
- ✅ Clean error logging and recovery
- 🔄 Zero false negatives in scam detection
- ❌ Guaranteed signal delivery

## Current Development Focus

**Immediate Priority**: Complete scam detection engine with pool sampling and state aggregation to achieve reliable scam detection capability.

**Next Steps**:
1. Implement pool sampling strategy in scam detection engine
2. Add state diff aggregation for multiple pending transactions
3. Create signal publisher for immediate sell signal broadcasting
4. Conduct end-to-end testing with live mempool data

The system is designed for high-frequency trading scenarios where milliseconds matter for profitable scam detection and response.
