# ERC20 Token Live Tracking System

## Overview

The ERC20 Token Live Tracking System is a comprehensive real-time blockchain analytics framework designed to monitor, analyze, and assess ERC20 tokens on Ethereum. It processes blockchain transactions to maintain token state, track liquidity pools across multiple DEX protocols, analyze trading networks, and detect potential scams through pattern recognition.

## Token Control Model

Ethereum enforces token behaviour through contract storage. Any change that toggles trading, rewrites tax parameters, freezes wallets, or drains liquidity must be executed by an address that satisfies the contract’s access control (e.g., `onlyOwner`, admin role, multisig). Ordinary traders invoking `transfer`/`swap` paths cannot mutate those variables—they merely read the flags that privileged callers set. The tracking system therefore keeps an explicit view of *who* can change token state and *which* transactions might have done so.

### Control Address Sources
- **Deployer & Initial Owner** – the account that deploys the token is often the first to hold admin privileges. Contracts typically assign `owner`, `governance`, or `admin` roles during construction so that address can configure taxes, launch trading, or renounce control later.
- **Ownership Transfers** – many tokens expose a `transferOwnership` function (or multisig-controlled governance). When ownership changes, the new administrator inherits the ability to tweak contract state. Even if the previous owner renounces control, historical owners remain important for forensic tracking because they may still interact with the token or related pools.
- **Preconfigured Administrators** – some projects hard-code additional controllers (marketing wallet, tax wallet, multisig) or allow an owner to delegate permissions (e.g., `setController`). Any address that can call privileged functions is part of the control set.
- **Liquidity Operators** – beyond pure contract governance, addresses that own significant liquidity provisioning power can alter tradability. Wallets holding concentrated Uniswap positions, pool hooks, or LP tokens can withdraw reserves, add restrictive hooks, or otherwise reshape liquidity dynamics.

### Broader Controller Discovery
While ownership covers the majority of control surfaces, many modern tokens rely on richer role systems. Conceptually we treat the following categories as privileged and aim to harvest them programmatically:

- **AccessControl Roles** – contracts based on OpenZeppelin-style role management emit `RoleGranted`/`RoleRevoked` events. Roles such as `DEFAULT_ADMIN_ROLE`, `PAUSER_ROLE`, `MINTER_ROLE`, or bespoke `CONTROLLER_ROLE` often govern trading switches and tax parameters. Watching those events lets us maintain an up-to-date roster of role holders.
- **Custom Governance Events** – projects frequently emit explicit events when setting tax wallets, blacklist managers, fee recipients, anti-bot guards, etc. Cataloguing common event signatures (`TaxReceiverUpdated`, `BlacklistManagerChanged`, `RouterSet`, `TradingStatusUpdated`) gives another path to discover active controllers.
- **Proxy & Multisig Admins** – upgradeable tokens introduce proxy admin contracts (e.g., TransparentUpgradeableProxy). The proxy admin or multisig addresses can redeploy logic, indirectly changing token behaviour. Following `AdminChanged`, `Upgraded`, or Gnosis Safe ownership events is essential to track those control surfaces.
- **Router & Allowance Gatekeepers** – some tokens restrict trading to specific routers via allowlists. Contracts that manage these allowlists (or are given infinite allowance to move user funds) qualify as controllers because they can freeze or reroute volume.
- **DEX-Specific Hooks** – Uniswap V4 hooks, limit-order managers, or external wrapper contracts can enforce arbitrary logic on swaps. Operators of those contracts belong alongside liquidity providers in the privileged set.

In practice we can enrich the control-address registry by:
1. Extending the Reth chain-query layer to stream relevant events (`OwnershipTransferred`, `RoleGranted`, custom governance events, proxy admin changes) and push their subjects into our controller set.
2. Inspecting bytecode or decoded function selectors of admin transactions seen in the mempool/chain to flag new privileged addresses dynamically.
3. Replaying simulator traces to observe who mutates storage slots tied to trading/tax flags, then caching those mutators as controllers.

### Governance & Parameter Events
- **Trading Toggles** – Honeypots and stealth launches often gate trading behind boolean flags (`tradingEnabled`, `swapEnabled`) or block lists. Only privileged callers can flip these switches, so each toggle event signals a change in the entire market’s ability to transact.
- **Tax & Fee Adjustments** – Many contracts levy buy/sell taxes routed to marketing or development wallets. Adjusting these rates, setting maximum buy sizes, or updating whitelist/blacklist tables can radically change trade viability and is only accessible to administrators.
- **Liquidity Management** – Large deposits or removals of liquidity performed by privileged wallets (or their routers) can trigger price shocks, slip protections, or leave pools illiquid. Recognising these moves as governance actions rather than organic trading is essential for correct viability analysis.
- **Allowance & Router Control** – Setting all-allowances for routers or revoking them dictates which DEX paths remain usable. Administrative accounts often manage these approvals to guide or restrict flow.

### Why It Matters
- **Simulation Triggers** – Automated viability checks re-run whenever a privileged actor or governance event occurs. If an admin disables trading, raises taxes to 90%, or withdraws liquidity, the next simulation reflects the new ground truth before any downstream consumer assumes trading is safe.
- **Scam & Risk Detection** – Sudden governance changes (tax spikes, blacklist additions, liquidity rugs) are strong scam indicators. Tracking the trusted set of controllers allows health modules to distinguish routine user trades from policy updates.
- **Auditability & Incident Response** – When something goes wrong, analysts need to answer “who changed what and when?” Capturing privileged transactions provides a full history of policy changes, making it possible to attribute actions to specific wallets and reason about counterparty risk.

## System Architecture

```
┌─────────────────────┐
│   ERC20Token       │  Main Facade
│ (erc20_token.py)   │
└─────┬───────────────┘
      │
      ├─── Token Data Layer ──────────────────────┐
      │                                           │
      │  ┌─────────────────────────────────┐     │
      │  │     ERC20TokenData              │     │
      │  │  (data/erc20_token_data.py)     │     │
      │  └─────────────┬───────────────────┘     │
      │                │                          │
      │                ├── Pool Management        │
      │                │   └── PoolManager        │
      │                │       └── V2/V3/V4 Pools │
      │                │                          │
      │                └── Liquidity Analysis     │
      │                    └── PoolLiquidityMatrix
      │
      ├─── Network Analysis Layer ─────────────────┐
      │                                            │
      │  ┌─────────────────────────────────┐      │
      │  │     LiveTokenNetwork            │      │
      │  │  (network/token_network.py)     │      │
      │  └─────────────┬───────────────────┘      │
      │                │                           │
      │                ├── Network Builder         │
      │                │   └── Graph Construction  │
      │                │                           │
      │                └── Subgraph Analysis       │
      │                    └── Related Address Detection
      │
      └─── Health Assessment Layer ────────────────┐
                                                   │
         ┌─────────────────────────────────┐       │
         │   TokenHealthPredictor          │       │
         │ (token_health/*)                │       │
         └─────────────┬───────────────────┘       │
                       │                            │
                       ├── Volume Analysis          │
                       │   └── Wash Trading Detection
                       │                            │
                       └── Actor Classification     │
                           └── Scam Scoring
```

## Data Flow

### 1. Transaction Entry Point

```python
ERC20Token.update_from_transaction(transaction)
    ├── ERC20TokenData.update_from_transaction()
    │   ├── Parse transaction logs
    │   ├── Extract relevant events
    │   ├── Update token state
    │   └── Process pool events
    │
    ├── LiveTokenNetwork.update_from_transaction()
    │   ├── Build/update transfer graph
    │   ├── Calculate user metrics
    │   └── Detect related addresses
    │
    └── TokenHealthPredictor.update_from_transaction()
        ├── Analyze volume patterns
        ├── Check actor involvement
        └── Generate scam scores
```

### 2. Event Processing Pipeline

```
Transaction Logs
    │
    ├── Token Events
    │   ├── ERC20 Transfers
    │   ├── Approvals
    │   └── Ownership Changes
    │
    ├── DEX Events
    │   ├── Pool Creation (V2/V3/V4)
    │   ├── Swaps
    │   ├── Liquidity Changes (Mint/Burn)
    │   └── Price Updates (Sync)
    │
    └── Internal Transactions
        ├── ETH Transfers
        └── Contract Interactions
```

## Core Components

### 1. ERC20Token (Main Interface)
**File**: `erc20_token.py`

The main facade that coordinates all subsystems:
- Initializes data, network, and health components
- Routes transactions to appropriate handlers
- Provides unified access to token analytics

**Key Methods**:
```python
update_from_transaction(transaction)  # Process new blockchain data
```

**Key Properties**:
```python
token_data                 # ERC20TokenData backing store
token_network              # LiveTokenNetwork view
token_health_predictor     # Health-scoring coordinator
latest_token_assessment    # Cached result from last health update
```

### 2. ERC20TokenData (Data Management)
**File**: `data/erc20_token_data.py`

Maintains all token state and historical data:

**Core Data**:
- Token metadata (name, symbol, decimals, supply)
- Creation details (block, timestamp, creator)
- Trading status and enablement tracking

**Event Collections**:
```python
# Transfer tracking (keyed by tx hash)
erc20_transfers: Dict[str, List[Transfer]]
eth_transfers: Dict[str, List[InternalTransfer]]
other_denom_transfers: Dict[str, List[Transfer]]

# Approvals & ownership
approvals: List[Approval]
owner_events: List[OwnershipTransferredEvent]
approved_addresses: Set[str]
address_tx_counter: Dict[str, int]
bribe_amount_dict: Dict[str, float]  # Per-tx bribe accounting

# Pool coordination
pool_manager: PoolManager                  # Centralized pool registry
liquidity_matrix: PoolLiquidityMatrix      # Aggregated liquidity/pricing view
pools: PoolCollection                      # Convenience accessor for BasePool instances
```

### 3. Pool Management System
**Directory**: `data/pools/`

Modular pool tracking across Uniswap protocols:

**PoolManager** (`pool_manager.py`):
- Discovers pools from creation events and swaps
- Routes events to appropriate pool instances
- Provides aggregated pool views

**Pool Implementations**:
- `UniswapV2Pool`: Constant product AMM, LP token tracking
- `UniswapV3Pool`: Concentrated liquidity, tick-based pricing
- `UniswapV4Pool`: Hook-enabled pools, PoolId identification

**PoolLiquidityMatrix** (`data/pool_liquidity_matrix.py`):
- Snapshots liquidity/pricing across every tracked pool
- Finds best executable prices (buy/sell) with trading availability checks
- Aggregates reserves by denomination and total token exposure
- Provides lightweight Uniswap V2 swap simulation helpers

**PoolReserveTracker** (`data/pools/pool_reserve_tracker.py`):
- Maintains bounded history of reserves and prices for each pool
- Flags scam patterns (liquidity rug, asymmetric drains) with metadata
- Surfaces scam label/block/hash back to the BasePool

### 4. Network Analysis
**Directory**: `network/`

Graph-based analysis of token transfer networks:

**LiveTokenNetwork** (`token_network.py`):
- Extends LiveTokenNetworkBuilder
- Manages transfer graph construction
- Calculates aggregated metrics for related addresses

**Key Features**:
```python
# User activity tracking
UserActivityData:
    - token_balance, denom_balance
    - realized_profit, unrealized_profit
    - transfer counts and counterparties

# Subgraph analysis
NetworkSubgraphAnalyzer:
    - find_subgraphs()  # Connected components
    - get_related_addresses()  # Address clusters
    - simplify_graph()  # Remove noise
```

### 5. Health Assessment
**Directory**: `token_health/`

Pattern-based scam and manipulation detection:

**TokenHealthPredictor** (`token_health_predictor.py`):
- Coordinates health assessment components
- Maintains scam scores with confidence levels

**VolumeAnalyzer** (`volume_analyzer.py`):
- Detects wash trading patterns
- Identifies fake volume generation
- Classifies actors (green/grey/neutral)

**Detection Patterns**:
```python
# Deceptive transfers
- >15 transfers or >20 addresses in single tx
- Circular transfers without economic purpose

# Malicious actors
- Known scammer addresses
- Suspicious contract interactions

# Liquidity manipulation
- Reserve depletion below thresholds
- Hidden mints (supply > expected)
```

## Key Algorithms

### Price Calculation

**Best Price Selection** (`PoolLiquidityMatrix.get_best_price`):
```python
1. Iterate through all tracked pools
2. Skip pools with zero/negative price or missing trading capability (`can_buy` / `can_sell`)
3. Build lightweight snapshots (price, reserves, protocol metadata)
4. Return min-price snapshot when buying, max-price snapshot when selling
```

### Scam Detection

**Multi-Signal Analysis**:
```python
ScamScore:
    - block_number: Detection block
    - confidence: 0.0 to 1.0
    - reason: Detection trigger
    
Signals:
    - Deceptive patterns (confidence: 0.5)
    - Malicious actors (confidence: 0.99)
    - Token scam label (confidence: 1.0)
```

### Network Analysis

**Related Address Detection**:
```python
1. Build directed graph from transfers
2. Find strongly connected components
3. Aggregate metrics per component:
   - Combined balances
   - Total profits
   - Shared counterparties
```

## Usage Patterns

### 1. Real-time Token Monitoring
```python
# Initialize token tracking
token = ERC20Token(contract_address)

# Process incoming transactions
for transaction in blockchain_stream:
    token.update_from_transaction(transaction)
    
# Access current state
# Returns (snapshot, price) when available
best_buy = token.token_data.liquidity_matrix.get_best_price(for_buy=True)
if best_buy:
    best_buy_snapshot, buy_price = best_buy

health = token.latest_token_assessment
```

### 2. Pool Analysis
```python
# Get all pools for token
pools = token.token_data.pool_manager.get_all_pools()

# Summarise pool health (scam labels, reserves, trading status)
pool_stats = token.token_data.pool_manager.get_pool_health_stats()

# Liquidity distribution across denominations
liquidity_by_denom = token.token_data.liquidity_matrix.total_liquidity_by_denom()

# LP holder analysis (V2)
lp_distribution = pool.get_lp_holders()
```

### 3. Network Analysis
```python
# Get user activity with related addresses
user_df = token.token_network.get_agg_user_activity_df()

# Find address clusters
components = token.token_network.connected_components

# Check specific address relationships
related = token.token_network.subgraph_analyzer.get_related_addresses(addr)
```

### 4. Health Assessment
```python
# Get latest health assessment
assessment = token.latest_token_assessment

# Check scam scores
if assessment['is_scam']:
    print(f"Scam detected: {assessment['scam_reason']}")
    
# Review involved actors
green_actors = assessment['involved_green_actors']
mal_actors = assessment['involved_mal_actors']
```

## Performance Considerations

### Memory Management
- Bounded collections (last 1000 events per type)
- Transaction-indexed storage for O(1) lookup
- Lazy initialization of expensive components

### Processing Efficiency
- Event batching per transaction
- Incremental graph updates
- Cached pool reserves

### Scalability
- Modular pool system supports new protocols
- Pluggable health detection algorithms
- Extensible event processing pipeline

## Configuration

### Thresholds
**File**: `config/scam_thresholds.py`
- Liquidity thresholds per denomination
- Pattern detection parameters
- Actor classification rules

### Logging
- Separate loggers per component
- Configurable log levels
- Performance metrics tracking

## Integration Points

### Required External Services
1. **Ethereum Node**: Web3 connection for blockchain data
2. **Contract ABIs**: For decoding event logs
3. **Price Feeds**: For USD valuations

### Output Interfaces
1. **Pandas DataFrames**: For data analysis
2. **JSON Serialization**: Via `to_dict()` methods
3. **Metric Dictionaries**: For monitoring systems

## Best Practices

### 1. Transaction Processing
- Always check transaction status before processing
- Handle missing events gracefully
- Maintain event ordering within transactions

### 2. Pool Management
- Verify pool existence before operations
- Handle multiple pools per token pair
- Consider liquidity depth for price reliability

### 3. Network Analysis
- Limit graph size for performance
- Filter noise addresses (exchanges, routers)
- Aggregate metrics for related addresses

### 4. Health Assessment
- Combine multiple signals for accuracy
- Weight signals by confidence
- Track historical assessments

## Extending the System

### Adding New DEX Protocol
1. Create new pool class extending `BasePool`
2. Add event handlers to `ERC20TokenData`
3. Update `PoolManager` discovery logic
4. Extend `PoolLiquidityMatrix` helpers or introduce protocol-specific simulation logic if needed

### Adding New Health Signals
1. Extend `VolumeAnalyzer` with new patterns
2. Add detection method to `TokenHealthPredictor`
3. Update `ScamScore` with new reason codes
4. Adjust confidence scoring logic

### Custom Analytics
1. Access raw event data from `ERC20TokenData`
2. Use network graph from `LiveTokenNetwork`
3. Implement custom analysis logic
4. Integrate results into token metrics

## Troubleshooting

### Common Issues

1. **Missing Pool Data**
   - Check if pools are discovered
   - Verify event processing for pool protocol
   - Ensure sufficient liquidity for tracking

2. **Incomplete Network Graph**
   - Verify transfer event processing
   - Check address filtering logic
   - Ensure graph updates are called

3. **False Positive Scams**
   - Review detection thresholds
   - Check actor classification
   - Verify liquidity calculations

### Debug Tools
- Component-specific loggers
- Event replay capabilities
- State inspection methods
- Metric validation checks
