# LiveToken System Architecture

## Overview

The LiveToken system provides real-time tracking and analysis of ERC20 tokens on Ethereum. It processes blockchain transactions to maintain comprehensive token state, analyze trading networks, and assess token health through pattern detection.

## Core Components

### 1. LiveERC20Token (Main Facade)

The primary interface that orchestrates all subsystems:

```python
class LiveERC20Token:
    def __init__(self, contract_address: str):
        self.token_data = LiveTokenData(contract_address)
        self.token_network = LiveTokenNetwork(self)
        self.token_health = TokenHealthPredictor()
```

### 2. LiveTokenData

The core data management component that maintains all token-related information and processes blockchain events.

#### 2.1 Token Metadata

Stores fundamental token information:
- **Basic Info**: `name`, `symbol`, `decimals`, `total_supply`
- **Creation Details**: `creation_block`, `creation_timestamp`, `creator_address`, `creator_nonce`
- **Trading State**: `trading_enabled`, `trading_enabled_block`, `token_status`

#### 2.2 Event Collections

Maintains categorized blockchain events with transaction-level grouping (keeping the latest 1K events for each type):

##### 2.2.1 Transfer Events
```python
# ERC20 token transfers
erc20_transfers: Dict[str, List[TransferEvent]]  # txn_hash -> transfers

# ETH transfers (internal transactions)
eth_transfers: Dict[str, List[EthTransferEvent]]  # txn_hash -> eth_transfers

# Liquidity token transfers
liquidity_token_transfers: Dict[str, List[LPTransferEvent]]  # txn_hash -> lp_transfers

# Other denomination token transfers (USDC, USDT, etc.)
other_denom_transfers: Dict[str, List[DenomTransferEvent]]  # txn_hash -> denom_transfers
```

##### 2.2.2 Approval Events
```python
approvals: List[ApprovalEvent]  # All approval events
approved_addresses: Set[str]    # Unique approved addresses
```

#### 2.3 DEX Integration Components

The system supports multiple DEX protocols with version-specific handlers:

##### 2.3.1 Uniswap V2 Components

**Event Handlers**:
- `PairEventHandler`: Processes pair creation events
- `SyncEventHandler`: Tracks liquidity pool synchronization
- `SwapEventHandler`: Records swap transactions
- `MintEventHandler`: Tracks liquidity additions
- `BurnEventHandler`: Tracks liquidity removals

**Data Structures**:
```python
univ2_pairs: List[PairCreationEvent]      # Pool creation events
univ2_syncs: List[SyncEvent]              # Price/reserve updates
univ2_swaps: Dict[str, List[SwapEvent]]   # txn_hash -> swaps
univ2_mints: List[MintEvent]              # Liquidity additions
univ2_burns: List[BurnEvent]              # Liquidity removals
```

##### 2.3.2 Uniswap V3 Components

**Event Handlers**:
- `PoolEventHandler`: Processes pool creation with fee tiers
- `SwapEventHandler`: Handles concentrated liquidity swaps
- `MintEventHandler`: Tracks position minting
- `BurnEventHandler`: Tracks position burning
- `CollectEventHandler`: Fee collection tracking

**Data Structures**:
```python
univ3_pools: List[PoolCreationEvent]      # Pool creation with fee tier
univ3_swaps: Dict[str, List[SwapEventV3]] # Enhanced swap data
univ3_mints: List[MintEventV3]            # Position-based mints
univ3_burns: List[BurnEventV3]            # Position-based burns
univ3_positions: Dict[int, PositionData]   # NFT position tracking
```

##### 2.3.3 Uniswap V4 Components

**Event Handlers**:
- `InitializeEventHandler`: Hook-enabled pool initialization
- `SwapEventHandler`: Handles hook-integrated swaps
- `ModifyPositionHandler`: Position modifications
- `DonateEventHandler`: Direct liquidity donations

**Data Structures**:
```python
univ4_pools: List[PoolInitEvent]          # Hook-enabled pools
univ4_swaps: Dict[str, List[SwapEventV4]] # Hook-aware swaps
univ4_positions: Dict[bytes32, Position]   # Position tracking
univ4_hooks: Dict[str, HookData]          # Active hooks
```

#### 2.4 Pool Management System

##### 2.4.1 Pool Information Registry
```python
pool_info: Dict[str, PoolInfo]
# PoolInfo contains:
# - pool_type: "V2", "V3", "V4"
# - denom_address: Address of paired token (WETH, USDC, etc.)
# - denom_currency: Human-readable name
# - decimals: Decimal places for denom token
# - token1_is_denom: Boolean for token ordering
# - lp_token_info: LP token metadata (V2 only)
```

##### 2.4.2 Pool State Tracking
```python
pool_prices: OrderedDict[str, List[float]]    # Historical prices
pool_reserves: OrderedDict[str, ReserveData]  # Current reserves

# ReserveData contains:
# - denom_reserve: Current denomination token amount
# - token_reserve: Current token amount
# - denom_reserve_changes: Historical changes
# - token_reserve_changes: Historical changes
```

#### 2.5 Ownership & Control

Tracks token ownership and control changes:
```python
owner_events: List[OwnershipEvent]  # Ownership transfers
current_owner: Optional[str]        # Current owner address
all_owners: List[str]              # Historical owners
ownership_renounced: bool          # Renouncement status
```

#### 2.6 Scam Detection Data

Maintains data for scam pattern detection:
```python
is_scam: bool                      # Scam status
scam_label: Optional[str]          # Reason for scam classification
scam_block: Optional[int]          # Block where scam detected
scam_txn: Optional[str]           # Transaction triggering detection

# Bribe tracking
total_bribe_amount: float          # Total bribes paid
bribe_amount_dict: Dict[str, float] # Address -> bribe amount
```

### 3. LiveTokenNetwork

Analyzes transaction networks and relationships between addresses.

#### 3.1 Network Builder

Constructs directed graph from token transfers:
```python
class LiveTokenNetworkBuilder:
    def build_graph(self) -> nx.DiGraph:
        # Creates nodes for each address
        # Adds edges for transfers
        # Enriches nodes with UserActivityData
```

#### 3.2 User Activity Tracking

For each address node:
```python
class UserActivityData:
    address: str
    token_balance: float
    denom_balance: float
    realized_profit: float
    unrealized_profit: float
    total_transfers_in: int
    total_transfers_out: int
    unique_counterparties: Set[str]
```

#### 3.3 Subgraph Analysis

Identifies connected components and relationships:
```python
class NetworkSubgraphAnalyzer:
    def find_subgraphs(self) -> List[Set[str]]
    def get_related_addresses(self, address: str) -> Set[str]
    def simplify_graph(self) -> nx.DiGraph
```

#### 3.4 Aggregated Metrics

Computes collective metrics for related addresses:
- `agg_token_balance`: Combined token holdings
- `agg_denom_balance`: Combined ETH/USDC holdings
- `agg_realized_profit`: Total realized profits
- `agg_unrealized_profit`: Total unrealized profits

### 4. TokenHealthPredictor

Assesses token health through pattern analysis.

#### 4.1 Volume Analyzer

Detects deceptive volume patterns:
```python
class VolumeAnalyzer:
    def analyze_transfer_patterns(self, transaction: Dict) -> VolumePattern
    def detect_wash_trading(self, transfers: List[Transfer]) -> bool
    def identify_fake_volume(self, swaps: List[Swap]) -> float
```

#### 4.2 Actor Classification

Categorizes addresses by behavior:
- **Green Actors**: Known legitimate addresses (CEX, verified contracts)
- **Grey Actors**: Suspicious or known malicious addresses
- **Neutral**: Unclassified addresses

#### 4.3 Scam Scoring System

Multi-signal detection with confidence levels:
```python
@dataclass
class ScamScore:
    block_number: int
    transaction_hash: str
    is_scam: bool
    confidence: float  # 0.0 to 1.0
    reason: str
```

Detection signals include:
- Deceptive transfer patterns (>15 transfers or >20 addresses)
- Malicious actor involvement
- Reserve depletion patterns
- Hidden mint detection

## Data Processing Flow

### Transaction Processing Pipeline

1. **Transaction Receipt** → `LiveERC20Token.update_from_transaction()`
2. **Event Extraction** → Parse logs for relevant events
3. **Token Data Update**:
   - Process transfers and update balances
   - Update pool states from DEX events
   - Check scam patterns
4. **Network Update**:
   - Add/update graph edges
   - Recalculate user metrics
   - Detect new relationships
5. **Health Assessment**:
   - Analyze volume patterns
   - Check actor involvement
   - Update scam scores

### Event Processing Hierarchy

```
Transaction
├── Contract Creation Events
│   └── Initialize token metadata
├── DEX Events
│   ├── Pair/Pool Creation
│   ├── Liquidity Events (Mint/Burn)
│   ├── Swap Events
│   └── Sync Events (V2 only)
├── Transfer Events
│   ├── ERC20 Transfers
│   ├── ETH Transfers
│   └── LP Token Transfers
├── Control Events
│   ├── Ownership Changes
│   └── Approval Events
└── Custom Events
    ├── Trading Enabled
    └── Tax/Limit Updates
```

## Key Algorithms

### Price Calculation

**Uniswap V2**:
```python
price = denom_reserve / token_reserve
```

**Uniswap V3/V4**:
```python
price = (sqrtPriceX96 / 2^96)^2
```

### Scam Detection

**Reserve Monitoring**:
- WETH pools: Alert if reserve < 0.01 ETH
- Stablecoin pools: Alert if reserve < 100 USDC/USDT
- Token supply: Alert if circulating > total supply

**Pattern Detection**:
- Wash trading: Circular transfers within same block
- Fake volume: Transfers to known addresses without swaps
- Rug pull: Sudden liquidity removal by large holders

### Network Analysis

**Related Address Detection**:
1. Build transfer graph
2. Find strongly connected components
3. Aggregate metrics for components
4. Identify high-value clusters

## Data Persistence

The system maintains in-memory state with:
- Event history (bounded collections)
- Current balances and reserves
- Network graph structure
- Health assessment scores

External systems can query current state through properties and methods exposed by LiveERC20Token.

## Performance Considerations

- **Memory Management**: Bounded collections prevent unlimited growth
- **Indexing**: Transaction hash indexing for O(1) event lookup
- **Caching**: Pool reserves cached to reduce RPC calls
- **Batch Processing**: Multiple events per transaction processed together

## Integration Points

The system integrates with:
- **Ethereum Node**: Via Web3 for blockchain data
- **Database**: For persistent storage (via external adapters)
- **Alert Systems**: For scam detection notifications
- **Analytics Platforms**: For aggregate metrics and reporting