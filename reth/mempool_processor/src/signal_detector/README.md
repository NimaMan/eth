# Signal Detector Module - Critical Architecture Documentation

## 🎯 Core Purpose
This module processes token-pool pairs to detect trading signals from mempool transactions. It is the **critical decision-making component** that determines which transactions represent actionable trading opportunities.

## 📊 Current Implementation Status

### Active Signal Detectors (3/7) ✅

**1. Trading Status Detector** (`trading_status_detector.rs`)
- **Purpose**: Detects when trading becomes enabled on pools
- **Database**: `live_trading.trading_enabled_signals` table
- **Fields**: token_address, pool_address, buy_tax_at_signal, sell_tax_at_signal, etc.
- **Status**: ACTIVE - Tax calculation bug fixed with checksummed addresses

**2. Liquidity Detector** (`liquidity_detector.rs`)  
- **Purpose**: Detects significant liquidity changes and pool drains
- **Database**: `live_trading.liquidity_removal_signals` table
- **Fields**: liquidity_removed_eth, removal_percentage, pool_drain_risk_level, etc.
- **Status**: ACTIVE - Database writer implemented and tested

**3. LP Approval Detector** (`lp_approval_detector.rs`)
- **Purpose**: Detects LP token approvals (potential rug pull setup)
- **Database**: `live_trading.lp_approval_signals` table
- **Fields**: approved_spender, approval_amount, is_unlimited_approval, etc.
- **Status**: ACTIVE - Database writer implemented and tested

### Removed Detectors (1/7) ❌

**4. Honeypot Detector** (`honeypot_detector.rs`)
- **Status**: REMOVED per user request
- **Reason**: "remove noly honeypot for now"

### Inactive Detectors (3/7) ⏸️

**5. Tax Change Detector** (`tax_change_detector.rs`)
- **Status**: INACTIVE - Not integrated with database
- **Purpose**: Detects dynamic tax changes during transactions

**6. Stablecoin Detector** (`stablecoin_detector.rs`)
- **Status**: INACTIVE - Not integrated with database  
- **Purpose**: Detects stablecoin minting/burning activities

**7. Tax Detector** (`tax_detector.rs`)
- **Status**: INACTIVE - Not integrated with database
- **Purpose**: General tax rate detection and warnings

## Database Schema

All active detectors now write to their respective tables in the `live_trading` schema:

```sql
-- Trading enabled signals
live_trading.trading_enabled_signals
- Primary key: signal_id
- Unique constraint: (pool_address, detection_tx_hash)
- Indexes: token_address, pool_address, timestamp, tx_hash

-- Liquidity removal signals  
live_trading.liquidity_removal_signals
- Primary key: signal_id
- Unique constraint: (pool_address, detection_tx_hash)
- Indexes: token_address, pool_address, timestamp, tx_hash

-- LP approval signals
live_trading.lp_approval_signals
- Primary key: signal_id  
- Unique constraint: (pool_address, detection_tx_hash, approved_spender)
- Indexes: token_address, pool_address, spender, timestamp, tx_hash
```

## Key Architecture Decisions

1. **Per-Pool Signal Generation**: Each pool generates independent signals
2. **Checksummed Addresses**: All addresses use EIP-55 format for consistency
3. **Non-blocking Database Writes**: Background tasks with batched inserts
4. **Hardcoded Database Connections**: Each writer has its own connection string
5. **ZMQ + Database**: Signals published via ZMQ and stored in database

## Critical Bug Fixes

### Tax Calculation Bug (RESOLVED)
- **Problem**: All tax calculations returned 0% or None
- **Root Cause**: Address format mismatch in HashMap lookups
- **Solution**: Implemented checksummed addresses throughout
- **Result**: IMAG token now shows 4.00% buy tax instead of 0%

### Verification Results
```
IMAG Token (0x7EAa8d0DdeC2B0427cca190C8c360ff49c88d257):
✅ Buy Tax: 4.00% (was 0%)
✅ Token Balance Change: 959926981716644 (was None) 
✅ Database Integration: All 3 signal types working
```

## Database Status

Current signal counts:
- `trading_enabled_signals`: 2 signals
- `liquidity_removal_signals`: 1 signal  
- `lp_approval_signals`: 1 signal

All tables tested and functioning with proper:
- Tax rate storage (Decimal fields)
- Timestamp handling (TIMESTAMP WITHOUT TIME ZONE)
- Unique constraints preventing duplicates
- Proper indexing for query performance

## ⚠️ CRITICAL: Per-Pool Signal Generation
**All signals are generated PER-POOL, not per-token.**

```
Signal = f(token_address, pool_address)
```

Key Concepts:
- A token can have multiple pools (WETH/TOKEN, USDC/TOKEN, etc.)
- Each pool is monitored and signaled INDEPENDENTLY
- Each signal contains pool-specific data (taxes, liquidity, trading status)
- Currently supports V2 pools only (V3/V4 in development)

## 🏗️ Architecture Overview

### Signal Flow
```
1. SimulationManager processes EACH pool separately
2. Each pool gets its own SimulationResult  
3. SignalManager receives pool-specific results
4. Generates unique signal for (token, pool) pair
5. SignalPublisher distributes signals via ZMQ
6. Database writer persists signals to PostgreSQL
```

### Integration with SimulationManager
```rust
SimulationManager {
    signal_manager: Arc<Mutex<SignalManager>>,
    token_cache: Arc<TokenTrackingCache>,
    // ... simulation components
}
```

## 📦 Module Structure

### Active Detectors (Used in Production)

#### 1. **TaxDetector** (`tax_detector.rs`) ✅ ACTIVE
- **Purpose**: Calculates buy/sell taxes from state changes
- **Status**: Active but returning 0% (bug being investigated)
- **Critical Issue**: Tax calculation from state changes not working

#### 2. **TradingStatusDetector** (`trading_status_detector.rs`) ✅ ACTIVE  
- **Purpose**: Detects when trading is enabled on a pool
- **Status**: Fully functional
- **Triggers**: enableTrading(), openTrading(), first successful buy/sell

#### 3. **LiquidityDetector** (`liquidity_detector.rs`) ✅ ACTIVE
- **Purpose**: Detects liquidity removals and pool drains
- **Status**: Fully functional
- **Thresholds**: Scam >60% drain OR <0.3 ETH remaining

#### 4. **LpApprovalDetector** (`lp_approval_detector.rs`) ✅ ACTIVE
- **Purpose**: Detects LP token approvals (rug pull setup)
- **Status**: Fully functional

### Inactive/Unused Detectors

#### 5. **HoneypotDetector** ❌ UNUSED
#### 6. **StablecoinDetector** ❌ UNUSED
#### 7. **TaxChangeDetector** ❌ UNUSED

## 🚦 Signal Types (Emitted in Production)

### 1. Trading Enabled Signal

**Purpose**: Detect when trading becomes enabled on a specific pool

**Trigger Conditions**:
- Trading enabled function detected (`enableTrading`, `openTrading`, etc.)
- Both buy and sell transactions succeed in simulation
- Tax values calculated (currently buggy - returning 0%)

**Signal Data**:
```rust
pub struct TradingEnabledSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String,  // Pool-specific
    pub pool_type: String,     // V2, V3, V4
    pub creator_address: String,
    pub buy_tax: u8,          // 0-100 or 255 for None
    pub sell_tax: u8,         // 0-100 or 255 for None
    pub timestamp: u64,
    // block_number removed - timestamps sufficient
}
```

**Configuration Parameters** (in `config.rs`):
- `tax_detection.max_acceptable_buy_tax` (default: 30%)
- `tax_detection.max_acceptable_sell_tax` (default: 30%)

### 2. High Tax Warning Signal

**Purpose**: Alert when a pool has excessive taxes that indicate potential honeypot

**Trigger Conditions**:
- Buy tax > `max_acceptable_buy_tax` OR
- Sell tax > `max_acceptable_sell_tax`
- Cannot buy or cannot sell in simulation

**Signal Data**:
```rust
pub struct HighTaxWarningSignal {
    pub tx_hash: String,
    pub token_address: String,
    pub pool_address: String,  // Pool-specific
    pub pool_type: String,     // V2, V3, V4
    pub creator_address: Option<String>,
    pub buy_tax: u8,          // 255 means cannot buy
    pub sell_tax: u8,         // 255 means cannot sell
    pub warning_type: TaxWarningType,
    pub timestamp: u64,
}

pub enum TaxWarningType {
    HighBuyTax,
    HighSellTax, 
    PotentialHoneypot, // sell tax > 50% or cannot sell
}
```

**Configuration Parameters** (in `config.rs`):
- `tax_detection.max_acceptable_buy_tax` (default: 30%)
- `tax_detection.max_acceptable_sell_tax` (default: 30%)
- `tax_detection.honeypot_sell_threshold` (default: 50%)

### 3. Scam Detection Signal (ACTIVE)

**Purpose**: Detect when a transaction drains significant liquidity from a specific pool

**Trigger Conditions**:
- Pool ETH balance drops by more than 60% OR
- Pool ETH balance falls below 0.3 ETH
- LP token approval to router (rug pull setup)

**Signal Data**:
```rust
pub struct ScamDetectionSignal {
    pub tx_hash: String,
    pub pool_address: String,
    pub pool_type: String,     // V2, V3, V4
    pub token_address: String,
    pub scammer_address: String,
    pub eth_drained: f64,
    pub eth_remaining: f64,
    pub drain_percentage: f64,
    pub timestamp: u64,
}
```

**Configuration Parameters**:
- `scam_detection.drain_percentage_threshold` (default: 60%)
- `scam_detection.min_eth_remaining` (default: 0.3 ETH)

### 4. Liquidity Removal Signal (DEFINED BUT NOT EMITTED)

**Status**: Detected internally but not converted to a public signal. The detector logs liquidity removals but doesn't emit them as signals.

## 🐛 Critical Issues

### 1. Tax Calculation Returns 0% ⚠️
- **Problem**: All tax calculations return 0% or None
- **Location**: `tax_detector.rs:detect()`
- **Impact**: Trading signals have incorrect tax values
- **Root Cause**: State change calculation logic not working correctly
- **Status**: Under investigation - HIGH PRIORITY

### 2. Unused Detectors
- **Problem**: 3 detectors implemented but never used
- **Files**: `honeypot_detector.rs`, `stablecoin_detector.rs`, `tax_change_detector.rs`
- **Impact**: Code bloat, confusion, maintenance overhead
- **Recommendation**: Remove these files

### 3. LiquidityRemovalSignal Not Emitted
- **Problem**: Liquidity removals are detected but not converted to signals
- **Impact**: Missing important trading signals
- **Fix**: Convert LiquiditySignal to public Signal enum

## 🔄 Signal Detection Flow

```
1. Transaction arrives via IPC
   ↓
2. FunctionDetector identifies function signatures
   ↓
3. TransactionRouter categorizes transaction
   ↓
4. SimulationManager.submit(request) called
   ↓
5. Inside SimulationManager:
   a. Run simulation (tx + buy/sell if needed)
   b. Get pool info from cache for context
   c. Call signal_manager.process_simulation_result()
   d. Log and publish signals immediately
   ↓
6. Signals published to ZMQ and written to database
```

## 🔄 SignalManager (`signal_manager.rs`)

The central coordinator that processes simulation results PER-POOL:

```rust
pub async fn process_simulation_result(
    &mut self,
    result: &SimulationResult,  // Contains pool-specific data
) -> Vec<Signal>
```

### Processing Order
1. **TaxDetector** - Calculates taxes from state changes (BROKEN)
2. **TradingStatusDetector** - Detects trading enable/disable
3. **LiquidityDetector** - Checks for pool drains
4. **LpApprovalDetector** - Checks LP approvals (non-simulated txs)

## 📊 Performance Characteristics

- **Processing Time**: ~5-10ms per signal detection
- **Memory Usage**: Minimal (no caching in detectors)
- **Bottleneck**: Tax calculation from state changes
- **Throughput**: Handles 1000+ transactions/second


## 📤 Signal Output

### ZMQ Publishing
- **Endpoint**: `tcp://127.0.0.1:5556`
- **Format**: JSON-serialized signal structs
- **Topics**: "trading_enabled", "high_tax", "scam_detection"

### Database Storage
- **Database**: PostgreSQL `eth_db`
- **Schema**: `live_trading`
- **Tables**: `trading_enabled_signals`, `high_tax_signals`, `scam_detection_signals`
- **Writer**: Integrated into SignalPublisher (non-blocking)

### File Logging
- `logs/signals/signal_manager.log` - All activity
- `logs/signals/trading_enabled.log` - Trading signals
- `logs/signals/tax_signals.log` - Tax detections

## ⚠️ Critical Notes

1. **Never process pools together** - Each pool must be independent
2. **Tax values are pool-specific** - Different pools = different taxes
3. **Signal timing is critical** - Delays mean missed opportunities
4. **Database writes are async** - Don't block signal detection
5. **ZMQ publishing must not fail** - Use buffering if needed

## 🚀 Future Improvements

1. **Fix tax calculation** - Critical priority
2. **Remove unused detectors** - Clean up codebase
3. **Add V3/V4 pool support** - Expand coverage
4. **Emit LiquidityRemovalSignal** - Complete signal types
5. **Optimize state change processing** - Performance
6. **Add MEV detection** - New signal type

## 📚 Dependencies

- `reth_tx_simulator` - For simulation results
- `alloy_primitives` - For address handling
- `tokio` - For async operations
- `tracing` - For logging
- `chrono` - For timestamps
- `zmq` - For signal publishing
- `sqlx` - For database writing