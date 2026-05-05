# Portfolio Manager Architecture
**Objective**: A production-ready Ethereum portfolio management system that tracks token positions across multiple DEX pools, executes investment strategies in real-time, publishes token information to external systems, and maintains comprehensive position state in a PostgreSQL database.

## Overview
The Portfolio Manager is a comprehensive system for managing cryptocurrency trading positions across multiple tokens and liquidity pools. It provides:

- **Real-time position tracking** with state management
- **Multi-pool support** for tokens trading on multiple DEXs (Uniswap V2/V3/V4)
- **Strategy-based trading** with extensible strategy framework
- **Backtesting capabilities** for strategy validation
- **Database persistence** for historical analysis
- **Performance metrics** calculation and tracking
- **Scam protection** through integrated detection systems
- **Token information publishing** via ZeroMQ to external systems (e.g., Rust mempool processor)

## Core Responsibilities
1. **Multi-Pool Position Management**
   - Track positions across multiple DEX pools per token
   - Use composite keys `{token_address}-{pool_address}` for unique identification
   - Support Uniswap V2 (address-based), V3 (address-based), and V4 (poolId-based) pools
   - Maintain independent position state for each token-pool combination

2. **Real-Time Investment Decisions**
   - Process live token updates from Token Manager
   - Apply configurable investment strategies
   - Generate BUY/SELL/HOLD signals based on strategy logic
   - Track strategy performance and metrics

3. **State Persistence & Recovery**
   - Store all position data in PostgreSQL with JSONB
   - Track position lifecycle: PENDING → ACTIVE → CLOSED
   - Maintain historical snapshots for analysis
   - Support system recovery from database state

4. **Performance Optimization**
   - LRU cache for frequently accessed positions
   - Concurrent position updates with semaphore limits
   - Efficient database queries with proper indexing
   - Memory-efficient data structures

5. **Token Information Publishing**
   - Extract critical token and pool data from updates
   - Publish lightweight ZeroMQ PUB/SUB notifications on port 5557
   - Store full token state in Redis snapshots/index for startup recovery
   - Support high-frequency updates with minimal latency


## Live Processing Pipeline

The live portfolio manager implements a complete pipeline for processing blockchain data in real-time:

### Pipeline Overview
```
Ethereum Node → Block Processor → Token Processor → Portfolio Manager → Database & Publishers
```

### Detailed Pipeline Stages

1. **Block Reception & Processing**
   - `LiveBlockSnapshotSubscriber` connects to Ethereum node WebSocket
   - Receives new blocks in real-time
   - Extracts all transactions and logs

2. **Token Event Detection**
   - `LiveBlockTokenProcessor` identifies token-related events:
     - Pool creation (Uniswap V2/V3/V4)
     - Liquidity additions/removals
     - Swap transactions
     - Price updates
   - Creates/updates `LiveToken` objects with current state

3. **Token Update Queue**
   - Updates placed in `unprocessed_token_updates` priority queue
   - `new_updates_event` signals availability of new data
   - Enables asynchronous processing by multiple consumers

4. **Strategy Execution**
   - `LiveBacktestEngineWithPools` processes token updates:
     - Routes to configured strategies (e.g., MarketTracker, BuyAll)
     - `StrategyPositionManager` updates positions per strategy
     - Generates trading signals based on strategy logic

5. **Database Persistence**
   - Strategy positions saved to `token_positions` table
   - Historical snapshots stored for analysis
   - PnL calculations written asynchronously

6. **Token Information Publishing**
   - `TokenInfoExtractor` extracts pool reserves and token data
   - `TokenInfoPublisher` publishes via ZeroMQ:
     - PUB socket (5557): Real-time updates stream
   - Enables external systems (Rust mempool processor) to hydrate state from Redis

### Configuration Parameters
- `warmup_blocks`: Historical blocks to process before live (default: 10000)
- `save_strategy_results`: Persist strategy results to database (default: True)
- `add_pnl_to_db`: Write PnL calculations to database (default: True)
- `max_pools`: Maximum pools to track per publisher (default: 2000)
- `min_eth_threshold`: Minimum ETH reserve for pool tracking (default: 0.01)

## Architecture Components

### 1. TokenPosition (Core Data Model)
The central aggregate that combines:
- **TokenPositionStaticData**: Immutable metadata (token address, pool address, entry price)
- **TokenPositionDynamicSnapshot**: Time-series data (current price, volume, P&L)
- **Methods**: Update from live data, calculate metrics, serialize/deserialize

Key features:
- Aggregate pattern combining static and dynamic data
- Factory methods for creation from tokens or database
- Built-in serialization for persistence
- State transition management
- Performance metric calculations

### 2. StrategyPositionManager
Manages positions for a specific strategy:
- Processes token updates concurrently (default semaphore: 20)
- Maintains position cache with LRU eviction
- Handles position lifecycle transitions
- Integrates with database for persistence
- Multi-pool support per token
- Strategy engine integration

### 3. LivePortfolioManager
Orchestrates live trading operations:
- Initializes Token Manager with warmup blocks
- Routes updates to strategy managers
- Coordinates component lifecycle
- Handles graceful shutdown
- Event-driven architecture
- Multi-strategy support

### 4. BacktestExecutionEngine
Enables historical strategy testing:
- Replays historical blockchain data
- Simulates strategy execution
- Generates performance reports
- Validates strategy parameters

### 5. Investment Strategies
Pluggable strategy system:
- **Base Interface**: StandardInvestmentStrategy
- **Built-in Strategies**: BuyAll, BuyScam, MarketTracker
- **Custom Strategies**: Extend base class with custom logic
- **Signals**: BUY, SELL, HOLD with associated metadata

### 6. Publishers Module
Real-time data distribution:
- **TokenInfoExtractor**: Extracts token and pool state from updates
- **TokenInfoPublisher**: Publishes via ZeroMQ to external systems
- **Published Data**: Pool reserves, tax rates, trading status, limits
- **Communication**: PUB/SUB for live notifications, Redis for state queries/recovery

### 7. Database Layer
PostgreSQL with optimized schema:
- **strategy_runs**: Strategy execution metadata
- **token_positions**: Current position state with JSONB storage
- **token_position_snapshots**: Historical time-series
- **position_trades**: Executed trades
- **strategy_metrics**: Performance tracking

## Data Models

### TokenPositionStaticData
Immutable position metadata:
```python
@dataclass
class TokenPositionStaticData:
    token_address: str
    symbol: str
    currency: str
    pool_address: str
    pool_type: str  # V2, V3, V4
    creation_block: int
    creation_timestamp: int
    trading_enabled_block: int
    trading_enabled_timestamp: int
    purchase_value: float
    entry_price_ratio: float
    exit_price_ratio: float
    entry_block: int
    exit_block: int
    entry_timestamp: int
    exit_timestamp: int
    entry_tx_fee: float
    exit_tx_fee: float
```

### TokenPositionDynamicSnapshot
Time-series position data:
```python
@dataclass
class TokenPositionDynamicSnapshot:
    current_price_ratio: float
    reserve: float
    roi: float
    current_value: float
    realized_profit: float
    unrealized_profit: float
    quantity: float
    token_age_blocks: int
    token_age_hours: int
    block_number: int
    timestamp: int
    has_active_position: bool
    position_state: TokenPositionState
    scam_probability: Optional[float]
    scam_reason: Optional[str]
```

## Data Flow
```ascii
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Block Processor│    │   Token Manager  │    │ Mempool Monitor │
│                 │    │                  │    │ (Scam Detection)│
└────────┬────────┘    └────────┬─────────┘    └────────┬────────┘
         │                      │                        │
         ▼                      ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LiveBlockTokenProcessor                      │
│  - Processes blocks and extracts token updates                  │
│  - Identifies multi-pool tokens (V2/V3/V4)                     │
│  - Triggers update events                                       │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LivePortfolioManager                         │
│  - Routes updates to strategy managers                          │
│  - Coordinates multiple strategies                              │
│  - Manages component lifecycle                                  │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                 StrategyPositionManager                         │
│  - Processes tokens concurrently (with semaphore)               │
│  - Maintains position cache (LRU)                               │
│  - Creates positions for each token-pool pair                   │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Investment Strategy                          │
│  - Evaluates token based on strategy logic                      │
│  - Generates BUY/SELL/HOLD signals                             │
│  - Returns trading decisions                                    │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      TokenPosition                              │
│  - Updates position state (PENDING→ACTIVE→CLOSED)               │
│  - Calculates P&L and metrics                                   │
│  - Tracks per-pool performance                                  │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                          │
│  - Persists position state and history                          │
│  - Stores strategy metrics                                      │
│  - Enables recovery and analysis                                │
└─────────────────────────────────────────────────────────────────┘
```

## Event Sequence

### Core Queues & Events

1. **`LiveBlockTokenProcessor.unprocessed_token_updates`** (asyncio.PriorityQueue)
   - Content: `(priority, (block_number, updated_tokens_dict))`
   - Flow: LiveBlockTokenProcessor → LiveBacktestEngine
   - Purpose: Passes processed token updates from block processor to main engine

2. **`LiveBlockTokenProcessor.new_updates_event`** (asyncio.Event)
   - Flow: Set by LiveBlockTokenProcessor, waited on by LiveBacktestEngine
   - Purpose: Notifies main engine that new items are available

3. **`LiveBacktestEngine._pool_updates_queue`** (asyncio.Queue)
   - Content: `(block_number, updated_tokens_dict)`
   - Flow: Main processing task → Pool update task
   - Purpose: Passes token updates for pool level tracking

### Flow A: Confirmed Block Processing

1. **Block Arrival**: LiveBlockSnapshotSubscriber receives new block from Ethereum node
2. **Block Processing**: 
   - LiveBlockTokenProcessor.process_block_live() called
   - Parses all transactions and logs in the block
   - Updates LiveTokensCache with new token states
   - Identifies which tokens have changed
3. **Update Queueing**:
   - Creates tuple: (block_number, updated_tokens_dict)
   - Puts updates onto unprocessed_token_updates priority queue
   - Sets new_updates_event to notify waiting consumers
4. **Main Engine Processing** (LiveBacktestEngineWithPools):
   - Waits for new_updates_event signal
   - Gets updates from unprocessed_token_updates queue
   - For each updated token:
     - Routes to all configured strategies
     - Updates positions based on strategy signals
     - Extracts token/pool information
5. **Information Publishing**:
   - TokenInfoExtractor processes updated tokens
   - Extracts pool reserves, tax rates, trading status
   - TokenInfoPublisher sends updates via ZeroMQ
6. **Database Operations**:
   - Strategy results saved if configured
   - PnL calculations written asynchronously
   - Position snapshots stored for analysis

### Flow B: Mempool Monitoring (Concurrent)

1. **Periodic Check**: Monitor task wakes up based on poll_interval
2. **Read Confirmed State**: Gets current pool ETH levels
3. **Query Mempool**: Gets pending state diffs from mempool processor
4. **Simulate Impact**: Applies pending changes to confirmed levels
5. **Detect Suspicious**: Compares simulated vs confirmed levels
6. **Alert**: Logs warnings for potential scams

### Database Write Sequence

1. **Token Update in Memory**: LiveTokensCache updated during block processing
2. **Strategy Execution**: Positions updated based on strategy logic
3. **Pool Level Updates**: Internal ETH levels updated for monitoring
4. **Strategy Results Writing**: Updated positions written to database
5. **PnL Writing**: Token PnL data written after all critical operations

## Detailed Data Flow

The portfolio manager processes live blockchain data through a multi-stage pipeline that enables real-time position tracking, strategy execution, and information publishing to external systems.

### 1. Block Processing & Token Detection
```
New Block Arrival
├── Block Processor extracts transactions
├── Token Manager identifies token events:
│   ├── Pool creation (V2/V3/V4)
│   ├── Liquidity additions
│   ├── Trading activity
│   └── Price updates
└── Generates LiveToken objects with pool data
```

### 2. Multi-Pool Token Processing
```
LiveBlockTokenProcessor
├── Receives updated tokens from block
├── For each token:
│   ├── Identifies all associated pools
│   ├── Pool types: V2 (address), V3 (address), V4 (poolId)
│   ├── Creates update event with pool data
│   └── Publishes to strategy managers
└── Triggers callbacks with block number
```

### 3. Position Management Flow
```
StrategyPositionManager
├── Receives token updates
├── For each token-pool combination:
│   ├── Creates composite key: {token}-{pool}
│   ├── Checks position cache (LRU)
│   ├── Creates/updates TokenPosition
│   ├── Applies strategy logic
│   └── Persists to database
└── Returns updated positions
```

### 4. Strategy Execution
```
Investment Strategy
├── Evaluates token metrics:
│   ├── Price movement
│   ├── Volume analysis
│   ├── Liquidity depth
│   ├── Scam indicators
│   └── Custom criteria
├── Generates signals:
│   ├── BUY: Open new position
│   ├── SELL: Close position
│   └── HOLD: Maintain position
└── Returns decision with metadata
```

### 5. Database Persistence
```
PostgreSQL Storage
├── token_positions table:
│   ├── Composite primary key (token, pool, strategy)
│   ├── Current position state
│   └── JSONB for flexible data
├── token_position_snapshots:
│   ├── Time-series data
│   ├── Historical performance
│   └── Metrics tracking
└── Indexed for performance
```

## Pool Type Handling

### Uniswap V2 Pools
- **Identifier**: Pool contract address
- **Example**: `0x1234...abcd`
- **Access**: Direct contract calls

### Uniswap V3 Pools
- **Identifier**: Pool contract address
- **Example**: `0x5678...efgh`
- **Access**: Direct contract calls with tick data

### Uniswap V4 Pools
- **Identifier**: PoolManager address + PoolId
- **Example**: `0x0000...0090#12345678`
- **Access**: Through singleton PoolManager contract

## Integration Points

### 1. Token Data Integration
- Receives `LiveToken` objects with `get_pool_info_dict()` method
- Handles backward compatibility with legacy pool_info structure
- Supports new PoolManager architecture

### 2. Blockchain Data Sources
- **Block Processor**: Real-time block data via WebSocket
- **Token Processor**: Token state updates from LiveTokensCache
- **Mempool Monitor**: Scam detection signals (when integrated)

### 3. External Systems
- **Rust Mempool Processor**: 
  - Receives token/pool updates via ZeroMQ PUB (port 5557)
  - Loads current state from Redis token snapshots and the Redis token index
  - Uses data for transaction impact assessment
- **Web Interface**: Portfolio monitoring UI (Sarigoz)
- **Analytics**: Performance reporting and backtesting

### 4. ZeroMQ Communication Protocol
```python
# PUB/SUB Updates (port 5557)
{
    "type": "pool_updates",
    "timestamp": 1234567890,
    "data": {
        "0xPoolAddress": {
            "token_address": "0xTokenAddress",
            "pool_type": "V2",
            "eth_reserve": 125.5,
            "token_reserve": 1000000.0,
            "buy_tax": 0.05,
            "sell_tax": 0.05,
            "trading_enabled": true
        }
    }
}

# Startup/query state is served from Redis token snapshots/index.
```

## Performance Considerations

### 1. Concurrency Management
- Semaphore-limited concurrent processing (default: 10)
- Prevents resource exhaustion
- Maintains system responsiveness

### 2. Caching Strategy
- LRU cache for active positions (default: 10,000)
- Composite key indexing
- Automatic eviction of stale data

### 3. Database Optimization
- JSONB for flexible schema evolution
- Composite indexes on (token, pool, strategy)
- Efficient bulk operations

### 4. Memory Management
- Lazy loading of position data
- Efficient serialization/deserialization
- Garbage collection friendly structures

## Position Lifecycle

### State Transitions
```
PENDING (Initial State)
    │
    ├─→ ACTIVE (Position Opened)
    │      │
    │      ├─→ CLOSED (Position Exited)
    │      │
    │      └─→ EXPIRED (Time-based Exit)
    │
    └─→ CANCELLED (Never Executed)
```

### Position States
```python
class TokenPositionState(Enum):
    INIT = "Init"  # Token created, no position
    BUY_SUBMITTED = "Buy Submitted"  # Buy order sent
    BUY_CONFIRMED = "Buy Confirmed"  # Position active
    SELL_SUBMITTED = "Sell Submitted"  # Sell order sent
    SELL_CONFIRMED = "Sell Confirmed"  # Position closed
    SCAMMED = "Scammed"  # Token identified as scam
```

### State Transitions
```
INIT → BUY_SUBMITTED
  - Trigger: Strategy buy signal
  - Actions: Record entry price, set active

BUY_SUBMITTED → BUY_CONFIRMED
  - Trigger: Confirmation signal
  - Actions: Begin P&L tracking

BUY_CONFIRMED → SELL_SUBMITTED
  - Trigger: Strategy sell signal
  - Actions: Prepare for exit

SELL_SUBMITTED → SELL_CONFIRMED
  - Trigger: Confirmation signal
  - Actions: Finalize P&L, close position

ANY_STATE → SCAMMED
  - Trigger: Scam detection
  - Actions: Mark loss, disable trading
```

## Strategy Framework

### Base Strategy Interface
```python
class BaseStrategy(ABC):
    @abstractmethod
    def analyze_token(self, live_token, token_position) -> Optional[TradeSignal]:
        pass
    
    @abstractmethod
    def handle_init_state(self, live_token, token_position) -> Optional[TradeSignal]:
        pass
    
    @abstractmethod
    def handle_buy_submitted_state(self, live_token, token_position) -> Optional[TradeSignal]:
        pass
    
    @abstractmethod
    def handle_buy_confirmed_state(self, live_token, token_position) -> Optional[TradeSignal]:
        pass
    
    @abstractmethod
    def handle_sell_submitted_state(self, live_token, token_position) -> Optional[TradeSignal]:
        pass
```

### Built-in Strategies

#### 1. BuyAll Strategy
- Buys all trading-enabled tokens
- Sells at configurable profit target
- Simple state machine implementation
- Used for testing and benchmarking

#### 2. BuyScam Strategy
- Focuses on high-risk tokens
- Quick entry/exit strategy
- Enhanced scam detection
- High-risk, high-reward approach

#### 3. MarketTracker Strategy
- Passive market monitoring
- No trading, only tracking
- Used for analytics
- Collects performance data

### Custom Strategy Development
```python
from eth_portfolio_manager.strategy.base_strategy import BaseStrategy
from eth_portfolio_manager.core.data_models import TradeSignal, TradingDecision

class MyCustomStrategy(BaseStrategy):
    def analyze_token(self, live_token, token_position):
        state = token_position.latest_snapshot.position_state
        
        if state == TokenPositionState.INIT:
            # Custom entry logic
            if self.should_buy(live_token):
                return TradeSignal(
                    token_address=live_token.token_data.contract_address,
                    decision=TradingDecision.SUBMIT_BUY,
                    quantity=0.01,
                    strategy_name="MyCustomStrategy"
                )
        
        return None
```

## Database Schema

### strategy_runs Table
```sql
CREATE TABLE strategy_runs (
    id SERIAL PRIMARY KEY,
    name VARCHAR,
    parameters JSONB,
    start_block INTEGER,
    end_block INTEGER,
    created_at TIMESTAMP
);
```

### token_positions Table
```sql
CREATE TABLE token_positions (
    id SERIAL PRIMARY KEY,
    strategy_run_id INTEGER REFERENCES strategy_runs(id),
    token_address VARCHAR NOT NULL,
    pool_address VARCHAR,
    currency VARCHAR,
    token_position JSONB,
    UNIQUE(strategy_run_id, token_address, pool_address)
);

CREATE INDEX idx_position_state ON token_positions(position_state);
CREATE INDEX idx_strategy_name ON token_positions(strategy_name);
CREATE INDEX idx_updated_at ON token_positions(updated_at);
```

### token_position_snapshots Table
```sql
CREATE TABLE token_position_snapshots (
    id SERIAL PRIMARY KEY,
    token_address VARCHAR(42) NOT NULL,
    pool_address VARCHAR(100) NOT NULL,
    strategy_name VARCHAR(100) NOT NULL,
    snapshot_timestamp TIMESTAMP NOT NULL,
    current_price NUMERIC,
    volume_24h NUMERIC,
    liquidity_usd NUMERIC,
    pnl_usd NUMERIC,
    pnl_percentage NUMERIC,
    snapshot_data JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (token_address, pool_address, strategy_name) 
        REFERENCES token_positions(token_address, pool_address, strategy_name)
);

CREATE INDEX idx_snapshot_timestamp ON token_position_snapshots(snapshot_timestamp);
CREATE INDEX idx_token_pool_strategy ON token_position_snapshots(token_address, pool_address, strategy_name);
```

## Usage Examples

### Running Live Portfolio Manager
```bash
# Live monitoring with token information publishing
python scripts/live/run_live_portfolio.py

# With mempool integration (if available)
python scripts/live/run_live_portfolio_with_mempool.py
```

The `run_live_portfolio.py` script:
- Connects to local Ethereum node at `http://127.0.0.1:8545`
- Processes blocks starting from `current_block - warmup_blocks`
- Runs configured strategies (default: MarketTracker)
- Publishes token/pool update notifications via ZeroMQ (PUB: 5557)
- Optionally saves strategy results and PnL to database

Configuration in script:
```python
warmup_blocks = 1200              # Process last 1200 blocks on startup
save_strategy_results = False     # Don't persist to DB by default
add_pnl_to_db = False            # Don't write PnL by default
max_pools = 2000                 # Track up to 2000 pools
min_eth_threshold = 0.01         # Only track pools with > 0.01 ETH
```

### Running Backtest
```bash
# Basic backtest
python scripts/backtesting/run_backtester.py

# With pool tracking
python scripts/backtesting/run_pool_backtester.py

# Debug mode
python scripts/backtesting/run_backtester_debug.py
```

### Accessing Portfolio Data
```python
# Via API
response = requests.get("http://localhost:5000/api/token_positions")
positions = response.json()

# Direct database access
from eth_portfolio_manager.db.token_position_db_models import TokenPosition
positions = session.query(TokenPosition).filter_by(
    strategy_run_id=run_id
).all()

# Get performance metrics
metrics = session.query(TokenPositionSnapshotORM).filter(
    TokenPositionSnapshotORM.snapshot_timestamp >= start_date
).all()
```

### Verification Tools
```bash
# Verify pool level publishing is working
python scripts/server/verify_pool_levels.py

# This tool:
# - Connects to ZeroMQ publishers
# - Monitors real-time updates
# - Compares published data with blockchain
# - Reports any discrepancies
```

## Monitoring & Observability

### Logging
- Structured logging with contextual information
- Log levels: DEBUG, INFO, WARNING, ERROR
- Separate log files per component
- Log rotation and retention policies

### Metrics
- Position count by state
- P&L tracking by strategy
- Processing latency measurements
- Cache hit rates

### Health Checks
- Database connectivity
- Cache availability
- Strategy engine status
- Block processing lag

## Error Handling

### Graceful Degradation
- Continue processing other tokens if one fails
- Retry transient failures with exponential backoff
- Log and alert on persistent errors
- Maintain position consistency

### Recovery Mechanisms
- Reload positions from database on restart
- Replay missed blocks
- Reconcile state discrepancies
- Manual intervention tools

## Best Practices

### Strategy Development
1. Start with backtesting
2. Validate on recent data
3. Monitor initial live performance
4. Use conservative position sizes

### Risk Management
1. Set maximum position limits
2. Implement stop-loss logic
3. Monitor scam indicators
4. Diversify across pools

### Performance Tuning
1. Adjust concurrency limits based on load
2. Monitor memory usage
3. Optimize database queries
4. Use caching effectively

### Operational Excellence
1. Regular database backups
2. Monitor system health
3. Set up alerting
4. Document custom strategies

## Future Enhancements

### Planned Features
1. **Cross-pool arbitrage detection**
2. **MEV protection strategies**
3. **Advanced risk management**
4. **Multi-chain support**
5. **Real-time P&L websocket API**

### Architecture Evolution
1. **Event sourcing for position history**
2. **Distributed processing with Kafka**
3. **Machine learning strategy optimization**
4. **Automated backtesting pipeline**
5. **Integration with DeFi protocols**

## Conclusion

The Portfolio Manager provides a robust, scalable foundation for managing cryptocurrency portfolios on Ethereum. Its architecture supports multiple pools per token, configurable strategies, and comprehensive state tracking, making it suitable for both research and production trading operations.
