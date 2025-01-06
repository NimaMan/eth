
# Portfolio Manager Architecture
**Objective**: Design a Portfolio Manager architecture that efficiently manages token investments based on real-time token updates while maintaining separation of concerns from the token monitoring system.

## Overview
The Portfolio Manager is responsible for making investment decisions based on token updates and managing the portfolio composition. It works alongside the Token Manager but maintains loose coupling through event-driven communication.

## Core Responsibilities
1. Portfolio State Management
   - Track current token holdings
   - Monitor portfolio value
   - Track position sizes
   - Track investment performance

2. Investment Decision Making
   - Process token updates from Token Manager
   - Apply investment strategies
   - Generate buy/sell signals
   - Manage risk parameters

3. Transaction Management
   - Queue trading decisions
   - Interface with Transaction Manager
   - Track order status
   - Handle transaction failures


## Architecture Components

### 1. PortfolioManager
Main orchestrator that:
- Initializes and manages the Token Manager as a subtask
- Maintains portfolio state
- Coordinates between components
- Handles shutdown sequence

### 2. PortfolioState
Maintains:
- Token positions
- Performance metrics


### 3. InvestmentStrategy
Processes:
- Token updates
- Generate trading signals

### 4. OrderManager
Handles:
- Order creation
- Transaction queuing
- Status tracking
- Interaction with Transaction Manager

## Data Flow
```ascii
                                                        ┌─────────────────┐
                                                        │  Transaction    │
                                                        │    Manager      │
                                                        └────────┬────────┘
                                                                 ▲
                                                                 │
┌─────────────────┐    ┌─────────────────┐    ┌────────────────┴────────────────┐
│   Token Manager │    │ Portfolio State │    │         Order Manager           │
│                 │    │                 │    │                                 │
│ (Token Updates) │───►│  (Positions)    │◄───│    (Transaction Processing)     │
└────────┬────────┘    └────────┬────────┘    └─────────────────┬──────────────┘
         │                      │                               ▲
         │                      │                               │
         ▼                      ▼                               │
┌────────────────────────────────────────────┐                  │
│              Portfolio Manager             │                  │
│                                            │                  │
│  - Coordinates components                  │                  │
│  - Manages token updates                   ├──────────────────┘
│  - Applies investment strategies           │
│  - Generates trading decisions             │
└────────────────────────────────────────────┘
```

## Implementation Strategy

### 1. Portfolio Manager Class
```python
class PortfolioManager:
    def __init__(self):
        self.token_manager = LiveTokenManager()
        self.portfolio_state = PortfolioState()
        self.investment_strategy = InvestmentStrategy()
        self.order_manager = OrderManager()
        
    async def process_token_updates(self, updated_tokens):
        # Generate trading decisions
        decisions = self.investment_strategy.analyze(updated_tokens)
        # Update portfolio state
        self.portfolio_state.update(decisions)
        # Queue transactions
        await self.order_manager.process_decisions(decisions)
```

### 2. Event Flow
1. Token Manager updates the token cache using the new block data
2. Updated tokens trigger
    -  portfolio state to be updated
    -  investment strategy to generate trading decisions
    -  trading decisions to be queued for execution


### 3. Integration Points
1. Token Manager Integration
   - Run as a subtask within Portfolio Manager
   - Subscribe to token updates
   - Access to LiveTokenObjectsCache

2. Transaction Manager Integration
   - Queue trading decisions
   - Receive transaction status updates
   - Handle execution results

## Key Design Considerations

1. **Loose Coupling**
   - Components communicate through well-defined interfaces
   - Event-driven architecture
   - Easy to modify individual components

2. **State Management**
   - Clear separation between token state and portfolio state
   - Consistent state updates
   - Recovery mechanisms

3. **Performance**
   - Efficient processing of token updates
   - Optimized decision making
   - Quick transaction queuing

4. **Monitoring**
   - Portfolio performance tracking
   - Risk metrics
   - Transaction success rates


# Data Flow
Flow Diagram: Token Detection to Portfolio State
---------------------------------------------


1. Token Detection (LiveTokenManager)
   ├── Processes new block
   ├── Detects token events:
   │   ├── New token creation
   │   ├── Trading enabled
   │   ├── Liquidity added
   │   └── Price updates
   └── Generates alerts

2. Token Processing (LiveBlockTokenProcessor)
   ├── Receives block data
   ├── Updates token states:
   │   ├── updated_tokens dictionary
   │   └── alerts list
   └── Triggers block_processed_event
       └── Calls registered callbacks with block_number

3. Portfolio Manager (_handle_token_updates)
   ├── Receives block_number from callback
   ├── Gets updated_tokens from processor
   ├── For each token:
   │   ├── Logs token details
   │   └── Passes to investment strategy
   └── Gets trading signals

4. Investment Strategy (SimpleTokenBuyStrategy)
   ├── Receives updated_tokens and current_holdings
   ├── For each token:
   │   ├── Checks if new (not in processed_tokens)
   │   ├── Checks if tradeable:
   │   │   ├── trading_enabled
   │   │   ├── has_uni_v2_pair
   │   │   └── not is_scam
   │   └── Generates BUY signal if conditions met
   └── Returns list of signals

5. Portfolio State Manager
   ├── Receives trading signals
   ├── For each signal:
   │   ├── Creates/updates position
   │   ├── Writes position to Redis:
   │   │   ├── Key: position:{token_address}
   │   │   └── Value: JSON position data
   │   └── Writes to history:
   │       ├── Key: position:history:{token_address}
   │       └── Value: JSON history entry
   └── Updates portfolio metrics:
       ├── Calculates totals
       ├── Writes metrics to Redis:
       │   ├── Key: portfolio:metrics
       │   └── Value: JSON metrics data
       └── Writes to history:
           ├── Key: portfolio:metrics:history
           └── Value: JSON metrics entry

Possible Issues:
---------------
1. Event Chain Breaks:
   - Block processed but callback not triggered
   - Callback triggered but no token updates
   - Token updates exist but no signals generated
   - Signals generated but state not updated
   - State updated but Redis write fails

2. Data Flow Issues:
   - Token data missing required fields
   - Invalid token state transitions
   - Incorrect signal generation
   - State calculation errors
   - Redis connection/write failures

3. Timing Issues:
   - Events processed out of order
   - Race conditions in state updates
   - Missed blocks during catchup
   - Delayed Redis writes

Required Logging Points:
----------------------
1. Block Processing:
   - Block number
   - Number of transactions
   - Processing time
   - Number of token events

2. Token Updates:
   - Token addresses
   - State changes
   - Alert generation
   - Validation results

3. Trading Signals:
   - Signal generation attempts
   - Decision criteria
   - Signal details
   - Processing results

4. State Updates:
   - Position changes
   - Metrics calculations
   - Redis write attempts
   - Success/failure status

## Next Steps

1. Implement core PortfolioManager class
2. Define portfolio state management
3. Create investment strategy interface
4. Build order management system
5. Add monitoring and logging
6. Implement recovery mechanisms
