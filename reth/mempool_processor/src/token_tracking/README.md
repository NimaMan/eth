# Token Tracking Module

This module provides a real-time cache of token and pool information, subscribing to updates from the Python token tracker via ZMQ.

## Overview

The token tracking system maintains a comprehensive cache of:
- All tracked tokens with their metadata, ownership, and tax information
- All liquidity pools and their current reserves
- All "creator" addresses (token creators, owners, tax setters) for fast lookup
- Simulation results (can buy/sell, measured taxes, honeypot detection)

## Architecture

```
Python Token Tracker (ZMQ Publisher)
         ↓
TokenTrackingSubscriber (ZMQ Subscriber)
         ↓
TokenTrackingCache
    ├── tokens: HashMap<String, TrackedToken>
    ├── all_creators: HashSet  // All authority addresses
    └── all_pools: HashSet     // All pool addresses
```

## Key Components

### TrackedToken
Represents a single token with all its information:
- Basic metadata (symbol, name, decimals, supply)
- Authority addresses (creator, owner, tax setters)
- Trading status and tax rates
- Associated pools with liquidity reserves
- Simulation results from Rust (can buy/sell, honeypot status)

### TokenTrackingCache
The main cache that:
- Subscribes to Python updates via ZMQ
- Maintains fast HashSets for mempool transaction filtering
- Stores simulation results from the Rust simulation engine
- Provides methods to query tokens, pools, and authority addresses

## Usage

### Basic Usage
```rust
use mempool_processor::token_tracking::{TokenTrackingSubscriber, TokenTrackingCache};

// Create subscriber with ETH threshold
let mut subscriber = TokenTrackingSubscriber::new(0.1); // 0.1 ETH threshold
let cache = subscriber.get_cache();

// Start listening in background
tokio::spawn(async move {
    subscriber.start_listening().await.unwrap();
});

// Use the cache
let creators = cache.get_all_creators().await;
let pools = cache.get_all_pools().await;
let token_addresses = cache.get_all_token_addresses().await;
```

### Checking Addresses in Mempool Processing
```rust
// Fast O(1) lookup to check if address is a token authority
if cache.is_creator(&tx.from).await {
    // This is a creator/owner/tax setter transaction
    classify_as_creator_transaction(&tx);
}

// Check if any state change affects a pool
let all_pools = cache.get_all_pools().await;
for (address, _) in simulation_result.state_changes {
    if all_pools.contains(&address) {
        // Pool liquidity was affected
    }
}
```

### Getting Token Information
```rust
// Get full token information
if let Some(token) = cache.get_token("0x...").await {
    println!("Token: {} ({})", token.symbol.unwrap_or_default(), token.name.unwrap_or_default());
    println!("Creator: {}", token.creator_address);
    println!("Owner: {}", token.current_owner);
    println!("Buy Tax: {:?}%", token.buy_tax);
    println!("Pools: {}", token.pools.len());
    
    // Check simulation results
    if let Some(sim_data) = token.simulation_data {
        println!("Can Buy: {}", sim_data.can_buy);
        println!("Can Sell: {}", sim_data.can_sell);
        println!("Is Honeypot: {}", sim_data.is_honeypot);
    }
}

// Get primary pool (highest liquidity)
if let Some(pool) = cache.get_primary_pool("0x...").await {
    println!("Primary pool: {} with {} ETH", pool.pool_address, pool.denom_reserve);
}
```

### Updating Simulation Results
```rust
use mempool_processor::token_tracking::SimulationData;

// After running buy/sell simulation
let sim_data = SimulationData {
    can_buy: true,
    can_sell: false,
    measured_buy_tax: Some(5.0),
    measured_sell_tax: None,
    is_honeypot: true,
    last_simulated_block: 12345678,
    simulation_error: None,
};

cache.set_simulation_results("0x...", sim_data).await;
```

### Getting All Token Addresses
```rust
// Get all unique token addresses in the system
let all_tokens = cache.get_all_token_addresses().await;
println!("Tracking {} tokens", all_tokens.len());

// Process each token
for token_address in all_tokens {
    if let Some(token_info) = cache.get_token(&token_address).await {
        // Process token...
    }
}
```

## Data Flow

1. **Python → Rust**: Token updates arrive via ZMQ with:
   - Token metadata and ownership
   - Current tax rates from contract reads
   - Pool addresses and reserves
   - Trading status

2. **Rust Simulations → Cache**: Simulation results are stored:
   - Can buy/sell status from actual swap attempts
   - Measured tax rates from simulations
   - Honeypot detection
   - Simulation errors

3. **Cache → Signal Detection**: The cache provides:
   - Fast lookups for transaction classification
   - Pool addresses for liquidity monitoring
   - Combined Python + simulation data for signal generation

## Performance Considerations

- Pre-computed HashSets enable O(1) lookups for address checking
- Cache size limits prevent unbounded growth (100K pools, 50K tokens)
- Scam tokens are evicted after 5000 blocks of inactivity
- All updates are done under async locks for thread safety

## Example: Token Cache Inspector

See `examples/token_cache_inspector.rs` for a complete example that:
- Connects to the token cache
- Lists all authority addresses (creators/owners/tax setters)
- Lists all tracked pools
- Displays detailed information for 10 sample tokens
- Shows all token addresses in the system