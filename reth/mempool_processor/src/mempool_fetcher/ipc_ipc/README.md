# IPC-IPC Mempool Transaction Measurement

## Overview

This module implements high-performance transaction measurement using **IPC-to-IPC** communication pattern:
- **Subscription**: `eth_subscribe("newPendingTransactions")` via Unix IPC socket
- **Fetch**: `eth_getTransactionByHash` via same Unix IPC socket  
- **Performance**: 1.201ms average latency baseline

## Method: IPC-IPC

```
┌─────────────────┐    Unix Socket     ┌─────────────────┐
│   Our Client    │ ←─────────────────→ │   Reth Node     │
│                 │   /tmp/reth.ipc     │                 │
│ 1. Subscribe    │ ───────────────────→ │ newPendingTxs   │
│ 2. Get Tx Hash  │ ←─────────────────── │ tx hash notify  │
│ 3. Fetch Tx     │ ───────────────────→ │ getTransactionBy│
│ 4. Measure Time │ ←─────────────────── │ full tx data    │
└─────────────────┘                     └─────────────────┘
```

## Performance Characteristics

**Benchmark Results (1,000 transactions)**:
- **Average latency**: 1200.8μs (1.201ms)
- **Median latency**: 1101μs (1.101ms)  
- **Sub-1ms performance**: 43.3% (433/1000 transactions)
- **Success rate**: 100.0%
- **Transactions/second**: 11.30

**Why IPC-IPC is Fast**:
- No TCP/WebSocket overhead
- No HTTP protocol overhead  
- Single persistent Unix socket connection
- Direct kernel-level communication
- Immediate fetch on notification

## Comparison to Other Methods

| Method | Subscription | Fetch | Expected Latency |
|--------|-------------|-------|------------------|
| **IPC-IPC** | IPC socket | IPC socket | **~1.2ms** ✅ |
| WebSocket-HTTP | WebSocket | HTTP RPC | ~5-10ms |
| DevP2P | P2P protocol | P2P protocol | <1ms (theoretical) |

## Usage

```bash
# Run measurement tool
cargo run --example measure_1k_transactions

# Expected output location
/home/nima/code/crypto/logs/mempool_fetch/
├── tx_measurements_YYYYMMDD_HHMMSS.csv
└── detailed_log_YYYYMMDD_HHMMSS.txt
```

## Integration

This module provides reusable components for IPC-based transaction timing:
- `IpcIpcMeasurementClient` - Core measurement logic
- `TransactionLatencyMeasurement` - Individual transaction timing
- Configurable measurement parameters
- CSV and detailed logging output

## Files

- `examples/measure_1k_transactions.rs` - 1K transaction benchmark tool
- `measurement_client.rs` - Reusable measurement client
- `mod.rs` - Module exports and configuration