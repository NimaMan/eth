# Signal Engine

The Signal Engine is a high-performance transaction analysis system that detects specific function calls and market events in Ethereum mempool transactions with sub-millisecond latency.

## Components

### 1. Function Detector (`function_detector.rs`)

The FunctionDetector is a real-time transaction categorization system that identifies specific function calls by analyzing 4-byte function selectors.

#### Architecture

**Core Components:**

1. **Three Specialized Detectors:**
   - `LiquidityRemovalDetector`: Detects 11 liquidity removal functions across major DEXs
   - `TradingEnabledDetector`: Detects 4 trading activation functions
   - `SwapDetector`: Detects 24 swap functions from major protocols

2. **Data Structures:**
   - `TransactionWithFunctions`: Enriched transaction data with detected functions
   - `SignalAlert`: Structured alert for ZMQ publishing
   - `FunctionStats`: Global statistics tracking

3. **Output Channels:**
   - Separate log files for liquidity removals and trading enabled events
   - ZMQ publisher on port 5556 for real-time alerts
   - Performance metrics logging

#### Function Signatures Detected

**Liquidity Removals (11 signatures):**
- Uniswap V2: `removeLiquidityETH`, `removeLiquidity`, `removeLiquidityETHSupportingFeeOnTransferTokens`, etc.
- Uniswap V3: `decreaseLiquidity`
- Balancer: `exitPool`
- Curve: `remove_liquidity`, `remove_liquidity_one_coin`, `remove_liquidity_imbalance`

**Trading Enabled (4 signatures):**
- `setTradingEnabled` (8a8c523c)
- `enableTrading` (8ee88c53)
- `openTrading` (c9567bf9)
- `startTrading` (fb201b1d)

**Swaps (24 signatures):**
- Uniswap V2/V3: 12 functions including `swapExactTokensForTokens`, `exactInputSingle`, etc.
- 1inch: 3 functions including protocol-specific swaps
- 0x Protocol: 2 functions
- Curve: 2 exchange functions
- Balancer: 2 swap functions

#### Performance Characteristics

- **Detection Latency**: Average 0.005ms, Maximum 0.256ms
- **Throughput**: Processes 600-700 transactions per minute
- **Memory Usage**: Minimal - uses static lazy initialization
- **Concurrency**: Thread-safe with Mutex-protected resources

#### Usage Flow

1. **Initialization**: Creates log directory with timestamp, initializes detectors
2. **Batch Processing**: `detect_batch()` processes multiple transactions efficiently
3. **Detection Logic**: 
   - Extracts 4-byte selector from transaction input
   - Checks against signature maps in order: liquidity → trading → swaps
   - Returns first match (functions are mutually exclusive)
4. **Output**: 
   - Logs to appropriate file with full transaction details
   - Publishes SignalAlert via ZMQ for downstream consumers
   - Updates global statistics

#### Integration Points

- **Input**: Receives `NonBlockingTransaction` from mempool fetcher
- **Output**: Returns `TransactionWithFunctions` with detected function names
- **Downstream**: Signal detector uses function names to filter swap transactions from scam alerts

#### Known Issues

1. **Limited Error Handling**: Panics on file creation failures if log directory cannot be created

#### Statistics Tracking

The detector maintains real-time statistics:
- Total transactions checked
- Liquidity removals detected
- Trading enabled events
- Swaps identified
- Other functions (unrecognized)

These statistics are logged periodically and available via `get_stats()`.

### 2. Signal Publisher (`publisher.rs`)

[To be documented]

### 3. Transaction Types (`types.rs`)

[To be documented]

### 4. Engine Core (`engine.rs`)

[To be documented]

## Architecture Decisions

1. **Static Lazy Initialization**: Uses lazy_static for one-time initialization of resources
2. **Separate Log Files**: Different event types go to dedicated log files for easier analysis
3. **ZMQ Publishing**: Real-time alerts for external consumers
4. **Function Priority**: Liquidity removals checked first, then trading, then swaps
5. **No Function Aggregation**: Each transaction has one primary function type

## Performance Optimization

1. **Early Exit**: Returns immediately upon first function match
2. **Pre-computed Hashes**: Function selectors stored as static strings
3. **Batch Processing**: Processes multiple transactions in single call
4. **Non-blocking ZMQ**: Uses DONTWAIT flag to prevent blocking

## Future Improvements

1. Fix log directory synchronization issue
2. Add more DEX protocols and functions
3. Implement function parameter decoding
4. Add metrics for specific function popularity
5. Support for multi-function transactions
6. Better error handling and recovery