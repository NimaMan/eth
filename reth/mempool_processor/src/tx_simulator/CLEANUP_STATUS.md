# TX Simulator Cleanup Status

## Current State (2025-06-15)

### Active Components (In Use)
- `debug_tracecall_simulator.rs` - Primary simulator using debug_traceCall RPC (~5ms)
- `debug_tracecall_state_diff_calculator.rs` - Calculates state changes from trace data
- `state_diff.rs` - Core types for representing state changes

### Deprecated Components (To Be Removed)
- `simulator.rs` - REVM-based simulator (was hardcoded to block 18,000,000)
- `simulator_wrapper.rs` - No longer needed since we only use debug_traceCall
- `state_cache.rs` - Was for REVM caching, not needed with RPC approach
- `conversions.rs` - TransactionView to REVM conversions, not needed anymore

## Why These Changes?

1. **Performance**: debug_traceCall is 8x faster (~5ms vs ~40-50ms)
2. **Reliability**: No more "pruned state" errors from hardcoded blocks
3. **Simplicity**: Single simulation method instead of multiple options
4. **Maintenance**: Less code to maintain and debug

## Log File Location
Scam detections are logged to: `/home/nima/code/crypto/logs/mempool/scam_detections.log`

Note: The log file is only created when a scam is detected. If the file doesn't exist, it means no scams have been found yet.