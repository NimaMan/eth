# Signal Subscriber Examples

This directory contains examples of how to subscribe to real-time signals from the mempool signal detector.

## Overview

The mempool signal detector publishes two types of signals via ZMQ:
- **Liquidity Removal** signals when liquidity is being removed from DEX pools
- **Trading Enabled** signals when new tokens enable trading

Signals are published to `tcp://127.0.0.1:5556` as JSON messages.

## Signal Format

Each signal contains the following fields:
```json
{
    "alert_type": "liquidity_removal",  // or "trading_enabled"
    "function_name": "removeLiquidityETH",
    "tx_hash": "0x...",
    "from_address": "0x...",
    "to_address": "0x...",
    "value": "0x0",
    "gas_price": "0x...",
    "selector": "02751cec",
    "timestamp": "2025-07-09 07:29:48.032",
    "detection_latency_us": 0
}
```

## Python Example

### Requirements
```bash
pip install pyzmq
```

### Running
```bash
python3 zmq_subscriber.py
```

## Rust Example

### Building
```bash
cd /home/nima/code/crypto/rust/mempool_processor/examples/signal_subscriber
cargo build --release
```

### Running
```bash
cargo run --release
```

## Example Output

```
================================================================================
Mempool Signal Detector - ZMQ Subscriber Example
================================================================================
[2025-07-09 09:30:00] Connected to ZMQ publisher at tcp://127.0.0.1:5556
Waiting for signals...

[2025-07-09 09:31:15] SIGNAL RECEIVED! #1
  Type: LIQUIDITY_REMOVAL
  Function: removeLiquidityETH
  TX Hash: 0xabc123...
  From: 0x123...
  To: 0x7a250d5630b4cf539739df2c5dacb4c659f2488d
  Value: 0x0
  Gas Price: 0x5f5e100
  Selector: 02751cec
  Timestamp: 2025-07-09 09:31:15.123
--------------------------------------------------------------------------------
```

## Integration with eth_kartal

The `eth_kartal` module can subscribe to these signals to:
1. React to liquidity removal events in real-time
2. Monitor new token launches when trading is enabled
3. Trigger automated trading strategies based on these signals

## Performance

The signal detector operates with:
- Average IPC detection latency: ~0.005ms
- Average function detection time: ~0.050ms
- Maximum latencies typically under 1ms