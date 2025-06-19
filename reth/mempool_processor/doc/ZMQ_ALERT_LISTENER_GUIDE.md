# ZMQ Alert Listener Development Guide

## Overview

The mempool processor publishes real-time scam alerts and market events via ZeroMQ (ZMQ) on port 5559. Your listener needs to connect to this port and process incoming JSON messages to protect against scams or capture arbitrage opportunities.

## Connection Details

- **Protocol**: ZMQ PUB/SUB pattern
- **Address**: `tcp://localhost:5559` (or `tcp://*:5559` if binding)
- **Message Format**: JSON strings
- **Latency**: Alerts published within 1-5ms of detection
- **Throughput**: Capable of 1000+ alerts/second

## Message Structure

Each alert is a JSON object with the following fields:

```json
{
  // Alert metadata
  "alert_id": "0x1234567890_1234567890",      // Unique ID (tx_hash prefix + timestamp)
  "timestamp": 1234567890,                     // Unix timestamp in seconds
  "severity": "Critical",                      // Critical/High/Medium/Low
  "event_type": "ScamAlert",                   // ScamAlert/LiquidityWarning/LargeTrade
  
  // Transaction context
  "tx_hash": "0x123...def",                   // Full transaction hash
  "detected_latency_us": 761000,              // Detection latency in microseconds
  
  // Pool information
  "pool_address": "0xabc...123",              // Liquidity pool address
  "pool_version": "V2",                       // Pool type (V2/V3/V4)
  "token_address": "0xdef...456",             // Token contract address
  "token_symbol": "SCAM",                     // Token symbol
  "token_decimals": 18,                       // Token decimal places
  
  // State changes
  "current_eth_reserve": 10.5,                // ETH in pool before tx
  "simulated_eth_reserve": 0.5,               // ETH in pool after tx
  "eth_change_amount": -10.0,                 // ETH removed (negative = drain)
  "eth_change_percent": -95.24,               // Percentage change
  
  // Price impact
  "current_price": 1000.0,                    // Price before tx (token/ETH)
  "simulated_price": 100.0,                   // Price after tx
  "price_impact_percent": -90.0,              // Price impact percentage
  
  // Risk metrics
  "confidence_score": 0.95,                   // Detection confidence (0-1)
  "gas_price_gwei": 30.0,                     // Current gas price
  
  // Details
  "details": "Critical liquidity drain: 95.24% of pool ETH removed"
}
```

## Event Types to Handle

### ScamAlert (Critical)
- **Trigger**: Pool drained >50% ETH
- **Action**: Immediate protective trading required
- **Strategy**: Execute emergency exits via different pools
- **Example**:
  ```python
  if alert["event_type"] == "ScamAlert" and alert["severity"] == "Critical":
      # Emergency sell if you hold this token
      emergency_exit(alert["token_address"], slippage=0.5)
  ```

### LiquidityWarning (High)
- **Trigger**: 20-50% liquidity change
- **Action**: Monitor closely, prepare for action
- **Strategy**: May indicate manipulation or large trades
- **Example**:
  ```python
  if alert["event_type"] == "LiquidityWarning":
      # Add to watch list, tighten stop losses
      add_to_monitoring(alert["token_address"])
  ```

### LargeTrade (Medium)
- **Trigger**: Significant price impact >15%
- **Action**: Check for arbitrage opportunities
- **Strategy**: Calculate profitability after gas
- **Example**:
  ```python
  if alert["event_type"] == "LargeTrade":
      # Check arbitrage opportunity
      profit = calculate_arbitrage(alert["pool_address"])
  ```

## Implementation Steps

### 1. Connect to ZMQ

```python
import zmq
import json

# Create context and socket
context = zmq.Context()
subscriber = context.socket(zmq.SUB)

# Connect to publisher
subscriber.connect("tcp://localhost:5559")

# Subscribe to all messages (empty filter)
subscriber.setsockopt_string(zmq.SUBSCRIBE, "")

print("Connected to alert publisher")
```

### 2. Message Processing Loop

```python
while True:
    try:
        # Receive message (blocking)
        message = subscriber.recv_string()
        
        # Parse JSON
        alert = json.loads(message)
        
        # Route by event type
        if alert["event_type"] == "ScamAlert":
            handle_scam_alert(alert)
        elif alert["event_type"] == "LiquidityWarning":
            handle_liquidity_warning(alert)
        elif alert["event_type"] == "LargeTrade":
            handle_large_trade(alert)
            
    except json.JSONDecodeError as e:
        print(f"Invalid JSON: {e}")
    except Exception as e:
        print(f"Error processing alert: {e}")
```

### 3. Decision Logic

```python
def handle_scam_alert(alert):
    """Handle critical scam alerts"""
    # Check if we hold this token
    balance = get_token_balance(alert["token_address"])
    if balance == 0:
        return
    
    # Check confidence threshold
    if alert["confidence_score"] < 0.8:
        print(f"Low confidence ({alert['confidence_score']}), monitoring only")
        return
    
    # Calculate potential loss
    eth_value = balance * alert["current_price"]
    potential_loss = eth_value * (alert["eth_change_percent"] / 100)
    
    # Execute protective trade if loss exceeds threshold
    if potential_loss > MIN_LOSS_THRESHOLD:
        execute_emergency_exit(
            token=alert["token_address"],
            amount=balance,
            max_slippage=0.5  # Accept 50% slippage for emergency
        )
```

### 4. Error Handling

```python
import time

def resilient_listener():
    """Listener with automatic reconnection"""
    while True:
        try:
            context = zmq.Context()
            subscriber = context.socket(zmq.SUB)
            subscriber.connect("tcp://localhost:5559")
            subscriber.setsockopt_string(zmq.SUBSCRIBE, "")
            
            # Set timeout to detect connection issues
            subscriber.setsockopt(zmq.RCVTIMEO, 5000)  # 5 second timeout
            
            while True:
                try:
                    message = subscriber.recv_string()
                    process_alert(json.loads(message))
                except zmq.Again:
                    # Timeout - check if publisher still alive
                    print("No alerts for 5 seconds, checking connection...")
                except json.JSONDecodeError:
                    print("Invalid JSON received")
                    
        except Exception as e:
            print(f"Connection error: {e}, reconnecting in 5s...")
            time.sleep(5)
        finally:
            subscriber.close()
            context.term()
```

## Testing Your Listener

### 1. Check Publisher Running
```bash
ps aux | grep mempool_signal.*enable-publisher
```

### 2. Test Connection
```bash
python3 /home/nima/code/crypto/rust/mempool_processor/tools/test_signal_integration.py
```

### 3. Monitor Real Alerts
```python
# Use the example listener
python3 /home/nima/code/crypto/rust/mempool_processor/tests/examples/test_zmq_publisher.py
```

### 4. Simulate Test Alerts
```python
# Create test publisher for development
import zmq
import json
import time

context = zmq.Context()
publisher = context.socket(zmq.PUB)
publisher.bind("tcp://*:5560")  # Different port for testing

# Send test alert
test_alert = {
    "alert_id": f"test_{int(time.time())}",
    "timestamp": int(time.time()),
    "severity": "Critical",
    "event_type": "ScamAlert",
    "tx_hash": "0x" + "a" * 64,
    "pool_address": "0x" + "b" * 40,
    "token_address": "0x" + "c" * 40,
    "token_symbol": "TEST",
    "eth_change_percent": -95.0,
    "confidence_score": 0.95,
    # ... other fields
}

publisher.send_string(json.dumps(test_alert))
```

## Complete Python Example

```python
#!/usr/bin/env python3
"""Production-ready alert listener with trading logic"""

import zmq
import json
import time
import logging
from datetime import datetime
from threading import Thread
from queue import Queue

class AlertListener:
    def __init__(self, endpoint="tcp://localhost:5559"):
        self.endpoint = endpoint
        self.alert_queue = Queue()
        self.running = False
        
    def start(self):
        """Start listener in background thread"""
        self.running = True
        Thread(target=self._listen, daemon=True).start()
        Thread(target=self._process_alerts, daemon=True).start()
        
    def _listen(self):
        """Listen for alerts and queue them"""
        while self.running:
            try:
                context = zmq.Context()
                socket = context.socket(zmq.SUB)
                socket.connect(self.endpoint)
                socket.setsockopt_string(zmq.SUBSCRIBE, "")
                socket.setsockopt(zmq.RCVTIMEO, 5000)
                
                logging.info(f"Connected to {self.endpoint}")
                
                while self.running:
                    try:
                        message = socket.recv_string()
                        alert = json.loads(message)
                        self.alert_queue.put(alert)
                    except zmq.Again:
                        # Timeout - normal, continue
                        pass
                    except Exception as e:
                        logging.error(f"Error receiving alert: {e}")
                        
            except Exception as e:
                logging.error(f"Connection error: {e}, retrying...")
                time.sleep(5)
            finally:
                socket.close()
                context.term()
    
    def _process_alerts(self):
        """Process queued alerts"""
        while self.running:
            try:
                alert = self.alert_queue.get(timeout=1)
                self._handle_alert(alert)
            except:
                continue
                
    def _handle_alert(self, alert):
        """Route alert to appropriate handler"""
        handlers = {
            "ScamAlert": self._handle_scam,
            "LiquidityWarning": self._handle_warning,
            "LargeTrade": self._handle_trade
        }
        
        handler = handlers.get(alert.get("event_type"))
        if handler:
            handler(alert)
            
    def _handle_scam(self, alert):
        """Handle scam alerts - execute protective trades"""
        if alert["confidence_score"] < 0.8:
            return
            
        if alert["eth_change_percent"] < -80:
            logging.critical(f"EMERGENCY: {alert['token_symbol']} drained {alert['eth_change_percent']:.1f}%")
            # Execute emergency exit logic here
            
    def _handle_warning(self, alert):
        """Handle liquidity warnings"""
        logging.warning(f"Liquidity change: {alert['token_symbol']} {alert['eth_change_percent']:.1f}%")
        # Add monitoring logic here
        
    def _handle_trade(self, alert):
        """Handle large trades - check arbitrage"""
        if alert["price_impact_percent"] < -20:
            logging.info(f"Large trade opportunity: {alert['token_symbol']}")
            # Add arbitrage logic here

# Usage
if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)
    
    listener = AlertListener()
    listener.start()
    
    print("Alert listener running. Press Ctrl+C to stop.")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        listener.running = False
        print("Shutting down...")
```

## Key Considerations

### Performance
- **Persistent Connection**: Keep connection alive, don't reconnect per message
- **Dedicated Thread**: Process alerts in separate thread for responsiveness
- **Queue Buffering**: Use queue to handle burst traffic
- **Non-blocking**: Use timeouts to avoid hanging

### Trading Logic
- **Position Tracking**: Know your holdings before acting
- **Slippage Tolerance**: Accept high slippage for emergency exits
- **Gas Costs**: Factor in transaction costs vs potential losses
- **Pool Selection**: Never trade into the attacked pool
- **Cooldowns**: Implement per-token cooldowns to avoid repeated actions

### Risk Management
- **Confidence Threshold**: Only act on high confidence alerts (>0.8)
- **Loss Limits**: Set maximum acceptable loss per token
- **Daily Limits**: Track cumulative losses/actions
- **Simulation**: Test trades before execution when possible

### Monitoring
- **Log All Alerts**: Store for analysis and backtesting
- **Track Performance**: Measure alert-to-action latency
- **Success Metrics**: Monitor profitable vs unprofitable actions
- **Alert Volume**: Track alerts per minute/hour

## Performance Requirements

Your listener should meet these targets:

| Metric | Target | Critical |
|--------|---------|----------|
| Connection Time | <100ms | <1s |
| Alert Processing | <5ms | <50ms |
| Decision Time | <50ms | <200ms |
| Total Latency | <100ms | <500ms |
| Throughput | 1000 alerts/s | 100 alerts/s |

## Security Notes

1. **Validate Addresses**: Check all addresses are valid Ethereum addresses
2. **Range Checks**: Verify percentages are -100 to +100
3. **Value Limits**: Set maximum trade sizes
4. **Simulation First**: Always simulate before executing
5. **Monitor Patterns**: Watch for unusual alert patterns

## Troubleshooting

### No Alerts Received
```bash
# Check publisher is running
ps aux | grep enable-publisher

# Test connection
nc -zv localhost 5559

# Run integration test
python3 /home/nima/code/crypto/rust/mempool_processor/tools/test_signal_integration.py
```

### High Latency
- Check network connection to localhost
- Verify no CPU throttling
- Monitor queue depth
- Profile processing function

### Missed Alerts
- Increase ZMQ high water mark
- Add persistent queue (Redis/RabbitMQ)
- Log dropped message count
- Scale horizontally if needed

## Next Steps

1. **Implement Core Logic**: Start with basic ScamAlert handling
2. **Add Position Tracking**: Integrate with your wallet/exchange
3. **Test Thoroughly**: Use historical data and simulated alerts
4. **Monitor Performance**: Track success rate and profitability
5. **Iterate and Improve**: Refine thresholds based on results

Remember: The faster you act on alerts, the better your protection against scams!