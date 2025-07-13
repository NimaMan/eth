# Token Tracking Cache Design

## Purpose

The token tracking cache is a high-performance, thread-safe cache that bridges Python's token data with real-time mempool transaction analysis for signal detection. It serves as the critical link between:

1. **Static token data** from Python (who created/owns tokens, which pools exist)
2. **Dynamic mempool activity** (real-time transactions from creators/owners)
3. **Signal generation** (immediate alerts when critical actions detected)

### Core Objectives

- **Fast Address Matching**: O(1) lookup to check if mempool tx is from a tracked creator/owner
- **Function History Tracking**: Build patterns of creator/owner behavior over time
- **Pool Monitoring**: Track pool addresses for post-simulation drain detection
- **Future DB Integration**: Structured data ready for persistence and analysis

## Signal Types and Emission Points

The system emits different signal types at different stages of processing:

### 1. Function Detection Signals (Immediate)
**Emitted by**: `FunctionDetector` when critical functions detected
**Signal Type**: `SignalAlert`
- **Liquidity Removal Signal**: When removeLiquidity functions detected
- **Trading Enabled Signal**: When enableTrading/openTrading detected  
- **General Function Signal**: Other important functions

These signals are emitted **immediately** upon detection, before simulation.

### 2. Simulation-Based Signals (Post-Simulation)
**Emitted by**: `SignalDetector` after transaction simulation
**Signal Type**: `SimulationSignal`
- **Pool Drain Signal**: When pool reserves drop >60% or below 0.3 ETH
- **Scam Alert Signal**: When significant drain detected
- **Stablecoin Burns/Mints**: USDC/USDT burn or mint events

### 3. Creator/Owner Action Signals (With Cache Integration)
**Purpose**: Enhanced signals when tx.from matches tracked creators/owners
- Combines function detection with creator/owner role information
- Tracks patterns of behavior over time
- Enables predictive detection based on historical actions

## Cache Architecture

```
TokenTrackingCache
├── AddressToTokenCache (Primary lookup)
│   ├── Creator addresses → Token info + function history
│   └── Owner addresses → Token info + function history
│
├── TokenInfoCache (Token metadata)
│   ├── Token address → Full token data
│   ├── Pools → Pool addresses for this token
│   └── Ownership history
│
└── PoolAddressCache (Pool monitoring)
    └── Pool address → Token address + reserves
```

## Complete Signal Flow

```
Mempool TX → Is from tracked address? → Record function → Critical function? → Generate signal
     ↓
Simulation → State changes → Affects our pools? → Drain detected? → Generate signal
```

### Detailed Flow:

1. **Mempool Transaction Analysis**
   - Check if `tx.from` is in our tracked addresses (creators/owners)
   - If yes: Record the function call in their history
   - If critical function (removeLiquidity, etc.): Generate immediate signal

2. **Transaction Simulation**
   - Simulate transactions (prioritize those from tracked addresses)
   - Get state changes from simulation

3. **Pool State Analysis**
   - Check if any state changes affect our tracked pools
   - If pool drain detected (>60% or <0.3 ETH): Generate signal

## Key Design Decisions

### 1. Address-First Indexing
- **Primary key**: Ethereum addresses (creators/owners)
- **Why**: Mempool txs only give us `from` address, need O(1) lookup
- **Trade-off**: Duplicate data when one address owns multiple tokens

### 2. Function History Storage
```rust
pub struct AddressActivity {
    pub token_address: String,
    pub role: AddressRole,  // Creator, Owner, Both
    pub function_history: Vec<FunctionCall>,
    pub last_activity: u64,
}

pub struct FunctionCall {
    pub tx_hash: String,
    pub function_selector: [u8; 4],
    pub function_name: String,
    pub block_number: Option<u64>,
    pub timestamp: u64,
    pub args_summary: Option<String>,  // For critical functions
}
```

### 3. Critical Function Detection
```rust
// Critical functions that trigger immediate signals
const CRITICAL_FUNCTIONS: &[&str] = &[
    "removeLiquidity",
    "removeLiquidityETH",
    "removeLiquidityWithPermit",
    "transferOwnership",
    "renounceOwnership",
    "pause",
    "blacklist",
    "setMaxTxAmount",
    "setMaxWalletSize",
];
```

### 4. Pool State Tracking
- Store pool addresses with token mapping
- Track latest reserves for drain detection
- Enable post-simulation state change analysis

## Key Functionality

### 1. Creator/Owner Tracking
- **Store**: All token creators and current owners from Python
- **Match**: Instantly identify if mempool tx is from a tracked address
- **Track**: Record all functions called by each creator/owner
- **Pattern**: Build behavioral profiles (e.g., "always removes liquidity after 100 blocks")

### 2. Pool Address Monitoring
- **Store**: All pool addresses for each token
- **Lookup**: Quick check if simulation state change affects our pools
- **Reserves**: Track current reserve levels for drain detection
- **Priority**: Mark primary pools vs secondary pools

### 3. Function Call History
- **Record**: Every function called by tracked addresses
- **Analyze**: Detect patterns and suspicious sequences
- **Alert**: Enhanced signals when known bad actors detected
- **Future**: Feed ML models for predictive detection

## Usage Example

```rust
// When mempool tx arrives
let tx_from = "0x123...";
let function_selector = &tx.input[0..4];

// Fast lookup: Is this address a creator/owner?
if let Some(address_info) = cache.get_address_info(tx_from).await {
    // Record the function call
    cache.record_function_call(
        tx_from,
        function_selector,
        tx.hash,
        timestamp,
    ).await;
    
    // Check if it's a critical function
    if is_critical_function(function_selector) {
        // Generate immediate signal with role context
        signal_detector.emit_creator_action_signal(
            address_info.token_address,
            tx_from,
            function_selector,
            address_info.role, // Creator vs Owner matters!
        );
    }
}

// After simulation
for state_change in simulation_results.state_changes {
    if let Some(pool_info) = cache.get_pool_info(&state_change.address).await {
        // Check for significant reserve changes
        if is_drain_detected(&pool_info, &state_change) {
            signal_detector.emit_pool_drain_signal(
                pool_info,
                state_change,
                tx_from,
            );
        }
    }
}
```

## Performance Considerations

1. **Memory Usage**: ~1KB per tracked address (including function history)
2. **Lookup Speed**: O(1) for address → token mapping
3. **Update Frequency**: Batch updates from Python every block
4. **Eviction Policy**: LRU for addresses inactive > 24 hours

## Signal Types Generated

1. **CreatorLiquidityRemoval**: Creator removing liquidity
2. **OwnershipChange**: Ownership transferred or renounced  
3. **TradingPaused**: Owner paused trading
4. **SignificantDrain**: Pool reserves dropped > 50%
5. **SuspiciousPattern**: Creator following known scam patterns

## Future Enhancements

1. **Pattern Learning**: ML model to detect scam patterns from function sequences
2. **Cross-Token Analysis**: Detect creators with multiple scam tokens
3. **MEV Protection**: Detect sandwich attack attempts on tracked tokens
4. **Collaborative Filtering**: Share creator reputation across instances