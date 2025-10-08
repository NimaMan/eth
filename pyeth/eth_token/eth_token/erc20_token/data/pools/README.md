# Pool Management System Architecture

## Overview

The pool management system is designed to track and analyze liquidity pools for ERC20 tokens across different DEX protocols (Uniswap V2, V3, V4). The architecture follows a clean separation of concerns with each component having a specific responsibility.

## Core Concepts

### 1. Pool Hierarchy
- **Individual Pools**: Each pool (V2, V3, V4) is a self-contained entity that manages its own state
- **Pool Manager**: Aggregates and coordinates all pools for a specific token
- **Pool Liquidity Matrix**: Provides token-level analytics and cross-pool comparisons

### 2. Data Flow
1. **Transaction Processing**: Blockchain transactions flow into the system
2. **Event Routing**: Pool Manager routes relevant events to appropriate pools
3. **State Updates**: Each pool updates its internal state (reserves, prices, LP holders)
4. **Analysis**: Pool Liquidity Matrix snapshots pool data for token-level insights

## Component Responsibilities

### Base Pool (`base_pool.py`)
- **Purpose**: Abstract base class defining the interface all pools must implement
- **Responsibilities**:
  - Define common pool properties (reserves, prices, decimals, trading flags)
  - Establish event processing interface and trading enablement hooks
  - Provide standard getters for pool state and health (scam detection, price history)

### Pool Implementations
#### UniswapV2Pool (`uniswap_v2_pool.py`)
- **Purpose**: Handle Uniswap V2 specific logic
- **Responsibilities**:
  - Track reserve changes via Sync events
  - Monitor LP token transfers for ownership tracking
  - Calculate prices using constant product formula
  - LP tokens as ERC20 with complete holder registry

#### UniswapV3Pool (`uniswap_v3_pool.py`)
- **Purpose**: Handle Uniswap V3 concentrated liquidity
- **Responsibilities**:
  - Track liquidity in price ranges
  - Handle multiple fee tiers (0.01%, 0.05%, 0.3%, 1%)
  - Calculate prices from tick data
  - NFT position representation

#### UniswapV4Pool (`uniswap_v4_pool.py`)
- **Purpose**: Handle Uniswap V4 singleton architecture
- **Responsibilities**:
  - Work with PoolId instead of addresses
  - Handle hook-based custom logic
  - Track dynamic fee structures
  - All pools share one PoolManager contract

### Pool Manager (`pool_manager.py`)
- **Purpose**: Central coordinator for all pools of a token
- **Responsibilities**:
  - Discover new pools from creation events
  - Route transaction events to appropriate pools
  - Provide aggregated views of all pools
  - Maintain pool registry and indexes
  - Support both creation-based and swap-based pool discovery

### Pool Chain Data Fetcher (`pool_chain_data_fetcher.py`)
- **Purpose**: Fetch pool configuration from blockchain
- **Responsibilities**:
  - Query pool contracts for token pairs and fees
  - Handle Web3 connections and RPC calls
  - Support pool discovery when detected through swaps
  - Cache discovered pool configurations

### Pool Liquidity Matrix (`pool_liquidity_matrix.py`)
- **Purpose**: Token-level analysis across all pools
- **Responsibilities**:
  - Snapshot liquidity and pricing state for every tracked pool
  - Determine best executable price for buys or sells across pools
  - Aggregate liquidity by denomination and overall token exposure
  - Provide lightweight trade impact simulations (constant-product AMMs)
  - Surface per-pool trading availability (`can_buy` / `can_sell`) to upstream consumers

### Pool Reserve Tracker (`pool_reserve_tracker.py`)
- **Purpose**: Historical tracking of pool reserves
- **Responsibilities**:
  - Take snapshots of pool states
  - Track reserve changes over time
  - Feed scam detection metadata back into the pool (rug detection, label, tx hash)

### Arbitrage Detector (`arbitrage_detector.py`)
- **Purpose**: Identify price discrepancies across pools
- **Responsibilities**:
  - Compare prices across different pools
  - Calculate arbitrage opportunities
  - Detect price manipulation

## Design Principles

### 1. Separation of Concerns
- Each pool type handles its own protocol-specific logic
- Pool Manager only coordinates, doesn't implement pool logic
- Analysis components are separate from data components

### 2. Event-Driven Architecture
- Pools react to blockchain events
- State changes are triggered by transactions
- No polling or active querying during normal operation

### 3. Self-Contained Pools
- Each pool maintains its complete state
- Pools don't depend on other pools
- Pool Manager provides coordination, not shared state

### 4. Clean Interfaces
- Base class defines standard interface
- Protocol differences are abstracted away
- Consumers work with pools uniformly

## What Doesn't Belong Here

### 1. Blockchain Interaction
- Web3 connections and RPC calls should be handled by upstream components
- Pool discovery from blockchain should be handled by specialized services
- ABI definitions should be in a central location, not embedded in code

### 2. Database Operations
- Persistence logic belongs in dedicated writer/reader classes
- Pool state should be pure in-memory during processing
- Database schemas are external concerns

### 3. Business Logic
- Scam detection rules belong in dedicated analyzers
- Trading strategies are external to pool tracking
- Price manipulation detection is a separate concern

## Refactoring Opportunities

### 1. Pool Discovery Service
The `_try_create_v3_pool_from_swap` method should be extracted to a separate PoolDiscoveryService that:
- Handles blockchain queries for pool details
- Manages Web3 connections
- Provides pool creation data to the manager

### 2. Price Oracle Service  
Price selection logic could be extracted to a dedicated PriceOracle that:
- Implements different pricing strategies
- Handles denomination preferences
- Provides confidence scores

### 3. Event Processing Pipeline
Event routing could be formalized into a pipeline pattern:
- Event filters determine relevance
- Event routers direct to appropriate handlers
- Event processors update state

## Key Features

### Dynamic Pool Discovery
The system employs two complementary discovery mechanisms:

**Creation Event Detection**
- Monitors `PairCreated` (V2), `PoolCreated` (V3), and `PoolInitialized` (V4) events
- Automatically registers new pools as they're deployed on-chain
- Captures pool metadata at creation time

**Swap-Based Retroactive Discovery**
- Identifies pre-existing pools through swap activity
- When a swap occurs in an untracked pool, uses PoolChainDataFetcher to query blockchain
- Ensures complete pool coverage even when starting mid-stream

### Liquidity Provider (LP) Tracking for V2 Pools
- Complete LP token holder registry with real-time updates
- Individual holder balances and percentage shares
- Transfer history for ownership changes
- Mint/burn event tracking for liquidity changes
- Critical for detecting:
  - High concentration (manipulation risk)
  - Single holder dominance (rug pull risk)
  - Distributed ownership (community health)
- Many holders (liquidity stability)

### Trading Status Detection
- Monitors first swap, mint, or burn event on any pool
- Tracks when trading becomes enabled for a token
- Records block number, transaction hash (`trading_enabled_tx`), and timestamp of enablement
- Maintains runtime-only sell status for honeypot detection

## Protocol Differences

### Address vs PoolId Management
- **V2/V3**: Each pool has a unique contract address
- **V4**: All pools share one PoolManager contract, identified by PoolId (bytes32 hash)
- Pool display format: V2/V3 use address directly, V4 uses "PoolManager#poolId"

### Fee Structure Variations
- **V2**: Fixed 0.3% fee on all swaps
- **V3**: Multiple fee tiers (0.01%, 0.05%, 0.3%, 1%) create separate pools
- **V4**: Dynamic fees possible via hooks

### Event Processing Patterns
- **Sync Events**: Reserve updates (V2 only)
- **Swap Events**: Trade execution (all protocols)
- **Mint/Burn**: Liquidity changes (V2/V3 have separate events)
- **ModifyLiquidity**: Combined mint/burn for V4
- **Initialize**: Pool creation (V3/V4)

## Usage Patterns

### For Token Analysis
```
1. Initialize PoolManager for a token
2. Process transactions through the manager
3. Snapshot pool state via PoolLiquidityMatrix.snapshots()
4. Access individual pools for detailed information and trading status
```

### For Price Discovery
```
1. PoolLiquidityMatrix.get_best_price(for_buy=...) for executable price
2. Filter by specific denominations (WETH, stablecoins)
3. Consider pool reserves and trading flags for confidence
```

### For Scam Detection
```
1. Analyze liquidity distribution across pools
2. Check for concentrated ownership (V2 pools)
3. Monitor abnormal reserve changes via PoolReserveTracker
4. Correlate pool scam labels with token-level health predictors
```

### For LP Analysis (V2 Pools)
```
1. Get LP holder distribution via pool.get_lp_holders()
2. Check ownership concentration via pool.get_lp_share(address)
3. Monitor mint/burn events for liquidity changes
4. Track LP token transfers for ownership shifts
```
