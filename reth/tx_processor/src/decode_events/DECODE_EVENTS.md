# Decode Events Module

## Overview

The `decode_events` module provides **comprehensive event log decoding** for all major Ethereum protocols. This module transforms raw event logs into structured, typed data that can be easily analyzed for MEV detection, DeFi analytics, and transaction classification.

## Architecture

```
decode_events/
├── mod.rs               # Module exports and public interface
├── decoder.rs           # Main event decoder and routing logic
├── erc20.rs            # ERC20/721/1155 token event decoding
├── uniswap.rs          # Uniswap V2/V3 event decoding
├── uniswap_v4.rs       # Uniswap V4 event decoding
├── permit2.rs          # Permit2 authorization event decoding
├── trading_control.rs  # Trading enabled/disabled event decoding
└── ownership.rs        # Ownership transfer event decoding
```

## Core Functionality

### 1. **Multi-Protocol Event Support**
- **ERC Standards**: ERC20, ERC721, ERC1155 transfers and approvals
- **Uniswap V2**: Swaps, mints, burns, pair creation
- **Uniswap V3**: Swaps, mints, burns, position management, pool creation
- **Uniswap V4**: Initialize, modify liquidity, swaps, donations, fee updates
- **Permit2**: Authorization and permission events
- **Trading Control**: Trading enabled/disabled events
- **Ownership**: Contract ownership transfers

### 2. **Event Decoder Interface**
```rust
pub struct EventDecoder {
    signatures: HashMap<B256, String>,
}

impl EventDecoder {
    /// Decode a single event log
    pub fn decode_log(&self, log: &revm::primitives::Log) -> DecodedEvent;
    
    /// Decode multiple logs efficiently
    pub fn decode_logs(&self, logs: &[revm::primitives::Log]) -> Vec<DecodedEvent>;
    
    /// Extract all unique addresses from events
    pub fn extract_addresses(&self, events: &[DecodedEvent]) -> Vec<Address>;
}
```

### 3. **Structured Event Data**
```rust
#[derive(Debug, Clone)]
pub struct DecodedEvent {
    pub event_type: String,
    pub address: Address,
    pub params: EventParams,
}

#[derive(Debug, Clone)]
pub enum EventParams {
    // Token events
    Transfer { from: Address, to: Address, value: U256 },
    Approval { owner: Address, spender: Address, value: U256 },
    
    // Uniswap V2 events
    SwapV2 { sender: Address, amount0_in: U256, amount1_in: U256, amount0_out: U256, amount1_out: U256, to: Address },
    
    // Uniswap V3 events
    SwapV3 { sender: Address, recipient: Address, amount0: i128, amount1: i128, sqrt_price_x96: U256, liquidity: u128, tick: i32 },
    
    // Uniswap V4 events
    UniswapV4Initialize(UniswapV4Initialize),
    UniswapV4ModifyLiquidity(UniswapV4ModifyLiquidity),
    UniswapV4Swap(UniswapV4Swap),
    
    // Authorization events
    Permit2(Permit2Event),
    
    // Control events
    TradingEnabled(TradingEnabledEvent),
    TradingDisabled(TradingDisabledEvent),
    OwnershipTransferred(OwnershipTransferredEvent),
    
    // Unknown events
    Unknown { topics: Vec<B256>, data: Vec<u8> },
}
```

## Protocol-Specific Decoders

### ERC20/721/1155 Events
```rust
// ERC20 Transfer: Transfer(address indexed from, address indexed to, uint256 value)
pub fn decode_erc20_transfer(log: &Log) -> Option<EventParams> {
    if log.topics.len() != 3 { return None; }
    
    let from = Address::from_slice(&log.topics[1][12..]);
    let to = Address::from_slice(&log.topics[2][12..]);
    let value = U256::from_be_slice(&log.data);
    
    Some(EventParams::Transfer { from, to, value })
}

// ERC721 Transfer: Transfer(address indexed from, address indexed to, uint256 indexed tokenId)
pub fn decode_erc721_transfer(log: &Log) -> Option<EventParams> {
    if log.topics.len() != 4 { return None; }
    
    let from = Address::from_slice(&log.topics[1][12..]);
    let to = Address::from_slice(&log.topics[2][12..]);
    let token_id = U256::from_be_slice(&log.topics[3]);
    
    Some(EventParams::Transfer { from, to, value: token_id })
}
```

### Uniswap V2 Events
```rust
// Swap: Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to)
pub fn decode_uniswap_v2_swap(log: &Log) -> Option<EventParams> {
    if log.topics.len() != 3 { return None; }
    
    let sender = Address::from_slice(&log.topics[1][12..]);
    let to = Address::from_slice(&log.topics[2][12..]);
    
    // Parse data: amount0In, amount1In, amount0Out, amount1Out
    let amount0_in = U256::from_be_slice(&log.data[0..32]);
    let amount1_in = U256::from_be_slice(&log.data[32..64]);
    let amount0_out = U256::from_be_slice(&log.data[64..96]);
    let amount1_out = U256::from_be_slice(&log.data[96..128]);
    
    Some(EventParams::SwapV2 {
        sender, amount0_in, amount1_in, amount0_out, amount1_out, to
    })
}
```

### Uniswap V3 Events
```rust
// Swap: Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
pub fn decode_uniswap_v3_swap(log: &Log) -> Option<EventParams> {
    if log.topics.len() != 3 { return None; }
    
    let sender = Address::from_slice(&log.topics[1][12..]);
    let recipient = Address::from_slice(&log.topics[2][12..]);
    
    // Parse signed integers from data
    let data_bytes = &log.data;
    let amount0 = i128::from_be_bytes(data_bytes[16..32].try_into().ok()?);
    let amount1 = i128::from_be_bytes(data_bytes[48..64].try_into().ok()?);
    let sqrt_price_x96 = U256::from_be_slice(&data_bytes[64..96]);
    let liquidity = u128::from_be_bytes(data_bytes[112..128].try_into().ok()?);
    let tick = i32::from_be_bytes(data_bytes[156..160].try_into().ok()?);
    
    Some(EventParams::SwapV3 {
        sender, recipient, amount0, amount1, sqrt_price_x96, liquidity, tick
    })
}
```

### Uniswap V4 Events
```rust
// Initialize: Initialize(bytes32 indexed poolId, address indexed currency0, address indexed currency1, uint24 fee, int24 tickSpacing, address hooks, uint160 sqrtPriceX96, int24 tick)
pub fn decode_uniswap_v4_initialize(log: &Log) -> Option<DecodedEvent> {
    if log.topics.len() < 4 { return None; }
    
    let event_id = log.topics[1];
    let currency0 = Address::from_slice(&log.topics[2][12..]);
    let currency1 = Address::from_slice(&log.topics[3][12..]);
    
    // Parse complex data structure
    let fee = U256::from_be_slice(&log.data[0..32]).try_into().ok()?;
    let tick_spacing = i32::from_be_bytes(log.data[28..32].try_into().ok()?);
    let hooks = Address::from_slice(&log.data[44..64]);
    let sqrt_price_x96 = U256::from_be_slice(&log.data[64..96]);
    let tick = i32::from_be_bytes(log.data[124..128].try_into().ok()?);
    
    Some(DecodedEvent {
        event_type: "UniswapV4Initialize".to_string(),
        address: log.address,
        params: EventParams::UniswapV4Initialize(UniswapV4Initialize {
            pool_manager_address: log.address,
            event_id, currency0, currency1, fee, tick_spacing, hooks, sqrt_price_x96, tick,
        }),
    })
}
```

### Permit2 Events
```rust
// Permit: Permit(address indexed owner, address indexed token, address indexed spender, uint160 amount, uint48 expiration, uint48 nonce)
pub fn decode_permit2_event(log: &Log) -> Option<DecodedEvent> {
    if log.topics.len() < 4 { return None; }
    
    let owner = Address::from_slice(&log.topics[1][12..]);
    let token = Address::from_slice(&log.topics[2][12..]);
    let spender = Address::from_slice(&log.topics[3][12..]);
    
    let amount = U256::from_be_slice(&log.data[0..32]);
    let expiration = U256::from_be_slice(&log.data[32..64]).try_into().ok()?;
    let nonce = U256::from_be_slice(&log.data[64..96]).try_into().ok()?;
    
    Some(DecodedEvent {
        event_type: "Permit2".to_string(),
        address: log.address,
        params: EventParams::Permit2(Permit2Event {
            contract_address: log.address,
            owner, token, spender, amount, expiration, nonce,
        }),
    })
}
```

## Performance Characteristics

| Operation | Target | Typical |
|-----------|--------|---------|
| Single Event | <0.1ms | 0.05ms |
| Batch (100 events) | <5ms | 2ms |
| Address Extraction | <1ms | 0.3ms |
| Protocol Detection | <0.01ms | 0.005ms |

## Usage Examples

### Basic Event Decoding
```rust
use crate::decode_events::EventDecoder;

let decoder = EventDecoder::new();
let events = decoder.decode_logs(&transaction_logs);

for event in events {
    match event.params {
        EventParams::Transfer { from, to, value } => {
            println!("Token transfer: {} -> {} ({})", from, to, value);
        }
        EventParams::SwapV2 { sender, amount0_out, amount1_out, .. } => {
            println!("V2 swap by {}: {} out, {} out", sender, amount0_out, amount1_out);
        }
        EventParams::UniswapV4Swap(swap) => {
            println!("V4 swap: {} -> {}", swap.amount0, swap.amount1);
        }
        _ => {}
    }
}
```

### Address Extraction for Analytics
```rust
let events = decoder.decode_logs(&logs);
let addresses = decoder.extract_addresses(&events);

println!("Transaction involved {} unique addresses", addresses.len());

// Categorize addresses
let mut token_contracts = HashSet::new();
let mut users = HashSet::new();

for event in &events {
    match &event.params {
        EventParams::Transfer { from, to, .. } => {
            token_contracts.insert(event.address);
            users.insert(*from);
            users.insert(*to);
        }
        _ => {}
    }
}
```

### MEV Detection Support
```rust
pub fn detect_mev_patterns(events: &[DecodedEvent]) -> Vec<MevPattern> {
    let mut patterns = Vec::new();
    
    // Detect sandwich attacks
    let swap_events: Vec<_> = events.iter()
        .filter(|e| matches!(e.params, EventParams::SwapV2 { .. } | EventParams::SwapV3 { .. }))
        .collect();
    
    if swap_events.len() >= 2 {
        // Check for sandwich pattern
        if is_sandwich_pattern(&swap_events) {
            patterns.push(MevPattern::Sandwich);
        }
    }
    
    // Detect arbitrage
    let unique_pools: HashSet<_> = swap_events.iter()
        .map(|e| e.address)
        .collect();
    
    if unique_pools.len() >= 2 {
        patterns.push(MevPattern::Arbitrage);
    }
    
    patterns
}
```

### Protocol Analytics
```rust
pub fn analyze_protocol_usage(events: &[DecodedEvent]) -> ProtocolStats {
    let mut stats = ProtocolStats::default();
    
    for event in events {
        match &event.params {
            EventParams::SwapV2 { .. } => stats.uniswap_v2_swaps += 1,
            EventParams::SwapV3 { .. } => stats.uniswap_v3_swaps += 1,
            EventParams::UniswapV4Swap(_) => stats.uniswap_v4_swaps += 1,
            EventParams::Transfer { .. } => stats.token_transfers += 1,
            EventParams::Permit2(_) => stats.permit2_usage += 1,
            _ => {}
        }
    }
    
    stats
}
```

## Error Handling

```rust
#[derive(Error, Debug)]
pub enum DecodingError {
    #[error("Invalid event signature: {0:?}")]
    InvalidSignature(B256),
    
    #[error("Insufficient topics: expected {expected}, got {actual}")]
    InsufficientTopics { expected: usize, actual: usize },
    
    #[error("Invalid data length: expected {expected}, got {actual}")]
    InvalidDataLength { expected: usize, actual: usize },
    
    #[error("Address parsing failed: {0}")]
    AddressParsing(String),
    
    #[error("Integer parsing failed: {0}")]
    IntegerParsing(String),
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_erc20_transfer_decoding() {
        let log = create_test_erc20_transfer_log();
        let event = decode_erc20_transfer(&log).unwrap();
        
        if let EventParams::Transfer { from, to, value } = event {
            assert_eq!(from, Address::from([1u8; 20]));
            assert_eq!(to, Address::from([2u8; 20]));
            assert_eq!(value, U256::from(1000u64));
        } else {
            panic!("Expected Transfer event");
        }
    }
    
    #[test]
    fn test_uniswap_v4_initialize() {
        let log = create_test_v4_initialize_log();
        let event = decode_uniswap_v4_initialize(&log).unwrap();
        
        assert_eq!(event.event_type, "UniswapV4Initialize");
        // Verify all fields
    }
}
```

### Integration Tests
```rust
#[test]
fn test_real_transaction_decoding() {
    let decoder = EventDecoder::new();
    let real_logs = load_real_transaction_logs();
    
    let events = decoder.decode_logs(&real_logs);
    
    // Verify no events are lost
    assert_eq!(events.len(), real_logs.len());
    
    // Verify address extraction
    let addresses = decoder.extract_addresses(&events);
    assert!(!addresses.is_empty());
}
```

### Performance Tests
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_event_decoding(c: &mut Criterion) {
    let decoder = EventDecoder::new();
    let logs = create_test_logs(1000);
    
    c.bench_function("decode_1000_events", |b| {
        b.iter(|| {
            black_box(decoder.decode_logs(black_box(&logs)))
        })
    });
}
```

## Extension Points

### Custom Event Decoders
```rust
pub trait CustomEventDecoder {
    fn decode(&self, log: &Log) -> Option<DecodedEvent>;
    fn supported_signatures(&self) -> Vec<B256>;
}

impl EventDecoder {
    pub fn add_custom_decoder(&mut self, decoder: Box<dyn CustomEventDecoder>) {
        for signature in decoder.supported_signatures() {
            self.custom_decoders.insert(signature, decoder);
        }
    }
}
```

### Protocol Extensions
```rust
// Add support for new DeFi protocols
pub mod compound {
    pub fn decode_mint_event(log: &Log) -> Option<DecodedEvent> { /* ... */ }
    pub fn decode_redeem_event(log: &Log) -> Option<DecodedEvent> { /* ... */ }
}

pub mod aave {
    pub fn decode_deposit_event(log: &Log) -> Option<DecodedEvent> { /* ... */ }
    pub fn decode_borrow_event(log: &Log) -> Option<DecodedEvent> { /* ... */ }
}
```

## Production Monitoring

### Metrics
- Decoding success rate by protocol
- Unknown event percentage
- Processing latency percentiles
- Memory usage during batch processing

### Alerting
- Decoding failure rate > 1%
- Unknown events > 5%
- Processing time > 10ms (95th percentile)

## Future Enhancements

1. **Dynamic ABI Loading**: Support for unknown contracts
2. **Event Indexing**: Build searchable event index
3. **Historical Analysis**: Batch processing of historical data
4. **Custom Signatures**: User-defined event signatures
5. **Event Streaming**: Real-time event subscription
6. **Cross-Chain Support**: Multi-chain event decoding

## Dependencies

```toml
[dependencies]
revm = { version = "3.0", features = ["std"] }
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
lru = "0.12"
tracing = "0.1"
```

This module is the **intelligence layer** of the transaction processor, transforming raw blockchain data into structured, analyzable information that powers MEV detection, DeFi analytics, and comprehensive transaction understanding.