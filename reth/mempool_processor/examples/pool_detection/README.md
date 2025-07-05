# Pool Detection Examples

This directory contains examples for monitoring pool detection and updates from the Python token processing system.

## pool_detection_monitor.rs

Connects to the live Python publisher to monitor pool updates in real-time.

### Features
- Connects to live Python TokenInfoPublisher on ports 5557/5558
- Tracks all pool updates with full details
- Logs new pools and changes to existing pools
- Provides periodic summaries
- Writes detailed logs to `/home/nima/code/crypto/logs/mempool/pool_detection_*.log`

### Usage
```bash
cargo run --example pool_detection_monitor --release
```

### What It Monitors
- Pool address
- Token address
- ETH reserves
- Token reserves
- Last updated block
- Pool age

### Log Format
CSV format with columns:
```
timestamp,pool_address,token_address,eth_reserve,token_reserve,last_updated_block,age_seconds
```

### Requirements
The Python token processing system must be running with:
- TokenInfoPublisher active on ports 5557 (PUB) and 5558 (REP)
- Live block processing feeding pool updates