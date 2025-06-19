# Signal Integration Guide - ZMQ Alert Subscriber

## Overview

The mempool processor publishes real-time alerts via ZMQ (ZeroMQ) PUB/SUB pattern on port 5559. This enables external systems like execution bots to receive scam alerts and market events with minimal latency.

## Signal Flow Architecture

```
Mempool Processor (Publisher)
    │
    ├── Detects scam/market event
    ├── Creates AlertMessage struct  
    ├── Serializes to JSON
    └── Publishes via ZMQ PUB socket → tcp://*:5559
                                            │
                                            ▼
                                    External Systems (Subscribers)
                                    ├── Trading Bots
                                    ├── Alert Systems
                                    ├── Analytics Engines
                                    └── ETH Kartal Executor
```

## Message Format

### AlertMessage Structure
```json
{
  "alert_id": "0x5aa1ca95_1718820590681",
  "timestamp": 1718820590681,
  "severity": "Critical",
  "event_type": "ScamAlert",
  
  "tx_hash": "0x5aa1ca955b80e5e275fae50591f46ebede9699afc01cc13bc8d8bc63ab06a3ea",
  "detected_latency_us": 384000,
  
  "pool_address": "0x7a4A6fe929362C8561DAC15162b9142A65A2E020",
  "pool_version": "V2",
  "token_address": "0x188b0b95AA5f3901ecaea2885b8fd724fac7E62D", 
  "token_symbol": "UNKNOWN",
  "token_decimals": 18,
  
  "current_eth_reserve": 2.151231,
  "simulated_eth_reserve": 0.0,
  "eth_change_amount": -2.151231,
  "eth_change_percent": -100.0,
  
  "current_price": 1000.0,
  "simulated_price": 0.0,
  "price_impact_percent": -100.0,
  
  "confidence_score": 0.98,
  "gas_price_gwei": 30.0,
  
  "details": "Critical liquidity drain: 100.00% of pool ETH removed"
}
```

## Event Types

### 1. ScamAlert (Critical Severity)
- Pool drained >50% of ETH reserves
- Triggers immediate protective action
- Highest priority for execution

### 2. LiquidityWarning (High Severity)  
- Pool liquidity change >20% but <50%
- May indicate manipulation or large trade
- Monitor for follow-up activity

### 3. LargeTrade (Medium Severity)
- Significant single transaction
- Price impact >15%
- May present arbitrage opportunity

### 4. VolumeSpike (Medium Severity)
- Trading volume >5x average
- Indicates high activity
- Check for wash trading

### 5. TokenSupplyAlert (High Severity)
- Token supply manipulation detected
- Mint/burn affecting price
- Check for infinite mint exploits

## Integration Examples

### Python Subscriber
```python
import zmq
import json
from datetime import datetime

# Connect to publisher
context = zmq.Context()
subscriber = context.socket(zmq.SUB)
subscriber.connect("tcp://localhost:5559")
subscriber.setsockopt_string(zmq.SUBSCRIBE, "")  # Subscribe to all

while True:
    message = subscriber.recv_string()
    alert = json.loads(message)
    
    if alert["event_type"] == "ScamAlert":
        print(f"🚨 SCAM: {alert['pool_address']}")
        print(f"   ETH Lost: {-alert['eth_change_amount']:.4f}")
        print(f"   Confidence: {alert['confidence_score']}")
        # Trigger protection bot
```

### Rust Subscriber
```rust
use zmq;
use serde_json;

let context = zmq::Context::new();
let subscriber = context.socket(zmq::SUB)?;
subscriber.connect("tcp://localhost:5559")?;
subscriber.set_subscribe(b"")?;

loop {
    let msg = subscriber.recv_string(0)??;
    let alert: AlertMessage = serde_json::from_str(&msg)?;
    
    match alert.event_type.as_str() {
        "ScamAlert" => execute_protection(&alert),
        "LiquidityWarning" => monitor_pool(&alert),
        _ => log_event(&alert),
    }
}
```

### Node.js Subscriber
```javascript
const zmq = require('zeromq');

async function subscribe() {
    const sock = new zmq.Subscriber();
    sock.connect("tcp://localhost:5559");
    sock.subscribe("");
    
    for await (const [msg] of sock) {
        const alert = JSON.parse(msg.toString());
        
        if (alert.event_type === 'ScamAlert') {
            console.log(`🚨 Scam detected: ${alert.pool_address}`);
            await executeProtection(alert);
        }
    }
}
```

## Decision Logic for Subscribers

### For ScamAlert Events
```python
def handle_scam_alert(alert):
    # 1. Check if we hold the token
    if not holding_token(alert['token_address']):
        return
    
    # 2. Check confidence and severity
    if alert['confidence_score'] < 0.8:
        return  # Too uncertain
    
    # 3. Calculate potential loss
    eth_at_risk = calculate_exposure(alert['token_address'])
    
    # 4. Execute protection if worthwhile
    if eth_at_risk > MIN_PROTECTION_THRESHOLD:
        # Sell immediately via DEX aggregator
        execute_emergency_sell(
            token=alert['token_address'],
            pool_to_avoid=alert['pool_address'],  # Don't sell into attacked pool
            slippage=0.5  # Accept high slippage to exit
        )
```

### For Arbitrage Opportunities
```python
def handle_large_trade(alert):
    # Check if price impact creates arbitrage
    if abs(alert['price_impact_percent']) > 5:
        # Calculate profitability
        arb_profit = calculate_arbitrage(
            pool=alert['pool_address'],
            impact=alert['price_impact_percent']
        )
        
        if arb_profit > gas_costs * 2:
            execute_arbitrage(alert)
```

## Performance Considerations

1. **Latency**: Alerts published within 1-5ms of detection
2. **Throughput**: Can handle 1000+ alerts/second
3. **Message Size**: ~500 bytes per alert
4. **Connection**: Keep persistent connection, don't reconnect per message
5. **Threading**: Run subscriber in dedicated thread/process

## Testing Your Integration

### 1. Basic Connectivity Test
```bash
# Install zmq tools
sudo apt-get install python3-zmq

# Run test subscriber
python3 /home/nima/code/crypto/rust/mempool_processor/tools/test_alert_subscriber.py
```

### 2. Simulate Test Alert
```python
# Publisher for testing
import zmq
import json
import time

context = zmq.Context()
publisher = context.socket(zmq.PUB)
publisher.bind("tcp://*:5560")  # Different port for testing

time.sleep(1)  # Let subscribers connect

test_alert = {
    "alert_id": "test_123",
    "timestamp": int(time.time() * 1000),
    "severity": "Critical",
    "event_type": "ScamAlert",
    "tx_hash": "0xtest...",
    "pool_address": "0xpool...",
    "eth_change_amount": -10.5,
    "confidence_score": 0.95,
    "details": "Test scam alert"
}

publisher.send_string(json.dumps(test_alert))
```

### 3. Load Testing
```python
# Measure your subscriber's performance
import time

message_count = 0
start_time = time.time()

while True:
    message = subscriber.recv_string()
    message_count += 1
    
    if message_count % 1000 == 0:
        elapsed = time.time() - start_time
        rate = message_count / elapsed
        print(f"Processing {rate:.0f} messages/second")
```

## Best Practices

1. **Error Handling**
   - Wrap JSON parsing in try/catch
   - Handle connection drops gracefully
   - Log failed messages for debugging

2. **Filtering**
   - Only process relevant event types
   - Check token/pool addresses against whitelist
   - Respect confidence thresholds

3. **Rate Limiting**
   - Implement cooldowns per pool
   - Avoid executing on same pool repeatedly
   - Track gas costs vs potential profit

4. **Monitoring**
   - Log all received alerts
   - Track execution success rate
   - Monitor missed opportunities

## Troubleshooting

### Not Receiving Messages
```bash
# Check if publisher is running
ps aux | grep mempool_signal

# Check if port is open
netstat -tlnp | grep 5559

# Test with netcat
nc -l 5559  # In one terminal
echo "test" | nc localhost 5559  # In another
```

### Message Parse Errors
- Ensure you're parsing complete messages
- Check for UTF-8 encoding issues
- Log raw messages when debugging

### Performance Issues
- Use ZMQ DONTWAIT flag for non-blocking receives
- Process messages in separate thread from execution
- Consider filtering at subscription level

## Security Considerations

1. **Validate Messages**
   - Check all addresses are valid Ethereum addresses
   - Verify reasonable value ranges
   - Don't blindly execute on any alert

2. **Network Security**
   - Run on localhost only unless secured
   - Consider ZMQ CURVE encryption for remote connections
   - Monitor for unusual message patterns

3. **Execution Safety**
   - Always simulate transactions before executing
   - Implement maximum loss limits
   - Use flashloan protection if available