# Simulator Module

## Overview

The simulator module provides transaction simulation capabilities with integrated signal detection. It executes transactions against current blockchain state, performs buy/sell testing, and passes results to the signal manager for detection.

## Architecture

### Core Components

#### 1. SimulationManager
The central coordinator that manages the simulation flow and passes results to signal detection.

```rust
pub struct SimulationManager {
    // Simulators
    tx_simulator: Arc<TxSimulator>,
    buy_sell_simulator: Arc<SequentialBuySellSimulator>,
    
    // Queue management
    queue: Arc<Mutex<SimulationQueue>>,
    
    // Signal detection
    signal_manager: Arc<Mutex<SignalManager>>,
    token_cache: Arc<TokenTrackingCache>,
    
    // Configuration
    max_concurrent_simulations: usize,
    enable_caching: bool,
    
    // Statistics
    stats: Arc<Mutex<ManagerStats>>,
}
```

**Key Features:**
- Manages transaction simulation queue with priority ordering
- Runs transaction and buy/sell simulations
- Passes results to SignalManager for detection
- Tracks simulation statistics

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
3. Calculate buy/sell taxes from state changes
4. Return results with taxes, can_buy/can_sell flags, and state changes

#### 4. SimulationQueue
Priority queue for managing simulation requests.

**Priority Levels:**
- `Critical`: Contract creations, high-value transactions
- `High`: Creator transactions, ownership changes
- `Normal`: Regular DEX interactions
- `Low`: Other transactions

## Data Flow and Calculations

### 1. Transaction Submission
```rust
simulation_manager.submit(SimulationRequest {
    tx: MempoolTransaction,
    category: TransactionCategory,
    priority: SimulationPriority,
    simulation_type: SimulationType,
})
```

### 2. Simulation Processing
```rust
async fn simulate_request(&self, request: SimulationRequest) -> SimulationResult {
    // 1. For ContractCreation or CreatorTransaction:
    let (tx_result, bs_result, token_addr, pool_addr) = 
        self.simulate_tx_with_buy_sell(&request).await;
    
    // 2. Build result structure
    SimulationResult {
        request: request,
        tx_simulation: Some(tx_result),
        buy_sell_result: Some(BuySellResult {
            can_buy: buy_result.success,
            can_sell: sell_result.success,
            buy_tax: None,    // Currently set to None, taxes calculated in SignalManager from state changes
            sell_tax: None,   // Currently set to None, taxes calculated in SignalManager from state changes
            tokens_received: None,
            eth_received_on_sell: None,
            buy_state_changes: Some(buy_result.state_changes),
            sell_state_changes: Some(sell_result.state_changes),
        }),
        token_address: Some(token_addr),
        pool_address: pool_addr,
        simulation_time_ms: elapsed_ms,
    }
    
    // 3. Pass to SignalManager
    signal_manager.process_simulation_result(&result).await;
}
```

### 3. SignalManager Processing

The SignalManager receives the SimulationResult and:

1. **Extracts key values**:
   - `buy_tax`, `sell_tax`, `can_buy`, `can_sell` from BuySellResult
   - State changes for liquidity and stablecoin detection

2. **Runs detectors**:
   - **TaxDetector**: Calculates taxes from state changes, detects honeypots/high taxes
   - **TradingStatusDetector**: Emits TradingEnabled if can_buy && can_sell
   - **LiquidityDetector**: Checks for pool drains using TokenTrackingCache
   - **StablecoinDetector**: Tracks USDT/USDC mints/burns

3. **Returns signals** as `Vec<Signal>`:
   - TradingEnabled
   - HighTaxWarning
   - LiquidityRemoval
   - ScamDetection

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