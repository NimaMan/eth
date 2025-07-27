# Unified Publisher Examples

## Objective

The unified publisher system consolidates all signal publishing into a single, coherent architecture. The main objectives are:

1. **Single Publishing Endpoint**: All signals go through one ZMQ endpoint (port 5560) with topic-based routing
2. **Consistent Message Format**: All signals share a common base structure while allowing type-specific data
3. **Flexible Signal Detection**: Not all signals require simulation - some can be published immediately from function detection
4. **Multipart Messaging**: Use ZMQ multipart format `[topic, json_data]` for efficient filtering

## Examples in this Directory

### 1. `publisher_demo.rs`
Demonstrates publishing various signal types through the unified publisher.

### 2. `multipart_subscriber.rs`
Shows how to subscribe to signals using the multipart format and filter by topic.

### 3. `legacy_adapter.rs`
Adapts the new multipart format to the old single-message format for backward compatibility.

### 4. `signal_router_demo.rs`
Demonstrates the signal routing logic that determines which signals need simulation.

### 5. `python_subscriber.py`
Python example showing how to consume signals with topic filtering.

## Running the Examples

1. Start the publisher demo:
```bash
cargo run --example unified_publisher_demo
```

2. In another terminal, start a subscriber:
```bash
cargo run --example multipart_subscriber
```

3. For Python subscriber:
```bash
python examples/unified_publisher/python_subscriber.py
```

## Message Format

All messages are sent as ZMQ multipart with two parts:
- **Part 1**: Topic string (e.g., "tax_manipulation", "liquidity_removal")
- **Part 2**: JSON-encoded UnifiedSignal

Example signal structure:
```json
{
  "signal_id": "tax_manipulation_0x1234_1234567890",
  "signal_type": "TaxManipulation",
  "timestamp": 1234567890,
  "tx_hash": "0x123...",
  "from_address": "0xabc...",
  "token_address": "0xdef...",
  "severity": "Critical",
  "confidence": 0.95,
  "source": "TaxDecoder",
  "data": {
    "data_type": "TaxManipulation",
    "data": {
      "current_buy_tax": 5.0,
      "predicted_buy_tax": 99.0,
      ...
    }
  }
}
```