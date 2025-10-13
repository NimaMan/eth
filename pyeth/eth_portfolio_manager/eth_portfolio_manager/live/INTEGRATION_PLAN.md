# Live Position Tracking Integration Plan

## Current State Issues

### 🔍 **Analysis Results:**

1. **Pool Tracking: ✅ WORKING**
   - Pools are correctly discovered and tracked by PoolManager
   - Pool information flows to TokenPosition objects
   - Pool data is shared with Rust mempool processor

2. **Position Writing: ❌ INCORRECT DATABASE**
   - Live system uses `LiveResultsWriter` → writes to `backtest` database
   - Should use `LiveTradingPositionWriter` → writes to `live_trading_db`
   - Missing integration with live_trading_db.live_positions

3. **Strategy Engine: ❌ NOT INTEGRATED**
   - `LiveStrategyEngine` exists but isn't used
   - Live system uses `BacktestStrategyEngine` for real trading
   - No correlation between published signals and database positions

## Required Changes

### 1. **Update LiveTokenTracker to use LiveTradingPositionWriter**

```python
# Current (INCORRECT):
from eth_data.database.writers.live_token_position_results_writer import LiveResultsWriter

# Should be (CORRECT):
from eth_data.database.writers.live_trading_position_writer import LiveTradingPositionWriter
```

### 2. **Integrate LiveStrategyEngine with LiveTokenTracker**

The LiveTokenTracker should:
- Use LiveStrategyEngine instead of BacktestStrategyEngine for real trading
- Create positions in live_trading_db when signals are published
- Track signal IDs and correlate with positions

### 3. **Position Lifecycle Management**

```python
# When strategy decides to buy:
1. LiveStrategyEngine.update_submit_buy()
   - Publishes signal to eth_kartal via TradeSignalPublisher
   - Creates position in live_trading_db via LiveTradingPositionWriter
   - Sets state to BUY_SUBMITTED

# When eth_kartal confirms execution:
2. LiveStrategyEngine._handle_execution_confirmation()
   - Updates position state to BUY_CONFIRMED
   - Records transaction hash and execution details

# When strategy decides to sell:
3. LiveStrategyEngine.update_submit_sell()
   - Publishes sell signal to eth_kartal
   - Updates position state to SELL_SUBMITTED

# When sell is confirmed:
4. LiveStrategyEngine._handle_execution_confirmation()
   - Updates position state to SELL_CONFIRMED
   - Records realized profit and closes position
```

## Implementation Steps

### Step 1: Create Integrated Live Position Manager

```python
class LivePositionManager:
    """Manages live positions with proper database integration"""
    
    def __init__(self, live_trading_writer: LiveTradingPositionWriter,
                 signal_publisher: TradeSignalPublisher):
        self.position_writer = live_trading_writer
        self.signal_publisher = signal_publisher
        self.strategy_engine = None  # Set when strategy is assigned
    
    def process_token_update(self, token: ERC20Token, wallet_id: int):
        """Process token update and manage positions"""
        # 1. Get or create position
        position = self._get_or_create_position(token, wallet_id)
        
        # 2. Apply strategy
        signal = self.strategy_engine.analyze_token(token, position)
        
        # 3. Handle signal
        if signal:
            self._handle_trade_signal(signal, position, token)
    
    def _handle_trade_signal(self, signal: TradeSignal, position: dict, token: ERC20Token):
        """Handle trade signal with database integration"""
        if signal.decision == TradingDecision.SUBMIT_BUY:
            # Publish signal to eth_kartal
            signal_id = self.signal_publisher.publish_signal(signal, ...)
            
            # Create position in live_trading_db
            position_id = self.position_writer.create_position(
                wallet_id=position['wallet_id'],
                token_address=token.token_address,
                pool_address=position['pool_address'],
                strategy_name=self.strategy_engine.strategy_name,
                entry_signal_id=signal_id
            )
```

### Step 2: Update LiveTokenTracker Integration

```python
class LiveTokenTracker:
    def __init__(self, ...):
        # Replace LiveResultsWriter with LiveTradingPositionWriter
        self.position_writer = LiveTradingPositionWriter(self.logger)
        
        # Create position manager
        self.position_manager = LivePositionManager(
            self.position_writer,
            self.signal_publisher
        )
    
    def process_token_updates(self, ...):
        # For each strategy that uses real trading:
        if strategy_uses_live_trading:
            # Use LivePositionManager instead of BacktestStrategyEngine
            self.position_manager.process_token_update(token, wallet_id)
```

### Step 3: Signal-Position Correlation

```python
class SignalPositionCorrelator:
    """Correlates signals from eth_kartal with database positions"""
    
    def handle_execution_confirmation(self, confirmation: ExecutionConfirmation):
        """Update position based on execution confirmation"""
        # Find position by signal_id
        position = self.position_writer.get_position_by_signal_id(
            confirmation.signal_id
        )
        
        if confirmation.status == ExecutionStatus.CONFIRMED:
            # Update position state and execution details
            self.position_writer.update_position_state(
                position['id'], 
                'BUY_CONFIRMED',
                {
                    'entry_tx_hash': confirmation.tx_hash,
                    'entry_block': confirmation.block_number,
                    'quantity_tokens': confirmation.actual_amount_tokens,
                    'entry_price': confirmation.actual_price
                }
            )
```

## Database Flow Correction

### Current (INCORRECT):
```
LiveTokenTracker → LiveResultsWriter → backtest.token_positions
     ↓
LiveStrategyEngine → TradeSignalPublisher → eth_kartal
```

### Target (CORRECT):
```
LiveTokenTracker → LiveTradingPositionWriter → live_trading_db.live_positions
     ↓                                              ↑
LiveStrategyEngine → TradeSignalPublisher → eth_kartal
                              ↓
                    live_trading_db.trade_signals
```

## Benefits

1. **Correct Database Usage**: Live trading data goes to live_trading_db
2. **Signal Correlation**: Positions are linked to trade signals
3. **Real-time Tracking**: Position states reflect actual blockchain confirmations
4. **Pool Integration**: Pools are registered in eth_db.pools before position creation
5. **Audit Trail**: Complete history of signals, executions, and position changes