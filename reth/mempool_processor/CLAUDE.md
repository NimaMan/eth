# Mempool Processor - Implementation Details

## System Overview

The mempool processor is a high-performance system designed to:
1. **Receive transactions** from Ethereum mempool via IPC
2. **Detect function signatures** in transaction calldata
3. **Simulate transactions** to analyze state changes
4. **Identify signals** (liquidity removals, trading enabled, scams)
5. **Process as fast as possible** (currently 2-7μs detection latency)

## Critical Implementation Notes

### Performance Optimizations

#### String Operations Issue
The system currently has inefficient hex encoding/decoding:
```rust
// CURRENT (inefficient):
let selector = hex::encode(&input_data[0..4]); // Allocates new string
self.signatures.get(selector) // String comparison

// BETTER:
let selector_bytes = &input_data[0..4];
self.signatures_bytes.get(selector_bytes) // Direct byte comparison
```

This happens hundreds of times per second. Consider:
- Using byte arrays for signature storage
- Pre-computing hex strings if needed
- Caching frequently used conversions

#### IPC Performance
- Non-blocking I/O is properly implemented
- Buffer reuse prevents allocations
- Zero-copy parsing where possible

### Current Architecture Flow

```
1. IPC Client (NonBlockingIpcClient)
   ↓ Raw bytes
2. Transaction Parser
   ↓ Parsed transaction
3. Function Detector
   ↓ Transaction + detected functions
4. Simulator Processor
   ↓ Simulation results
5. Signal Detector
   ↓ Detected signals
6. Publishers (ZMQ/Logs)
```

### Key Components

#### 1. NonBlockingIpcClient (`mempool_fetcher/nonblocking_ipc_client.rs`)
- Connects to Reth IPC socket
- Uses `tokio::net::UnixStream` for async I/O
- Parses transactions using custom zero-copy parser
- Auto-reconnects on connection loss

#### 2. FunctionDetector (`signal_engine/function_detector.rs`)
- Checks 4-byte function selectors
- Currently detects:
  - Trading enabled: `0x8a8c523c` (enableTrading), `0xc9567bf9` (openTrading)
  - Liquidity removals: 10 different signatures
  - Swaps: 24 different signatures
- Uses HashMap lookup (O(1))

#### 3. SimulatorProcessor (`tx_simulator/simulator_processor.rs`)
- Batches transactions for simulation
- Filters out simple transfers
- Uses Reth's transaction simulator
- Handles nonce conflicts with retry logic

#### 4. SignalDetector (`tx_simulator/signal_detector.rs`)
- Analyzes simulation results
- Detects scams (>60% pool drain OR <0.3 ETH remaining)
- Tracks stablecoin burns/mints
- Publishes to ZMQ (tcp://127.0.0.1:5557)

### Critical Issues to Fix

#### 1. Error Handling
**Problem**: Extensive use of `unwrap()` causing panics
```rust
// CURRENT (bad):
let from = hex::decode(&tx.from).unwrap(); // PANIC!

// FIXED:
let from = hex::decode(&tx.from)
    .context("Invalid from address hex")?;
```

#### 2. Silent Failures
**Problem**: Errors logged but processing continues
```rust
// CURRENT (bad):
if let Err(e) = simulator.process(tx).await {
    warn!("Simulation failed: {}", e);
    // Continues to next transaction!
}

// FIXED:
simulator.process(tx).await
    .map_err(|e| {
        error!("Critical simulation failure: {}", e);
        ProcessingError::SimulationFailed(e)
    })?;
```

#### 3. Database Safety
**Problem**: SQL injection risk
```rust
// CURRENT (vulnerable):
let query = format!("INSERT INTO {} VALUES ({})", table, values);

// FIXED:
sqlx::query!("INSERT INTO transactions (hash, from_addr) VALUES ($1, $2)")
    .bind(&tx_hash)
    .bind(&from_address)
    .execute(&pool).await?;
```

### Configuration Requirements

#### Essential Environment Variables
```bash
# IPC connection (required)
RETH_IPC_PATH=/tmp/reth.ipc

# Simulation RPC (required)  
ETH_RPC_URL=http://localhost:8545

# Database (optional)
DATABASE_URL=postgresql://user:pass@localhost/db
```

#### Hard-coded Values to Extract
- Buffer sizes: `50000` (channel size)
- Timeouts: `Duration::from_secs(120)`
- Thresholds: `0.3` ETH minimum, `60%` drain threshold
- Batch sizes: `20` transactions per batch

### Performance Metrics

Current performance from production logs:
- **Detection Latency**: 2-7μs average, 40μs max
- **Transactions Processed**: 70,000+ per day
- **Simulation Success Rate**: ~85% (nonce conflicts cause failures)
- **Memory Usage**: ~500MB steady state

### Testing Strategy

#### Unit Tests Needed
1. Function detector accuracy
2. State change calculation
3. Scam detection logic
4. Error handling paths

#### Integration Tests Needed
1. Full pipeline test
2. Simulation failure handling
3. Network disconnection recovery
4. High load scenarios

### Monitoring Requirements

#### Key Metrics to Track
1. **Latency Percentiles**: p50, p95, p99
2. **Error Rates**: By component and error type
3. **Queue Depths**: Channel utilization
4. **Resource Usage**: CPU, memory, network

#### Alerting Thresholds
- Detection latency > 50μs (degraded)
- Error rate > 1% (warning)
- Queue depth > 40,000 (capacity risk)
- Any panic (critical)

### Security Considerations

1. **Input Validation**: All external data must be validated
2. **Resource Limits**: Prevent DoS through queue limits
3. **Access Control**: IPC socket permissions
4. **Credential Management**: Use secure storage for DB passwords

### Future Improvements

1. **Connection Pooling**: For database operations
2. **Caching Layer**: For frequently accessed data
3. **Horizontal Scaling**: Multiple instances with coordination
4. **Advanced Signals**: MEV detection, sandwich attacks

### Development Guidelines

1. **No `unwrap()`**: Use proper error handling
2. **Async All The Way**: Don't block the runtime
3. **Measure Everything**: Add metrics for new features
4. **Test Edge Cases**: Especially error paths
5. **Document Assumptions**: Make implicit knowledge explicit

### Common Pitfalls

1. **Hex String Allocations**: Use bytes where possible
2. **Blocking Operations**: Keep async runtime clear
3. **Unbounded Growth**: Set limits on all collections
4. **Silent Failures**: Always propagate critical errors
5. **Hard-coded Paths**: Use configuration for all paths

### Quick Debugging

```bash
# Check IPC connection
nc -U /tmp/reth.ipc

# Monitor performance
tail -f logs/mempool/signal_detector_*/signal_detector.log

# Check ZMQ messages
python examples/signal_subscriber/zmq_subscriber.py

# Database queries
psql $DATABASE_URL -c "SELECT COUNT(*) FROM transactions"
```

## Notes for Future Development

- The 50K channel size is not a concern - system handles it well
- Focus on reliability over micro-optimizations
- The simulation pipeline is the bottleneck, not detection
- Pool state must be kept synchronized with Python publisher
- Transaction ordering matters for nonce handling