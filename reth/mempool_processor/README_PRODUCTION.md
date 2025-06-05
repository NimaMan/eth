# Mempool Processor - Production Deployment

## Overview
High-performance Ethereum mempool processor for real-time scam detection using REVM simulation.

## Performance
- **137 TPS** sustained throughput
- **2.43ms** average processing time
- **96.6%** SLA compliance (<50ms)

## Prerequisites
- Reth node running with IPC enabled at `/tmp/reth.ipc`
- PostgreSQL database `eth_db`
- Python pool service running on ZMQ ports 5557/5558

## Build
```bash
cargo build --release --bin scam_detection_service
```

## Run Production Mode
```bash
# Default mode (DevP2P via IPC)
./target/release/scam_detection_service --verbose

# Process all transactions (not just pool-related)
./target/release/scam_detection_service --verbose --process-all-transactions
```

## Configuration
- **ETH Threshold**: 0.15 ETH (transactions below this are flagged)
- **Database**: `postgresql://postgres:postgres@localhost:5432/eth_db`
- **Pool Service**: `tcp://localhost:5557` (ZMQ)

## Monitoring
- Logs: `/home/nima/code/crypto/logs/mempool/scam_detection_service_*.log`
- Timing analysis: `/home/nima/code/crypto/logs/mempool/transaction_timing_analysis_*.csv`

## Integration
This service logs scam detections to the database. The Python portfolio manager can read these logs for trading decisions.