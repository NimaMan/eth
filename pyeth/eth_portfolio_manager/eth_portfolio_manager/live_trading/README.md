# Live Trading Module

## Overview

The Live Trading module manages real-time token tracking, strategy execution, and pool liquidity monitoring for Ethereum tokens. It serves as the Python-side coordinator that processes blockchain data, generates trading signals, and shares critical pool information with the Rust mempool processor for scam detection.

## Architecture Status

**Current State**: The module is in a transitional phase with both legacy and new systems coexisting:
- **Legacy Path**: LiveTokenTracker → LiveResultsWriter → backtest database
- **New Path**: LiveTokenTracker → LiveTradingAdapter → LiveTradingCoordinator → live_trading_db

The LiveTradingAdapter serves as a temporary bridge component during migration.

## Core Components

### 1. LiveTokenTracker (`live_token_tracker.py`)
**Purpose**: Main orchestrator for real-time blockchain data processing and pool level tracking

**Key Responsibilities**:
- Processes blockchain data via LiveBlockTokenProcessor
- Manages strategy execution through LiveStrategyEngine
- Extracts and publishes pool ETH reserves to Rust via ZMQ (ports 5557/5558)
- Persists token and pool status to database via TokenStatusWriter
- Coordinates PnL tracking when enabled

**Status**: ✅ **Core Component** - Essential for the entire pipeline

### 2. LiveStrategyEngine (`live_strategy_engine.py`)
**Purpose**: Real-time trading signal generator with eth_kartal integration

**Key Responsibilities**:
- Generates trading signals based on strategy decisions
- Publishes signals to eth_kartal via ZMQ (port 5559)
- Manages position state transitions:
  - INIT → BUY_SUBMITTED → BUY_CONFIRMED
  - → SELL_SUBMITTED → SELL_CONFIRMED
- Tracks execution confirmations and real slippage

**Status**: ✅ **Core Component** - Critical for live trading execution

### 3. LiveTradingCoordinator (`live_trading_coordinator.py`)
**Purpose**: Central router for multi-wallet/multi-strategy trading

**Key Responsibilities**:
- Manages multiple LivePositionManager instances
- Routes token updates to appropriate strategies
- Tracks system-wide metrics and performance
- Handles graceful shutdown and resource cleanup

**Status**: ✅ **Core Component** - New preferred architecture

### 4. LivePositionManager (`live_position_manager.py`)
**Purpose**: Individual position lifecycle management

**Key Responsibilities**:
- Processes token updates through strategy engine
- Creates and updates database positions
- Tracks position metrics (price, profit, state)
- Handles execution confirmations

**Status**: ✅ **Core Component** - Essential for position tracking

### 5. LiveTradingAdapter (`live_trading_adapter.py`)
**Purpose**: Backward compatibility bridge

**Key Responsibilities**:
- Drop-in replacement for LiveResultsWriter
- Converts legacy format to new system
- Enables gradual migration

**Status**: 🔄 **Transitional** - Will be deprecated post-migration

## Data Flow

```
Ethereum Blockchain
        ↓
LiveBlockTokenProcessor (block & event processing)
        ↓
LiveTokenTracker (orchestration)
        ├─→ TokenTrackingCache (pool reserves)
        │       ↓
        │   TokenTrackingPublisher → Rust Mempool Processor
        │   (ZMQ 5557/5558)
        │
        ├─→ Strategy Processing
        │       ↓
        │   LiveStrategyEngine → eth_kartal
        │   (ZMQ 5559)
        │
        └─→ Database Persistence
                ↓
            TokenStatusWriter (tokens & pools)
            TokenPnLWriter (trade metrics)
```

## Code Quality Issues & Recommendations

### 1. Duplicate Code
**Issue**: Wallet address extraction logic duplicated in multiple files

**Current**:
- `live_trading_adapter.py`: `_extract_wallet_address()`
- `live_trading_coordinator.py`: `_extract_wallet_address()`
- Similar logic in `live_strategy_engine.py`

**Recommendation**: Create `wallet_utils.py`:
```python
def extract_wallet_address(strategy):
    """Extract wallet address from strategy config"""
    # Consolidated logic here
```

### 2. Redundant Database Operations
**Issue**: Wallet ID creation duplicated

**Current**:
- `live_trading_adapter.py`: `_get_or_create_wallet_id()`
- `live_trading_coordinator.py`: `_get_or_create_wallet_id()`

**Recommendation**: Move to `LiveTradingPositionWriter.get_or_create_wallet_id()`

### 3. Architecture Simplification
**Issue**: Parallel paths create complexity

**Recommendation**: 
1. Complete migration to new system
2. Remove LiveTradingAdapter
3. Simplify to: LiveTokenTracker → LiveTradingCoordinator → Database

## Integration Points

### Pool Level Publishing (Python → Rust)
- **Purpose**: Share real-time liquidity for scam detection
- **Publisher**: TokenTrackingPublisher
- **Ports**: 
  - 5557 (PUB): Real-time updates
  - 5558 (REP): Query interface
- **Data**: Pool reserves, token addresses, block numbers

### Trading Signals (Python → eth_kartal)
- **Purpose**: Execute trades on-chain
- **Publisher**: TradeSignalPublisher  
- **Port**: 5559
- **Data**: Buy/sell signals with amounts and pools

### Token Persistence (Python → PostgreSQL)
- **Purpose**: Track token metadata and status
- **Writers**:
  - TokenStatusWriter: Token/pool metadata
  - TokenPnLWriter: Trading metrics
- **Features**: Retry logic for race conditions

## Configuration

### Key Parameters
```python
# LiveTokenTracker
warmup_blocks = 20000       # Blocks before live trading
save_strategy_results = True # Enable database persistence  
add_pnl_to_db = True        # Track profit/loss
min_eth_threshold = 0.01    # Min pool liquidity

# ZMQ Endpoints (hardcoded)
POOL_PUB_ENDPOINT = "tcp://*:5557"
POOL_REP_ENDPOINT = "tcp://*:5558"
SIGNAL_PUB_ENDPOINT = "tcp://localhost:5559"
```

## Performance Optimizations

1. **Event-driven processing**: Queue-based token updates
2. **Parallel strategy execution**: Concurrent position management
3. **Retry mechanisms**: Handle database race conditions
4. **Silent retries**: Reduce log noise for transient errors
5. **Batch operations**: Where possible for database writes

## Monitoring & Debugging

### Key Metrics
- Active positions per wallet
- Database write latency
- Signal publishing rate
- Pool update frequency

### Health Checks
- Database connectivity
- ZMQ socket status
- Strategy responsiveness
- Token tracking cache size

## Future Improvements

1. **Code Consolidation**
   - Extract common utilities
   - Unify wallet management
   - Standardize error handling

2. **Architecture Cleanup**
   - Remove LiveTradingAdapter
   - Simplify data paths
   - Reduce coupling

3. **Performance**
   - Implement connection pooling
   - Add circuit breakers
   - Optimize database queries

## Summary

The live trading module is well-architected but shows signs of ongoing migration. Core components (LiveTokenTracker, LiveStrategyEngine, LiveTradingCoordinator, LivePositionManager) are production-ready and actively used. The main improvement opportunity is completing the migration to remove transitional components and consolidate duplicate utility functions.