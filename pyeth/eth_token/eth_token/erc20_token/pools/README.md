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

### Trading Viability Simulation
- `evaluate_trading_status` replays all creator-controlled transactions from the **current** block against the **previous** canonical block header. We do this because reth keeps the prior block fully indexed even when the latest block is still synchronizing.
- The simulator seeds its EVM state with the previous block and replays `latest_block_txs` in canonical order before running the test buy/approve/sell sequence. Any transaction the real chain mined before the target (such as a funding transfer in block *N*) must therefore be present in this replay set.
- If a funding transfer landed in block *N* but we only replay block *N*+1 transactions, the simulator will believe the account is unfunded and produce the “lack of funds … for max fee …” error. On-chain the same transaction succeeds, so this discrepancy signals that our replay omitted earlier state transitions.
- Fix strategies:
  - Extend `latest_block_txs` (or a companion buffer) to include control-address activity from the previous block whenever we simulate against `block_number - 1`.
  - Detect the specific “lack of funds” failure, cross-check the account balance via `chain_query.get_account`, and either fetch/replay the missing funding transaction or temporarily top up the account in simulation with the known difference.
  - Keep the error logged for observability, but treat it as non-fatal if on-chain execution already confirms the action, to avoid cascading pool viability failures.
- Conceptually, viability failures fall into two categories: genuine honeypot behavior (buy or sell reverts) and replay gaps. Understanding which side a failure belongs to is critical for downstream consumers of `can_buy` / `can_sell`.

### Simulation Triggers & Control Addresses
- Every transaction that flows through a tracked pool ends with `BasePool.check_and_update_trading_status`. This gate decides whether we should simulate the buy/approve/sell sequence.
- Before we have proof that trading works we run the simulator whenever a block contains a transaction from a control address or when we first observe swaps/mints that indicate trading may have gone live. Once we have recorded at least one successful buy *and* sell (`can_buy` and `can_sell` true), we do **not** keep simulating arbitrary user flows—unprivileged wallets cannot flip global buy/sell gates on Ethereum, so their activity cannot change viability.
- Control addresses live in `token_control_addresses`. The set starts with the deployer and current owner (captured during creation), plus any addresses supplied by token metadata.
- Owner change events keep the set fresh, and renounce/transfer events continue to be tracked so we can observe the same wallets should they act again.
- Pool-specific code adds more actors: Uniswap V3 mints register their position owner/recipient, V4 liquidity modifications register owner/sender/recipient/account fields, and the `PoolManager` propagates any global token-level control list to every pool instance.
- Transactions carry a `unique_addresses` set (from, to, event participants, internal call targets). `check_and_update_trading_status` simply intersects that set with `token_control_addresses`. A hit means the transaction came from someone who can toggle trading flags, mutate taxes, or drain liquidity, so we simulate immediately.
- Ethereum enforcement-wise, only storage writes to the token contract can change trading gates (e.g., `tradingEnabled = false`, tax rate hikes, allow/deny lists). Those state writes require the caller to satisfy the contract’s access control (typically `onlyOwner` or an admin role). Ordinary wallets invoking `transfer` or swapping through a router execute code paths that read the flags but cannot modify them, so their activity is read-only with respect to buy/sell viability.
- We therefore re-run simulations when we detect a transaction that *could* have mutated those toggles: (1) direct calls from known admins/control addresses, (2) events that explicitly signify parameter updates (`tax_events`, `trading_disabled_events`, `max_buy_limit_events`, etc.), or (3) liquidity actions initiated by the same privileged wallets (because they might pair a code change with a liquidity rug). Everything else is observed without retriggering the simulator.
- The replay state is built from `latest_block_txs`, so when a control address fires multiple setup calls inside the same block, the simulator sees them all in order before testing the buy path.

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
