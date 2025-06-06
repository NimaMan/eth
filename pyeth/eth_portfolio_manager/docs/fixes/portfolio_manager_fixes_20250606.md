# eth_portfolio_manager Fixes - June 6, 2025

## Summary
Fixed two critical issues in the eth_portfolio_manager module that were causing errors during portfolio position tracking and strategy execution.

## Issue 1: BacktestStrategyEngine Missing strategy_name Attribute

### Problem
The `BacktestStrategyEngine` class was missing a `strategy_name` attribute that was being accessed by `StrategyPositionManager` at line 72, causing an AttributeError.

### Root Cause
- The `BaseStrategy` class has a `strategy_name` property
- `BacktestStrategyEngine` wraps a strategy instance but didn't expose this property
- `StrategyPositionManager` expected the engine to have this property for logging

### Fix
Added a `strategy_name` property to `BacktestStrategyEngine` that delegates to the wrapped strategy:

```python
@property
def strategy_name(self):
    return self.investment_strategy.strategy_name
```

**File Modified**: `/home/nima/code/crypto/py/eth_portfolio_manager/eth_portfolio_manager/backtesting/backtest_strategy_engine.py`

## Issue 2: KeyError When Accessing Pool Addresses

### Problem
`TokenPosition.update_from_token_data()` was throwing KeyError when:
1. Accessing empty pool_addresses tuple with index `[0]`
2. Looking up pool info for addresses that don't exist in the pool_info dictionary

### Root Cause
The code assumed:
- There's always at least one pool address when `trading_enabled_block` is set
- Pool info always exists for every pool address
- These assumptions fail for newly created tokens or tokens without pools

### Fixes Applied

1. **Safe pool address access** (lines 183-193):
   ```python
   # Safely handle pool addresses and pool info
   pool_addresses = live_token.token_data.pool_addresses
   if pool_addresses:
       self.static_data.pool_address = pool_addresses[0]
       
       # Check if pool info exists for this address
       if self.static_data.pool_address in live_token.token_data.pool_info:
           pool_info = live_token.token_data.pool_info[self.static_data.pool_address]
           self.static_data.pool_type = pool_info.get('pool_type')
           self.static_data.currency = pool_info.get('denom_currency')
   ```

2. **Safe price ratio lookup** (lines 195-197):
   ```python
   current_price_ratio = 0
   if self.static_data.pool_address:
       current_price_ratio = live_token.token_data.latest_pools_price_ratio.get(self.static_data.pool_address, 0)
   ```

3. **Safe reserve lookup** (line 211):
   ```python
   reserve=live_token.token_data.get_pool_reserve(self.static_data.pool_address) if self.static_data.pool_address else 0,
   ```

4. **Similar fixes in BacktestStrategyEngine** for all price ratio lookups

**Files Modified**: 
- `/home/nima/code/crypto/py/eth_portfolio_manager/eth_portfolio_manager/core/token_position.py`
- `/home/nima/code/crypto/py/eth_portfolio_manager/eth_portfolio_manager/backtesting/backtest_strategy_engine.py`

## Testing
Created comprehensive tests to verify:
1. BacktestStrategyEngine correctly exposes strategy_name
2. TokenPosition handles tokens without pools gracefully
3. TokenPosition correctly processes tokens with valid pool data

All tests pass successfully.

## Impact
These fixes ensure:
- Strategy engines can be properly identified in logs
- New tokens without pools don't crash the system
- Pool data is safely accessed with proper validation
- The portfolio manager can handle all token states gracefully