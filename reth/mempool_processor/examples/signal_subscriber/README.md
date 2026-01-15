# Signal Subscriber

Simple Python subscriber for testing mempool processor signals via ZMQ.

## Usage

```bash
python signal_subscriber.py
```

## Environment Variables

- `SIGNAL_ENDPOINT`: ZMQ endpoint (default: `tcp://127.0.0.1:5557`)

## Multiple Consumers

This subscriber uses ZMQ PUB/SUB pattern which supports multiple concurrent consumers:
- Database writers can consume signals
- eth_kartal can consume signals  
- Multiple monitoring tools can consume simultaneously
- Each consumer receives all published signals independently

## Signal Types Supported

- Tax detection signals (current format)
- Liquidity removal signals
- Trading enabled signals  
- LP approval signals
- JSON-formatted signals (future)

## Example Output

```
======================================================================
🚀 Mempool Processor - Signal Subscriber
======================================================================
📡 Connected to signal publisher: tcp://127.0.0.1:5557
🔄 Waiting for signals... (Press Ctrl+C to stop)

📨 TAX Signal #1 [14:10:46]
   [2025-08-12 14:10:46.355] TAX_SIGNAL | Token: 0xc334... | Pool: 0x67fa... | Type: HighTaxOrHoneypot | BuyTax: 0% | SellTax: 0%
------------------------------------------------------------
📨 TAX Signal #2 [14:12:05] 
   [2025-08-12 14:12:05.876] TAX_DETECTION | TX: 0xba4b... | Token: 0xc334... | can_buy: false | can_sell: false | buy_tax: -1.0%
------------------------------------------------------------
```

## Integration Notes

- **Database Writers**: Can consume signals to store detection results
- **eth_kartal**: Can consume signals for automated trading decisions
- **Monitoring**: Multiple monitoring tools can run simultaneously
- **Real-time**: Signals are published as soon as detected (microsecond latency)
