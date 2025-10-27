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
- Consumes transaction classifications from `TransactionRouter` and token/pool ownership context from the Python tracking service so only brand-new deployers need additional probing.

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
- The simulator replays the creator's pending transaction sequence (ordered by nonce) before the buy/approve/sell probe. Launch helpers such as `openTrading()` and router seeding calls are applied via `prior_txs` so the viability check reflects post-call state in a single pass.
- Token ownership and pool membership come from the Python token-tracking service. The router therefore already knows about the vast majority of creator/token pairs; only first-time deployers are missing until the deployment is mined.
- When a contract creation appears in the mempool, the manager performs a quick metadata probe (`reth_chain_query::get_token_metadata`) to confirm the bytecode exposes ERC-20 semantics. If the launch is legitimate, we retain the deploy tx only long enough to connect a same-block trading-enablement attempt; otherwise the Python feed will surface the token on the next block and we drop the creation from our pending buffer.

#### Pending Sequence Buffer (unmined creator history)

```
new creator tx ---> SimulationQueue (priority = Critical/High)
                    │
                    ├─> submit() stores the request and (if simulated)
                    │   captures the resulting ProcessedTransaction
                    │   inside `prior_tx_history[creator, token]`
                    │
                    └─> when the next creator tx arrives, we:
                         1. pull the existing deque (bounded to PRIOR_TX_HISTORY_LIMIT = 6)
                         2. replay each ProcessedTransaction in nonce order
                            on the sequential simulator chain
                         3. execute the current transaction and, if needed,
                            run pool viability checks.
```

*Current pruning policy*

- The buffer is keyed by `(creator, target_token)` and only tracks transactions that are still in the mempool (unmined). Canonical-head updates will evict any entries whose hashes appear in the new block.
- Entries are deduplicated by hash / nonce and inserted in-order so the deque always reflects the most recent nonce progression we have seen from that creator.
- The deque is capped (`PRIOR_TX_HISTORY_LIMIT`) to avoid unbounded memory; oldest entries are dropped when the cap is reached.
- SimulationQueue only decides execution order; it does **not** own the buffer. Even if a lower-priority job is evicted when the queue is full, any processed creator transaction remains in the pending buffer until one of the pruning rules above removes it.
- A contract-creation transaction is retained only when it pairs with a follow-on trading-enablement helper in the same block. Otherwise the Python token-tracking service ingests the deployed token on the next block, so the buffer drops the creation artifact once the canonical head advances.
- Planned enhancement: hook the canonical-head listener so that when a new block lands we can (a) rebuild the in-memory sequence from on-chain data if required and (b) reseed prior helper transactions for newly tracked tokens. This keeps the buffer aligned with the canonical state without waiting for limit-based eviction.

Until the block-aware pruning is in place, operators should note that the sequence buffer only tracks the last few creator actions per token. If a creator floods the mempool with more than the configured limit before we see a block, older entries will be discarded; the subsequent replay will still succeed because we execute the current transaction against the canonical nonce, but early setup actions may need to be refetched from chain if they become relevant again.

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
simulation_manager.submit(TxSimulationJob {
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
- Creator launches often execute *multiple* helper calls (contract creation → openTrading → router call) in the same block. When we only replay an individual helper, the sandbox misses the earlier state changes.  
  The simulation manager maintains a short per-creator/token history of processed transactions and injects them as the `prior_txs` sequence for each pool probe. This keeps the queue bounded while ensuring launch helpers are faithfully reproduced.

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
        let request = TxSimulationJob {
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
