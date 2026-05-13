# Mempool Processor - Implementation Details

## System Overview

The mempool processor is a high-performance system designed to:
1. **Receive transactions** from Ethereum mempool via IPC
2. **Detect function signatures** in transaction calldata
3. **Simulate transactions** to analyze state changes
4. **Identify signals** (liquidity removals, trading enabled, scams, tax detection)
5. **Process as fast as possible** (currently 2-7μs detection latency)

## Current Module Structure
## Architecture Flow

```
1. IPC Client (MempoolFetcherIPCClient)
   ↓ Raw bytes
2. Transaction Parser
   ↓ Parsed transaction
3. Function Detector + TX Router
   ↓ Categorized transaction
4. Simulation Manager
   ↓ Simulation results (buy/sell, state changes)
5. Signal Manager
   ↓ Detected signals (tax, trading, liquidity, scam)
6. Signal Publisher
   ↓ ZMQ + Log files
7. DB Writers (optional)
```

### Core Processing Pipeline
```
src/
├── bin/
│   └── mempool_signal_detector.rs      # Main service binary
├── mempool_fetcher/                    # Transaction ingestion
│   └── mempool_fetcher_ipc_client.rs   # Reth IPC client (non-blocking)
├── function_detector/                  # Function signature detection
│   └── function_detector.rs            # 4-byte selector matching
├── tx_router/                         # Transaction categorization
│   ├── tx_router.rs                   # Main routing logic
│   └── creator_tx_router.rs           # Token creator detection
├── simulator/                         # Transaction simulation
│   ├── simulation_manager.rs          # Per-pool simulation + dispatch
│   ├── pool_buy_sell_simulator.rs     # Buy/sell tax simulation wrapper
│   └── mempool_simulator.rs           # Unified mempool + pool sim (shared TxSimulator)
├── signal_detector/                   # Signal generation (ACTIVE)
│   ├── signal_manager.rs              # Central signal coordinator
│   ├── tax_detector.rs                # Tax bucket risk detection
│   ├── trading_status_detector.rs     # Trading enabled detection  
│   ├── liquidity_detector.rs          # Pool drain detection
│   ├── lp_approval_detector.rs        # LP approval tracking
│   └── types.rs                       # Signal type definitions
├── token_tracking/                    # Token state management (ACTIVE)
│   ├── cache.rs                       # Token/pool caches
│   ├── address_tracking_cache.rs      # Creator address tracking
│   └── token_parameter_extraction/    # Tax calculation functions
│       ├── tax_calculator.rs          # Buy/sell tax functions (USED)
│       └── tax_calculator_from_...    # Alternate approach (unused)
├── signal_publisher.rs                # Log & ZMQ publishing (ACTIVE)
├── db_writers/                       # Database persistence
│   ├── trading_signal_writer.rs       # Trading enabled signals
│   ├── tax_signal_writer.rs           # Tax signals
│   ├── scam_signal_writer.rs          # Honeypot/sell-blocked signals
│   └── liquidity_removal_...rs        # Liquidity signals
└── config.rs                         # Configuration types
```


## Key Components

### 1. Signal Detection Pipeline (ACTIVE)

**SignalManager** (`signal_detector/signal_manager.rs`):
- Central coordinator for all signal types
- Processes simulation results from each pool
- Applies trading status filtering for TAX_SIGNAL
- Generates per-pool signals for (token_address, pool_address) pairs
- Implements contract creation noise reduction

**TaxDetector** (`signal_detector/tax_detector.rs`):
- Calculates buy/sell taxes from state changes
- Uses `token_tracking::calculate_buy_tax()` and `calculate_sell_tax()`
- Detects high taxes and honeypot patterns
- Returns -1% when calculation fails (not 0%)

**TradingStatusDetector** (`signal_detector/trading_status_detector.rs`):
- Detects trading enabled signals when taxes are reasonable (<25%)
- Does not write successful simulation rows; success is tracked by interval metrics
- Checks token cache for existing trading status

### 2. Token Tracking System (ACTIVE)

**TokenTrackingCache** (`token_tracking/cache.rs`):
- Maintains token metadata and trading status
- Used by signal manager for trading status filtering
- Hydrates from `eth_token_server` `/live/tokens` and `/live/pools`

**Tax Calculation** (`token_tracking/token_parameter_extraction/tax_calculator.rs`):
- Core functions: `calculate_buy_tax()`, `calculate_sell_tax()`
- Analyzes token movements in state changes
- Used by tax_detector.rs for accurate tax computation

### 3. Logging & Publishing

**SignalPublisher** (`signal_publisher.rs`):
- Writes to separate log files per signal type:
  - `trading_enabled.log` - TRADING_ENABLED entries only
  - `honeypot_signals.log` - HONEYPOT_SIGNAL entries
  - `tax_signals.log` - TAX_SIGNAL tax bucket risk entries
  - `liquidity_removals.log` - Pool drain signals
  - Scam alerts are included in `liquidity_removals.log`
- ZMQ multipart publishing to tcp://127.0.0.1:5556
- Database writing via db_writers/; required for live runs because token-server
  and ASENA read persisted signals

**Simulation diagnostics**:
- `simulation_errors.log` at the run root is the focused artifact for failed
  simulations and buy/sell branch errors.
- Successful simulations are counted in interval stats and not logged
  one-by-one.
- The `signals/` directory is semantic only; `tax_signals.log` contains actual
  tax risk signals, not routine tax calculations.

## Critical Implementation Notes

### Tax Detection Algorithm
```rust
// Tax calculation pipeline:
1. Simulate buy transaction → state changes
2. Calculate tokens received vs expected (calculate_buy_tax)
3. Simulate sell transaction → state changes  
4. Calculate ETH received vs expected (calculate_sell_tax)
5. Generate HONEYPOT_SIGNAL only if can_buy=true, can_approve=true, can_sell=false
6. Generate TAX_SIGNAL only for tax bucket/threshold risks:
   - trading_enabled = true, OR
   - trading_enabled = false AND (can_buy OR can_sell)
```

### Signal Generation Rules
- **Per-pool signals**: Each (token, pool) pair generates independent signals
- **Trading status filtering**: Uses token cache to reduce honeypot noise
- **Tax thresholds**: High/extreme tax buckets or configured tax thresholds = `tax_signal`; buy succeeds and sell fails = `honeypot_signal`
- **Simulation fallback**: 0.01 ETH buy amount, fallback to 0.001 ETH on failure

### Performance Optimizations
- Non-blocking IPC with connection recovery
- Batch simulation (20 transactions per batch)
- Token cache for trading status lookups
- ZMQ publishing with configurable buffer sizes
- Direct tax calculation from state changes (no RPC calls)

## Configuration

### Essential Environment Variables
```bash
# IPC connection (required)
RETH_IPC_PATH=/home/nima/storage/samsung8tb/ethereum/reth/reth.ipc

# Simulation RPC (required)  
RETH_HTTP_RPC=http://127.0.0.1:8545

# Database (optional)
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db
```

### Key Configuration Values
- **Simulation amounts**: 0.01 ETH primary, 0.001 ETH fallback
- **Tax thresholds**: 25% for buy/sell tax warnings
- **Channel buffer**: 50,000 transactions
- **Signal ZMQ endpoint**: tcp://127.0.0.1:5556


## Notes for Future Development

- Simulation manager is the bottleneck, not signal detection
- Pool state synchronization with token-server HTTP is critical
- Transaction ordering matters for nonce handling
- V2/Sushi and V3 pool support are active where the token cache has enough
  metadata; V4 removal intent is surfaced as unknown-severity risk until V4
  buy/sell simulation is validated.
- The 50K channel buffer size works well in practice
- Focus on reliability over micro-optimizations
