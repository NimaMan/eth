# Alert Processor Module

## Overview
The Alert Processor module is responsible for receiving, parsing, and validating scam detection alerts from the mempool processor. It serves as the entry point for all trading protection actions.

## Components

### `receiver.rs`
- Implements ZeroMQ subscriber to receive alerts from mempool processor
- Manages connection to ZMQ publisher endpoint
- Handles reconnection logic and error recovery
- Forwards parsed alerts to internal message queue

### `parser.rs`
- Parses JSON alert messages into strongly-typed structures
- Validates all required fields are present
- Performs sanity checks on alert data
- Converts between mempool processor format and internal format

### `types.rs`
- Defines `ScamAlert` struct with all alert fields
- Defines `MarketEvent` enum for different alert types
- Contains severity levels and alert metadata structures

## Alert Flow
```
Mempool Processor → ZMQ Publisher → Alert Receiver → Parser → Strategy Engine
```

## Alert Data Structure
```rust
pub struct ScamAlert {
    // Metadata
    pub alert_id: String,
    pub timestamp: u64,
    pub severity: Severity,
    
    // Transaction info
    pub tx_hash: String,
    pub block_number: u64,
    
    // Pool data
    pub pool_address: String,
    pub token_address: String,
    pub eth_reserve: f64,
    pub eth_change: f64,
    pub percentage_change: f64,
}
```

## Configuration
- `zmq_endpoint`: ZeroMQ subscription endpoint (default: `tcp://localhost:5558`)
- `queue_size`: Internal message queue size (default: 10000)
- `reconnect_interval`: Seconds between reconnection attempts (default: 5)

## Usage Example
```rust
use eth_kartal::alert_processor::{AlertReceiver, ScamAlert};

let mut receiver = AlertReceiver::new("tcp://localhost:5558");
let (tx, rx) = mpsc::channel();

// Start receiver in background
tokio::spawn(async move {
    receiver.start_listening(tx).await;
});

// Process alerts
while let Ok(alert) = rx.recv() {
    match alert.severity {
        Severity::Critical => // Emergency action
        Severity::High => // Quick response
        Severity::Medium => // Normal processing
    }
}
```

## Integration Points
- **Input**: ZMQ messages from mempool processor
- **Output**: Parsed `ScamAlert` structs to strategy engine
- **Dependencies**: `zmq`, `serde_json`, `tokio`

## Error Handling
- Connection failures trigger automatic reconnection
- Malformed messages are logged and skipped
- Critical errors are propagated to main error handler

## Performance Considerations
- Designed for <10ms processing latency
- Non-blocking message reception
- Efficient JSON parsing with pre-allocated buffers
- Concurrent processing of multiple alerts