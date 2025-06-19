# Mempool Processor Structure Audit

## Current Module Structure

### ✅ Core Modules (Keep)
These are the essential modules that make the system work:

1. **signal_engine/** - Market event and scam detection logic
   - `engine.rs` - Core detection algorithms
   - `service.rs` - Service wrapper with database integration
   - `types.rs` - Event types and data structures
   - Status: **ACTIVE**, well-structured

2. **tx_simulator/** - Transaction simulation using debug_traceCall
   - `debug_tracecall_simulator.rs` - Fast production simulator
   - `debug_tracecall_state_diff_calculator.rs` - State change analysis
   - `state_diff_types.rs` - Core types (has commented LMDB code)
   - Status: **ACTIVE**, needs LMDB cleanup

3. **pool_subscriber/** - Real-time pool state updates via ZeroMQ
   - `cache.rs` - In-memory pool state cache
   - `types.rs` - Pool update structures
   - `tests/` - Unit tests
   - Status: **ACTIVE**, working well

4. **database/** - PostgreSQL integration
   - `scam_prediction_writer.rs` - Writes scam detections to DB
   - Status: **ACTIVE**, properly named

5. **common/** - Shared utilities and types
   - `address.rs` - Address utilities
   - `constants.rs` - System constants
   - `types.rs` - Common types
   - Status: **ACTIVE**, clean

### 🔧 Mempool Fetcher Modules (Partial Keep)
Transaction detection methods - only some are used:

1. **ipc_ipc/** - IPC subscription + fetch (**KEEP**)
   - `measurement_client.rs` - Performance measurement
   - Status: **UNUSED** but could be useful for benchmarking

2. **ipc_ipc_variants/** - Full TX IPC implementation (**KEEP**)
   - `full_tx_client.rs` - The main active implementation
   - Status: **ACTIVE** - This is what we use!

3. **websocket/** - WebSocket client (**QUESTIONABLE**)
   - `client.rs` - WebSocket implementation
   - Status: **UNUSED** - All WS binaries are broken

4. **processor/** - Transaction processing (**KEEP**)
   - `processor.rs` - Transaction processor
   - `pools.rs` - Pool tracking
   - `scam_prediction_writer.rs` - DB writer
   - Status: **ACTIVE**

### 🗑️ Broken/Unused Binaries (9 out of 12)
These binaries don't compile due to old interfaces:
- `measure_full_mempool.rs` - Uses old MempoolFetcher
- `measure_mempool_timing.rs` - Uses old interfaces
- `measure_optimized_mempool.rs` - Old measurement tool
- `measure_streaming_performance.rs` - Streaming test
- `mempool_latency_benchmark.rs` - Old benchmark
- `mempool_websocket_full.rs` - WebSocket monitoring
- `realtime_latency_validator.rs` - Validation tool
- `test_direct_integration.rs` - Direct integration test
- `websocket_latency_test.rs` - WebSocket test

### ✅ Working Binaries (3 only)
1. **mempool_signal_detection_full_tx_ipc.rs** - Main production binary
2. **mempool_tracker.rs** - Basic tracking tool
3. **metrics_api_server.rs** - HTTP metrics server

### 📚 Documentation Status
- Main `README.md` - Outdated, references old structure
- Module READMEs - Mix of missing and outdated
- `AUDIT_REPORT.md` - Current and accurate
- No architecture diagram

## Recommendations

### Immediate Actions
1. **Remove all broken binaries** (9 files)
2. **Remove websocket module** if not planning to fix
3. **Clean up LMDB comments** in state_diff_types.rs
4. **Fix unused imports** across the codebase

### Documentation Updates Needed
1. **Main README.md** - Complete rewrite needed
2. **Architecture diagram** - Create new one
3. **Module READMEs** - Add for each core module
4. **Usage guide** - For the 3 working binaries

### Code Organization
1. Consolidate measurement tools into one working binary
2. Remove examples that use old interfaces
3. Create proper integration tests

## Module Dependencies
```
mempool_signal_detection_full_tx_ipc
├── signal_engine (scam detection)
├── tx_simulator (debug_traceCall)
├── pool_subscriber (ZeroMQ updates)
├── database (PostgreSQL writer)
├── mempool_fetcher/ipc_ipc_variants (Full TX IPC)
└── common (utilities)
```

## Final Structure (After Cleanup)
```
src/
├── bin/
│   ├── mempool_signal_detection_full_tx_ipc.rs  # Main
│   ├── mempool_tracker.rs                       # Tool
│   └── metrics_api_server.rs                    # API
├── common/          # Shared utilities
├── database/        # DB integration
├── mempool_fetcher/ # TX detection (IPC only)
├── pool_subscriber/ # Pool state tracking
├── signal_engine/   # Scam detection
└── tx_simulator/    # Transaction simulation
```