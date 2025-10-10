# Simulator Module

## Overview

The simulator module provides transaction simulation capabilities with integrated signal detection. It executes transactions against current blockchain state, performs buy/sell testing, and passes results to the signal manager for detection.

## Architecture

### Core Components

#### 1. SimulationManager
Central coordinator that manages per-pool simulation and passes results to signal detection.

```rust
pub struct SimulationManager {
    // Simulators
    mempool_simulator: Arc<MempoolSimulator>,
    liquidity_removal_simulator: Arc<LiquidityRemovalSimulator>,

    // Queue management
    queue: Arc<Mutex<SimulationQueue>>,

    // Signal detection
    signal_manager: Arc<Mutex<SignalManager>>,
    token_cache: Arc<TokenTrackingCache>,

    // Configuration
    max_concurrent_simulations: usize,

    // Statistics
    stats: Arc<Mutex<ManagerStats>>,
}
```

**Key Features:**
- Manages a priority queue and runs simulations concurrently
- For CreatorTransaction, simulates EACH pool independently and sends a result per pool to SignalManager
- For other categories, sends a single SimulationResult to SignalManager
- Tracks simulation statistics

#### 2. MempoolSimulator
Unified simulator that wraps a single shared `Arc<TxSimulator>` used across components to avoid DB write-locks. Provides:
- `simulate_mempool_tx` with automatic nonce retry for "nonce too high, expected N"
- `simulate_pool_buy_sell(_simple)` delegating to tx_processor via the PoolBuySellSimulator wrapper

#### 3. PoolBuySellSimulator (wrapper)
Thin wrapper around tx_processor’s `check_can_buy_sell_pool` building a PoolBuySellParameters and returning PoolBuySellSimulationResult (can_buy, can_sell, buy/sell tax, etc.).

#### 4. LiquidityRemovalSimulator
Specialized simulator for liquidity removals (MEV accounts with 0 balance). Supports simulating at the transaction’s block when available; falls back to latest if historical state is pruned.

#### 5. SimulationQueue
Priority queue for managing simulation requests.

**Priority Levels:**
- `Critical`: Contract creations, high-value transactions
- `High`: Creator transactions, ownership changes
- `Normal`: Regular DEX interactions
- `Low`: Other transactions

## Data Flow and Calculations

### Per‑Pool Simulation Flow (Creator Transactions)

```
Incoming tx (CreatorTransaction)
        │
        ▼
Determine token address (router or cache lookup)
        │
        ▼
Discover pools for token (TokenTrackingCache)
        │
        ▼
For EACH pool (run independently / concurrently)
  ┌───────────────────────────────────────────────────────────────┐
  │ 1) Prepare optional tx call (if simulating tx itself)         │
  │ 2) Build PoolBuySellParameters { token, pool, at_block?, … }    │
  │ 3) mempool_simulator.simulate_pool_buy_sell(config)           │
  │ 4) Build SimulationResult {                                   │
  │       token_address, pool_address, pool_type,                 │
  │       pool_viability_result (can_buy/can_sell/taxes),         │
  │       liquidity_removal_result (if applicable)                 │
  │     }                                                         │
  │ 5) signal_manager.process_simulation_result(result)           │
  └───────────────────────────────────────────────────────────────┘
```

Notes:
- Each pool generates its own SimulationResult and downstream signals; one noisy pool does not block others.
- If no pools are found for a creator, a single “no-pools” result is sent to SignalManager (for logging/consistency).
- **Current limitation**: the creator transaction itself is not yet baked into the buy/approve/sell probe. The current test therefore reflects the state *before* the pending creator call executes. The follow-up work on `feature/apply-creator-tx-before-buysell` will capture the creator mempool tx and inject it as the `prior_tx` when running the pool buy/approve/sell chain so the probe reflects post-call state without a separate primary simulation.

### Other Categories

```
Incoming tx (Non‑creator category)
        │
        ▼
Run appropriate simulation path (mempool_simulator …)
        │
        ▼
Build single SimulationResult
        │
        ▼
signal_manager.process_simulation_result(result)
```

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
High-level `simulate_request` flow:
- CreatorTransaction:
  - Liquidity removal → LiquidityRemovalSimulator; send result to SignalManager
  - Otherwise → per-pool simulation; send EACH pool’s SimulationResult to SignalManager
- Other categories:
  - Single simulation; send one SimulationResult to SignalManager

### Practical Considerations

- Shared DB connection: MempoolSimulator wraps a single shared `Arc<TxSimulator>` so pool simulations and other paths reuse the same provider and avoid LMDB writer locks.
- Nonce handling: `simulate_mempool_tx` retries on “nonce too high, expected N”. For historical transactions (or MEV sequences), prefer at‑block simulation to avoid nonce/basefee drift.
- Historical state: When simulating at a transaction’s original block, a pruned node may report “state at block is pruned”. In that case, fall back to latest‑block behavior with best‑effort nonce handling.

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
let head_manager = Arc::new(CanonicalHeadCache::new());
let mempool_simulator = Arc::new(MempoolSimulator::new(&reth_db_path, head_manager.clone())?);
// Start the canonical head listener so simulations always have fresh headers
let _head_task = head_manager.spawn_head_listener(ipc_path);
head_manager.wait_for_latest_header(Duration::from_secs(10)).await?;
let publisher = Arc::new(tokio::sync::Mutex::new(SignalPublisher::new(cfg).await?));
let simulation_manager = SimulationManager::new(
    mempool_simulator,
    token_cache,
    signal_config,
    publisher,
    max_workers,
);
```

### Signal Configuration
```rust
SignalManagerConfig {
    log_dir: PathBuf,
    tax_detection: TaxDetectionConfig,
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
