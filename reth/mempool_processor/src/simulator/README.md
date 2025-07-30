# Simulator Module

## Overview

The simulator module provides transaction simulation capabilities with integrated signal detection. It executes transactions against current blockchain state, performs buy/sell testing, and automatically detects trading signals based on simulation results.

## Architecture

### Core Components

#### 1. SimulationManager
The central coordinator that manages the entire simulation and signal detection flow.

```rust
pub struct SimulationManager {
    // Simulation components
    tx_simulator: Arc<TxSimulator>,
    buy_sell_simulator: Arc<SequentialBuySellSimulator>,
    queue: Arc<Mutex<SimulationQueue>>,
    
    // Signal detection (integrated)
    signal_manager: Arc<Mutex<SignalManager>>,
    token_cache: Arc<AddressTrackingCache>,
    
    // Optional metric counters
    trading_enabled_counter: Option<Arc<AtomicU64>>,
    honeypot_counter: Option<Arc<AtomicU64>>,
    high_tax_counter: Option<Arc<AtomicU64>>,
}
```

**Key Features:**
- Manages transaction simulation queue with priority ordering
- Integrates signal detection directly into simulation flow
- Uses token cache for context-aware signal detection
- Automatically publishes detected signals
- Updates external metrics if provided

#### 2. TxSimulator
Executes individual transactions against the current blockchain state.

**Capabilities:**
- Loads current state from Reth DB
- Executes transaction in EVM
- Tracks state changes (balance updates, storage changes)
- Returns success/failure with gas usage

#### 3. SequentialBuySellSimulator
Tests token tradability by simulating buy and sell transactions.

**Process:**
1. Simulate 0.1 ETH buy transaction
2. If successful, simulate selling received tokens
3. Calculate taxes from price impact
4. Detect honeypots (can buy but can't sell)

#### 4. SimulationQueue
Priority queue for managing simulation requests.

**Priority Levels:**
- `Critical`: Contract creations, high-value transactions
- `High`: Creator transactions, ownership changes
- `Normal`: Regular DEX interactions
- `Low`: Other transactions

## Integrated Signal Detection Flow

### 1. Submission
```rust
simulation_manager.submit(SimulationRequest {
    tx: MempoolTransaction,
    category: TransactionCategory,
    priority: SimulationPriority,
    simulation_type: SimulationType,
})
```

### 2. Internal Processing
```rust
async fn simulate_request(&self, request: SimulationRequest) {
    // 1. Execute simulation
    let result = self.execute_simulation(request).await;
    
    // 2. Get token context from cache
    let token_info = self.token_cache.get_token_info(token_address).await;
    
    // 3. Detect signals with context
    let signals = self.signal_manager.process_simulation_result(
        &result,
        token_info.as_ref(),
    ).await;
    
    // 4. Log and publish signals
    for signal in signals {
        self.log_signal(&signal);
        self.update_counters(&signal);
        self.publish_signal(signal).await;
    }
}
```

### 3. Context-Aware Signal Detection

#### For Contract Creations:
```rust
if buy_sell_result.can_buy && buy_sell_result.can_sell {
    // New token with trading enabled!
    emit TradingEnabledSignal
}
```

#### For Creator Transactions:
```rust
// Check previous state from cache
if !token_info.trading_enabled && buy_sell_result.can_buy && buy_sell_result.can_sell {
    // Trading just got enabled!
    emit TradingEnabledSignal
}

if token_info.trading_enabled && !buy_sell_result.can_sell {
    // Honeypot - was tradeable, now can't sell!
    emit HoneypotSignal
}
```

## Simulation Types

### 1. TransactionOnly
- Executes just the mempool transaction
- Used for simple transfers or view functions
- No buy/sell testing

### 2. TransactionWithBuySell
- Executes the transaction first
- If successful, performs buy/sell test
- Used for contract creations and trading control functions

### 3. BuySellOnly
- Skips transaction execution
- Only tests current tradability
- Used for periodic token health checks

## Signal Types Detected

### 1. Trading Enabled
- **Trigger**: Buy and sell simulations both succeed
- **Context**: Only if trading wasn't already enabled (checked via cache)
- **Data**: Token address, tax rates, transaction hash

### 2. High Tax Warning
- **Trigger**: Buy or sell tax > 25%
- **Types**: HighBuyTax, HighSellTax, PotentialHoneypot (>50%)
- **Data**: Token address, buy/sell tax percentages

### 3. Honeypot Detection
- **Trigger**: Previously tradeable token can no longer be sold
- **Context**: Requires token cache showing previous trading_enabled = true
- **Data**: Token address, scammer address, reason

### 4. Liquidity/Scam Detection
- **Trigger**: Significant liquidity removal or pool drain
- **Handled by**: Separate liquidity detector in signal manager
- **Data**: Pool address, ETH drained, percentage change

## Performance Characteristics

### Queue Management
- Maximum queue size: 10,000 transactions
- Batch processing: Up to 50 transactions
- Critical transactions bypass batching
- Low priority transactions dropped when full

### Timing
- Transaction simulation: 5-10ms average
- Buy/sell testing: 10-20ms additional
- Signal detection: <1ms
- Total end-to-end: 15-30ms per transaction

### Concurrency
- Configurable worker count (default: based on CPU cores)
- Parallel simulation execution
- Thread-safe queue management
- Async signal publishing

## Configuration

### SimulationManager Creation
```rust
let simulation_manager = SimulationManager::new(
    tx_simulator,
    buy_sell_simulator,
    token_cache,
    signal_config,
    max_workers,
);

// Optional: Set metric counters
simulation_manager.set_metric_counters(
    trading_enabled_counter,
    honeypot_counter,
    high_tax_counter,
);
```

### Signal Configuration
```rust
SignalManagerConfig {
    log_dir: PathBuf,
    enable_liquidity_detection: bool,
    enable_stablecoin_detection: bool,
}
```

## Usage Example

```rust
// Main processing loop (simplified with new architecture)
loop {
    let batch = ipc_client.get_transactions().await?;
    
    for tx in batch {
        // Classify transaction
        let classification = tx_router.route(&tx);
        
        // Skip if not interesting
        if !classification.requires_simulation {
            continue;
        }
        
        // Create simulation request
        let request = SimulationRequest {
            tx: tx.clone(),
            category: classification.category,
            priority: classification.priority,
            simulation_type: determine_simulation_type(&classification),
        };
        
        // Submit - signal detection happens internally!
        simulation_manager.submit(request).await?;
    }
}
```

## Benefits of Integrated Architecture

1. **Encapsulation**: Signal detection logic is hidden from main loop
2. **Context Preservation**: All simulation context available for detection
3. **Atomic Operations**: Simulation and signal detection in single flow
4. **Simplified API**: Just submit() - no result handling needed
5. **Better Performance**: No queue polling or result passing overhead
6. **Maintainability**: Single place to update signal detection logic

## Future Enhancements

1. **Additional Signals**:
   - Tax change detection (comparing before/after)
   - MEV opportunity detection
   - Sandwich attack detection

2. **Optimization**:
   - Result caching for repeated simulations
   - Parallel signal detection
   - Batch signal publishing

3. **Monitoring**:
   - Simulation success rates by category
   - Signal detection rates
   - Performance metrics per signal type