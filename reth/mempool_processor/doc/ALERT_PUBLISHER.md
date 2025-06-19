# Alert Publisher Integration

## Overview

The mempool processor now includes a ZeroMQ (ZMQ) alert publisher that broadcasts significant market events to external systems like ETH Kartal. This enables real-time response to detected scams and market anomalies.

## Configuration

### Command Line Arguments

```bash
# Enable alert publishing
./mempool_signal_detection_full_tx_ipc --enable-publisher

# Custom ZMQ endpoint (default: tcp://*:5559)
./mempool_signal_detection_full_tx_ipc --enable-publisher --alert-zmq-address tcp://*:5559
```

### Environment Variables

```bash
export ENABLE_PUBLISHER=true
export ALERT_ZMQ_ADDRESS=tcp://*:5559
```

**Note**: The default `tcp://*:5559` binds to all network interfaces. For security, use `tcp://localhost:5559` to restrict to local connections only.

## Alert Message Format

Alerts are published as JSON messages with the following structure:

```json
{
  "alert_id": "0x1234567890_1234567890",
  "timestamp": 1234567890,
  "severity": "Critical",
  "event_type": "ScamAlert",
  
  "tx_hash": "0x...",
  "detected_latency_us": 761000,
  
  "pool_address": "0x...",
  "pool_version": "V2",
  "token_address": "0x...",
  "token_symbol": "TOKEN",
  "token_decimals": 18,
  
  "current_eth_reserve": 10.5,
  "simulated_eth_reserve": 0.5,
  "eth_change_amount": -10.0,
  "eth_change_percent": -95.24,
  
  "current_price": 1000.0,
  "simulated_price": 100.0,
  "price_impact_percent": -90.0,
  
  "confidence_score": 0.95,
  "gas_price_gwei": 30.0,
  
  "details": "Critical liquidity drain: 95.24% of pool ETH removed"
}
```

## Event Types

The publisher broadcasts the following event types:

- **ScamAlert**: Critical liquidity drains (>50% ETH removed)
- **LiquidityWarning**: Significant liquidity changes (20-50%)
- **LargeTrade**: Unusually large transactions

Other event types (TokenSupplyAlert, VolumeSpike, PriceImpact) are detected but not published by default.

## Testing

### 1. Start the Alert Listener

```bash
cd /home/nima/code/crypto/rust/mempool_processor
python tests/test_zmq_publisher.py
```

### 2. Start Mempool Processor with Publishing

```bash
./target/debug/mempool_signal_detection_full_tx_ipc --enable-publisher
```

### 3. Monitor Logs

The processor logs alert publishing activity:

```
[INFO] 📢 Initializing ZMQ alert publisher on tcp://localhost:5558
[INFO] ✅ Alert publisher initialized successfully
[INFO] Published 100 alerts via ZMQ
```

## Performance

- Alert publishing is non-blocking (uses DONTWAIT flag)
- Minimal overhead: <0.1ms per alert
- High water mark: 10,000 messages
- No persistence: drops messages if buffer full

## Integration with ETH Kartal

ETH Kartal can receive these alerts by:

1. Creating a ZMQ SUB socket
2. Connecting to tcp://localhost:5559
3. Parsing JSON messages
4. Taking action based on event_type and severity

Example ETH Kartal receiver:

```rust
let alert_receiver = AlertReceiver::new("tcp://localhost:5558")?;
while let Ok(alert) = alert_receiver.receive().await {
    match alert.event_type.as_str() {
        "ScamAlert" if alert.severity == "Critical" => {
            // Emergency sell
        }
        "LiquidityWarning" => {
            // Partial exit
        }
        _ => {
            // Monitor only
        }
    }
}
```

## Troubleshooting

### No Alerts Received

1. Check publisher is enabled: `--enable-publisher` flag
2. Verify ZMQ endpoint matches between publisher and subscriber
3. Check firewall rules for port 5558
4. Monitor processor logs for "Failed to publish alert" errors

### High Message Drop Rate

1. Increase ZMQ high water mark
2. Add more subscribers to distribute load
3. Filter events at source (only publish critical events)

### Connection Refused

1. Ensure mempool processor starts before subscribers
2. Check endpoint address (localhost vs 127.0.0.1)
3. Verify no other process is using port 5559