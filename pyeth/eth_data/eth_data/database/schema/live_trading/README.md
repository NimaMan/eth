# Live Trading Database Schema

## Overview

This database schema is designed specifically for live cryptocurrency trading operations on Ethereum. It tracks real-time positions, trading signals, execution history, and performance metrics for automated trading strategies.

## Database Name: `live_trading_db`

## Core Objectives

1. **Position Tracking**: Real-time tracking of all open and closed positions
2. **Signal Management**: Record all trading signals sent to eth_kartal for execution
3. **Execution History**: Complete audit trail of all trade executions
4. **Performance Metrics**: Track P&L, ROI, and other performance indicators
5. **Risk Management**: Monitor exposure and risk metrics per wallet
6. **Scam Detection**: Integration with mempool scam predictions

## Schema Design Principles

1. **Separation of Concerns**: Trading data is isolated from blockchain analytics (eth_db)
2. **Real-time Focus**: Optimized for low-latency updates and queries
3. **Audit Trail**: Complete history of all trading decisions and executions
4. **Multi-Pool Support**: Handles Uniswap V2, V3, and V4 pools
5. **Multi-Wallet**: Supports multiple trading wallets

## Table Structure

### 1. `tokens` - Token Registry
Local copy of relevant token data for trading operations.
- Basic token information
- Scam status synchronization
- Trading enabled status
- Links to creation details

### 2. `wallets` - Trading Wallet Management
Tracks all wallets used for trading.
- Wallet addresses
- Strategy assignments
- Performance metrics
- Risk limits

### 3. `live_positions` - Active Position Tracking
Real-time position state for each wallet-token-pool combination.
- Current position state (INIT, BUY_SUBMITTED, etc.)
- Entry/exit prices and timestamps
- Quantity and value tracking
- P&L calculations
- ROI metrics

### 4. `trade_signals` - Signal History
All trading signals sent to eth_kartal.
- Signal parameters (action, amount, slippage)
- Target pool and token
- Strategy identification
- Execution status tracking
- Error handling

### 5. `executions` - Execution Details
Detailed execution records from eth_kartal.
- Transaction hashes
- Gas costs
- Actual vs expected amounts
- Slippage realized
- MEV protection status

### 6. `position_snapshots` - Historical Performance
Periodic snapshots of position states.
- Point-in-time position values
- Pool reserve states
- Performance metrics
- Risk metrics

### 7. `mempool_scam_predictions` - Risk Monitoring
Scam predictions from mempool analysis.
- Token/pool combinations
- ETH level thresholds
- Prediction confidence
- Alert triggers

### 8. `strategy_configs` - Strategy Parameters
Configuration for each trading strategy.
- Strategy names and versions
- Parameter sets
- Risk limits
- Wallet assignments

### 9. `performance_metrics` - Aggregated Performance
Daily/hourly performance aggregations.
- Wallet-level P&L
- Strategy performance
- Win rates
- Risk metrics

## Key Features

### Position State Machine
```
INIT → BUY_SUBMITTED → BUY_PENDING → BUY_CONFIRMED → SELL_SUBMITTED → SELL_PENDING → SELL_CONFIRMED
         ↓                ↓                               ↓                ↓
      FAILED           FAILED                          FAILED           FAILED
```

### Pool Identification
- **V2/V3**: Use `pool_address` (standard contract address)
- **V4**: Use `pool_id` (bytes32 identifier) since V4 pools don't have addresses
- `pool_type` field distinguishes between protocols

### Signal Tracking
- Unique `signal_id` (UUID) for each trading signal
- Links signals to executions and position updates
- Complete parameter history for debugging

### Performance Tracking
- Real-time P&L calculations
- ROI tracking per position
- Gas cost accounting
- Slippage analysis

## Query Patterns

### Common Queries
1. **Active Positions**: All open positions for a wallet
2. **Pending Signals**: Unconfirmed trading signals
3. **Performance Today**: P&L for current day
4. **Risk Exposure**: Total value at risk per wallet
5. **Failed Trades**: Recent execution failures

### Performance Indexes
- `(wallet_address, token_address, pool_address)` - Position lookups
- `(signal_id)` - Signal tracking
- `(tx_hash)` - Execution verification
- `(timestamp)` - Time-based queries
- `(position_state, wallet_address)` - State filtering

## Integration Points

1. **eth_kartal**: Receives signals, sends execution confirmations
2. **Live Token Tracker**: Updates position states
3. **Strategy Engine**: Generates trading signals
4. **Risk Monitor**: Checks exposure limits
5. **Performance Dashboard**: Displays real-time metrics

## Data Retention

- **Positions**: Keep forever (historical analysis)
- **Signals**: Keep for 90 days (debugging)
- **Executions**: Keep forever (audit trail)
- **Snapshots**: Daily for 30 days, then weekly
- **Metrics**: Hourly for 7 days, daily for 1 year

## Security Considerations

1. **Wallet Addresses**: Never store private keys
2. **Signal Integrity**: UUID prevents replay attacks
3. **Access Control**: Read-only for most services
4. **Audit Trail**: Immutable execution history
5. **Data Encryption**: Sensitive fields encrypted at rest