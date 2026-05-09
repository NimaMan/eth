# TX_FUND_FLOW Core Types

## 🎯 **Overview**

The Core Types module provides the foundational data structures, error types, and utilities shared across all TX_FUND_FLOW components. This module has minimal dependencies and serves as the foundation for the entire TX_FUND_FLOW analytics system.

## 🏗️ **Architecture**

### **Module Structure**
```
core_types/
├── src/
│   ├── lib.rs          # Public API exports
│   ├── types.rs        # Core data structures
│   ├── error.rs        # Error handling types
│   └── utils.rs        # Utility functions
└── Cargo.toml          # Minimal dependencies
```

### **Core Principles**
- **Zero External Dependencies**: Only uses standard library and alloy primitives
- **Shared Foundation**: Provides consistent types across all modules
- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Performance**: Optimized data structures for high-throughput analysis

## 📊 **Core Data Structures**

### **Transaction Types**
```rust
pub struct Transaction {
    pub hash: TransactionHash,           // B256 transaction hash
    pub from_address: Address,           // Transaction initiator
    pub to_address: Option<Address>,     // Recipient (None for contract creation)
    pub value: u128,                     // ETH value in wei
    pub gas_price: u64,                  // Gas price in wei
    pub gas_limit: u64,                  // Gas limit
    pub gas_used: u64,                   // Actual gas used
    pub block_number: BlockNumber,       // Block number (u64)
    pub transaction_index: u32,          // Transaction index in block
    pub status: TransactionStatus,       // Success/Failed/Pending
    pub internal_transfers: Vec<EthMovement>,     // Internal ETH transfers
    pub token_transfers: Vec<TokenMovement>,     // ERC-20 token transfers
    pub timestamp: u64,                  // Block timestamp
}

pub struct EthMovement {
    pub from_address: Address,           // Sender address
    pub to_address: Address,             // Recipient address
    pub amount: u128,                    // Amount in wei
    pub movement_type: MovementType,     // Call/Create/Suicide
}

pub struct TokenMovement {
    pub token_address: Address,          // Token contract address
    pub from_address: Address,           // Sender address
    pub to_address: Address,             // Recipient address
    pub amount: U256,                    // Token amount (raw)
    pub token_symbol: Option<String>,    // Token symbol (e.g., "USDC")
    pub token_decimals: Option<u8>,      // Token decimals
}
```

### **Fund Flow Analysis**
```rust
pub struct FundFlow {
    pub from_address: Address,           // Flow origin
    pub to_address: Address,             // Flow destination
    pub eth_amount: f64,                 // ETH amount (converted from wei)
    pub token_flows: Vec<TokenFlow>,     // Associated token movements
    pub movement_types: Vec<String>,     // Classification tags
    pub gas_cost: f64,                   // Gas cost in ETH
}

pub struct TokenFlow {
    pub token_address: Address,          // Token contract
    pub amount: f64,                     // Normalized token amount
    pub symbol: String,                  // Token symbol
    pub usd_value: Option<f64>,          // USD value if available
}
```

### **Network Components**
```rust
pub struct NetworkNode {
    pub address: Address,                // Node address
    pub entity_type: EntityType,         // User/Contract/Exchange/Pool
    pub label: Option<String>,           // Human-readable label
    pub total_eth_in: f64,              // Total ETH inflow
    pub total_eth_out: f64,             // Total ETH outflow
    pub net_eth_change: f64,            // Net ETH change
    pub transaction_count: u32,          // Number of transactions
    pub risk_score: Option<f64>,        // Risk assessment (0-1)
}

pub struct NetworkEdge {
    pub from_node: Address,              // Source node
    pub to_node: Address,                // Target node
    pub eth_amount: f64,                 // ETH flow amount
    pub token_flows: Vec<TokenFlow>,     // Token flows
    pub transaction_count: u32,          // Number of transactions
    pub edge_type: EdgeType,             // Transfer classification
    pub timestamp_range: (u64, u64),    // First and last transaction
}
```

## 🚨 **Error Handling**

### **QarqaError Enum**
```rust
pub enum QarqaError {
    Database(String),                    // Database connection/query errors
    Io(std::io::Error),                 // File system errors
    Serialization(String),               // JSON/serialization errors
    InvalidAddress(String),              // Address parsing errors
    InvalidTransaction(String),          // Transaction validation errors
    Conversion(String),                  // Type conversion errors
    Other(String),                       // Generic errors
}

pub type QarqaResult<T> = Result<T, QarqaError>;
```

### **Error Handling Patterns**
```rust
// Error propagation with context
pub fn parse_transaction_hash(hash_str: &str) -> QarqaResult<TransactionHash> {
    hash_str.parse::<B256>()
        .map_err(|e| QarqaError::InvalidTransaction(
            format!("Invalid transaction hash '{}': {}", hash_str, e)
        ))
}

// Graceful error handling
pub fn safe_address_lookup(addr: &str) -> Option<Address> {
    match addr.parse::<Address>() {
        Ok(address) => Some(address),
        Err(_) => {
            tracing::warn!("Failed to parse address: {}", addr);
            None
        }
    }
}
```

## 🛠️ **Utility Functions**

### **Wei/ETH Conversion**
```rust
use tx_fund_flow_core_types::utils::*;

// Convert wei to ETH with precision
let eth_amount = wei_to_eth(1_000_000_000_000_000_000u128); // 1.0 ETH
let wei_amount = eth_to_wei(1.5); // 1.5 ETH in wei

// Format for display
let formatted = format_eth_amount(1_234_567_890_123_456_789u128); // "1.234 ETH"
```

### **Address Formatting**
```rust
// Truncate addresses for display
let short_addr = format_address(&address); // "0x1234...abcd"

// Validate Ethereum addresses
let is_valid = is_valid_address("0x742d35Cc6032C0532c6FAEF");
```

### **Mathematical Utilities**
```rust
// Statistical functions for analysis
let avg = moving_average(&values, window_size);
let std_dev = standard_deviation(&values);
let correlation = correlation_coefficient(&x_values, &y_values);
let pct_change = percentage_change(old_value, new_value);
```

## 🎨 **Usage Examples**

### **Basic Transaction Creation**
```rust
use tx_fund_flow_core_types::*;
use alloy::primitives::Address;

let transaction = Transaction {
    hash: "0xabc123...".parse().unwrap(),
    from_address: "0x742d35Cc6032C0532c6FAEF".parse().unwrap(),
    to_address: Some("0x1234567890123456789012345678901234567890".parse().unwrap()),
    value: eth_to_wei(1.5), // 1.5 ETH
    gas_price: 20_000_000_000, // 20 gwei
    gas_limit: 21_000,
    gas_used: 21_000,
    block_number: 18_500_000,
    transaction_index: 42,
    status: TransactionStatus::Success,
    internal_transfers: Vec::new(),
    token_transfers: Vec::new(),
    timestamp: 1698765432,
};
```

### **Fund Flow Analysis**
```rust
use tx_fund_flow_core_types::*;

let fund_flow = FundFlow {
    from_address: "0x742d35Cc6032C0532c6FAEF".parse().unwrap(),
    to_address: "0x1234567890123456789012345678901234567890".parse().unwrap(),
    eth_amount: 1.5,
    token_flows: vec![
        TokenFlow {
            token_address: "0xA0b86a33E6441b8Ec1...".parse().unwrap(),
            amount: 1000.0,
            symbol: "USDC".to_string(),
            usd_value: Some(1000.0),
        }
    ],
    movement_types: vec!["Direct Transfer".to_string()],
    gas_cost: 0.0042, // 0.0042 ETH gas cost
};

// Analyze fund flow
let is_high_value = fund_flow.eth_amount > 10.0;
let has_stablecoins = fund_flow.token_flows.iter()
    .any(|tf| ["USDC", "USDT", "DAI"].contains(&tf.symbol.as_str()));
```

### **Network Node Creation**
```rust
use tx_fund_flow_core_types::*;

let node = NetworkNode {
    address: "0x742d35Cc6032C0532c6FAEF".parse().unwrap(),
    entity_type: EntityType::User,
    label: Some("Vitalik".to_string()),
    total_eth_in: 1250.5,
    total_eth_out: 1100.2,
    net_eth_change: 150.3,
    transaction_count: 1337,
    risk_score: Some(0.05), // Low risk
};

// Analyze node characteristics
let is_profitable = node.net_eth_change > 0.0;
let is_active = node.transaction_count > 100;
let is_high_risk = node.risk_score.unwrap_or(0.0) > 0.8;
```

## 🧪 **Testing**

### **Running Tests**
```bash
cd /home/nima/code/crypto/rust/tx_fund_flow/core_types
cargo test
```

### **Test Coverage**
- ✅ **Utility Functions**: Wei/ETH conversion, address formatting, mathematical functions
- ✅ **Error Handling**: Error creation, propagation, and conversion
- ✅ **Type Safety**: Address parsing, transaction validation
- ✅ **Data Structures**: Transaction creation, fund flow analysis

### **Example Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wei_eth_conversion() {
        assert_eq!(wei_to_eth(1_000_000_000_000_000_000u128), 1.0);
        assert_eq!(eth_to_wei(2.5), 2_500_000_000_000_000_000u128);
    }

    #[test]
    fn test_address_formatting() {
        let addr = "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7".parse().unwrap();
        assert_eq!(format_address(&addr), "0x742d...e4a7");
    }
}
```

## 🔗 **Integration with Other Modules**

### **Used By**
- **data_access**: Uses Transaction, Address, and error types for database operations
- **tx_simulation**: Uses Transaction and FundFlow for simulation results
- **network_building**: Uses NetworkNode, NetworkEdge for graph construction
- **api_layer**: Uses all types for CLI and API responses

### **Dependencies**
- **alloy::primitives**: For Address, B256, U256 types
- **serde**: For JSON serialization (optional feature)
- **tracing**: For logging (optional feature)

## 📈 **Performance Characteristics**

- **Memory Usage**: Minimal - core types use stack allocation where possible
- **Serialization**: Fast JSON serialization with serde
- **Type Safety**: Zero-cost abstractions with compile-time guarantees
- **Conversion**: Optimized wei/ETH conversion functions

## 🔄 **Development Guidelines**

### **Adding New Types**
1. Add to `types.rs` with comprehensive documentation
2. Implement necessary traits (Debug, Clone, PartialEq)
3. Add serialization support if needed (serde)
4. Create utility functions in `utils.rs`
5. Add comprehensive tests

### **Error Handling**
1. Use `QarqaResult<T>` for all fallible operations
2. Provide descriptive error messages with context
3. Use `map_err` for error conversion
4. Log errors at appropriate levels

### **Backwards Compatibility**
- Never remove public fields from existing structs
- Use deprecation warnings before removing functionality
- Version breaking changes appropriately

This module provides the foundation for all TX_FUND_FLOW operations, ensuring type safety, performance, and consistency across the entire analytics system.