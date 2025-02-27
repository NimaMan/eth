
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

4. Investment Strategy
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
