# Signal Subscriber

Simple Python subscriber for testing mempool processor signals via ZMQ.

## Usage

```bash
python signal_subscriber.py
```

## Environment Variables

- `SIGNAL_ENDPOINT`: ZMQ endpoint (default: `tcp://127.0.0.1:5556`)

## Multiple Consumers

This subscriber uses ZMQ PUB/SUB pattern which supports multiple concurrent consumers:
- Database writers can consume signals
- eth_kartal can consume signals  
- Multiple monitoring tools can consume simultaneously
- Each consumer receives all published signals independently

## Signal Types Supported

- Honeypot/sell-blocked signals
- Tax bucket risk signals
- Liquidity removal signals
- Trading enabled signals  
- LP approval signals
- JSON-formatted signal payloads

## Example Output

```
======================================================================
🚀 Mempool Processor - Signal Subscriber
======================================================================
📡 Connected to signal publisher: tcp://127.0.0.1:5556
🔄 Waiting for signals... (Press Ctrl+C to stop)

📨 JSON Signal #1 [14:10:46]
   Topic: tax_signal
   Type: tax_signal
   Data: {
     "type": "tax_signal",
     "token_address": "0xc334..."
   }
------------------------------------------------------------
📨 JSON Signal #2 [14:12:05]
   Topic: lp_approval
   Type: lp_approval
   Data: {
     "type": "lp_approval",
     "token_address": "0x1b36..."
   }
------------------------------------------------------------
```

## Integration Notes

- **Database Writers**: Can consume signals to store detection results
- **eth_kartal**: Can consume signals for automated trading decisions
- **Monitoring**: Multiple monitoring tools can run simultaneously
- **Real-time**: Signals are published as soon as detected (microsecond latency)
- **Wire Format**: Signal publisher sends multipart `{topic, json}` on port `5556`.
