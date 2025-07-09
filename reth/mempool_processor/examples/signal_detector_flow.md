# Simplified Signal Detector Architecture

## Overview

The signal detector now follows a clean, simple flow:

```
IPC Fetch → Function Detection → (Future: Simulation → Pool Analysis)
```

## Components

### 1. **Mempool Fetcher (IPC Client)**
- Location: `src/mempool_fetcher/nonblocking_ipc_client.rs`
- Provides ultra-fast transaction detection via non-blocking IPC
- Returns `NonBlockingTransaction` with JSON data

### 2. **Conversion Logic**
- Location: `src/common/convert.rs`
- Converts IPC JSON data to ethers Transaction format
- Clean separation - conversion logic is in the common module, not in the binary

### 3. **Function Detector**
- Location: `src/signal_engine/function_detector.rs`
- Detects function signatures and categorizes them:
  - Liquidity removal functions
  - Trading enabled functions
  - Other functions
- Logs to timestamped directories with full transaction details

### 4. **Signal Detector Binary**
- Location: `src/bin/mempool_signal_detector.rs`
- Simple orchestration:
  1. Fetches transactions from IPC
  2. Converts to transaction view
  3. Passes to function detector
  4. Logs statistics every 60 seconds

## Key Improvements

1. **Removed complex conversion logic from binary** - now in `common/convert.rs`
2. **Simplified main loop** - just orchestrates components
3. **Clean separation of concerns** - each component has a single responsibility
4. **Built-in logging** - function detector handles its own logging

## Future Extensions

When ready, we can add:
- Transaction simulator (after function detection)
- Pool analyzer (analyzes state changes from simulator)
- Scam detection (based on pool analysis results)

The architecture is now ready for these additions without requiring major refactoring.