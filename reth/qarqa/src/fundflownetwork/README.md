# Fund Flow Network Module

This module analyzes Ethereum transactions to build visual networks showing how funds (ETH and ERC-20 tokens) flow between addresses.

## Table of Contents
1. [Core Problem](#core-problem)
2. [Two-Layer Architecture](#two-layer-architecture)
3. [Algorithm Design](#algorithm-design)
4. [Component Structure](#component-structure)
5. [Implementation Guide](#implementation-guide)
6. [Usage Examples](#usage-examples)

## Core Problem

Given a starting point (address or transaction), explore the graph of fund movements to understand:
- Where funds came from
- Where funds went
- Key intermediaries and endpoints
- Patterns of value movement

**The Challenge**: Ethereum has millions of addresses and billions of transactions. A popular address might have 100,000+ transactions. We need intelligent algorithms to explore this space efficiently.

**Performance Constraints**:
- DB Query: ~5-50ms per address lookup (indexed on tx_participants)
- Transaction Processing: ~15-40ms per tx (REVM simulation)
- Network Effects: Popular addresses can have 1000s of transactions
- Data Growth: O(b^d) where b=avg connections per node, d=depth

## Two-Layer Architecture

Our solution uses a two-layer approach to balance speed and depth:

### Layer 1: High-Level Graph Exploration (Participant Discovery)
- **Data source**: `tx_participants` and `addresses` tables (5-50ms queries)
- **Purpose**: Build UNDIRECTED graph by discovering ALL transaction participants
- **Key Algorithm**: 
  1. Start with seed address
  2. Get tx_hashes for address from tx_participants table
  3. For each tx_hash, get ALL participant addresses (not just main from/to)
  4. Create participant connection pairs (undirected edges)
  5. Apply routing rules to decide expansion (avoid CEX/dead ends)
  6. Repeat for discovered addresses up to max_depth
- **Operations**: 
  - Discover ALL addresses that participated in same transactions
  - Query entity types for routing decisions (CEX = stop, unknown = continue)
  - Build undirected participant graph (who was in same transactions)
  - Select transactions with unknown/interesting addresses for deep analysis
- **Output**:
  - UndirectedGraph with participant connections
  - Priority transaction list for Layer 2
  - **NOTE**: No fund flow directions, no values, no balance calculations - just participant relationships

### Layer 2: Deep Transaction Analysis  
- **Data source**: `tx_processor` with REVM simulation (15-40ms per tx)
- **Purpose**: Extract detailed DIRECTED fund flows with exact amounts
- **Operations**:
  - Process selected transactions to get all transfers
  - Extract ETH flows (direct, internal, gas)
  - Extract token transfers with amounts
  - Determine flow directions (from → to)
- **Output**:
  - Directed fund flows with exact amounts
  - All internal transfers and token movements
  - Gas costs and fees
  - **NOTE**: This layer determines actual flow directions and values

This allows exploring 1000s of connections quickly in Layer 1, then selectively analyzing 10s-100s in Layer 2.

## Algorithm Design

### High-Level Algorithm

```
ALGORITHM: Build Fund Flow Network
INPUT: seed (address or tx_hash), config (depth, min_value, max_nodes)
OUTPUT: FundFlowNetwork (graph with nodes and edges)

1. EXPLORATION PHASE (Layer 1)
   - Start from seed, query tx_participants
   - Build preliminary graph using BFS
   - Apply smart routing rules
   - Identify high-priority transactions
   
2. ANALYSIS PHASE (Layer 2)
   - Process priority transactions in parallel
   - Extract detailed fund flows
   - Identify patterns (swaps, arbitrage, etc.)
   
3. SYNTHESIS PHASE
   - Merge Layer 1 structure with Layer 2 details
   - Aggregate flows between same addresses
   - Calculate net balances
   - Mark intermediaries
   
4. EXPORT PHASE
   - Convert to visualization format
   - Add metadata and statistics
```

### Layer 1: Graph Exploration Algorithm

```
ALGORITHM: Explore Graph Structure (Layer 1)
INPUT: seed_address, exploration_config
OUTPUT: undirected_graph, priority_transactions

frontier = PriorityQueue()
visited = Set()
graph = UndirectedGraph()  // Key: This is UNDIRECTED
priority_txs = []

// Add seed node
seed_info = query_address_info(seed_address)
graph.add_node(seed_address, {
    entity_type: seed_info.entity_type,
    is_contract: seed_info.is_contract,
    explored: false,
    exploration_depth: 0
})

// Step 1: Get transaction hashes for seed address
seed_tx_hashes = query("
    SELECT DISTINCT tp.tx_hash, t.block_number 
    FROM tx_participants tp 
    JOIN transactions t ON t.tx_hash = tp.tx_hash
    WHERE tp.address_id = get_address_id(?)
      AND t.block_number <= ?
    ORDER BY t.block_number DESC 
    LIMIT ?
", seed_address, max_block_number, limit)

// Step 2: For each transaction, get ALL participants
for (tx_hash, block_number) in seed_tx_hashes:
    all_participants = query("
        SELECT DISTINCT a.address
        FROM tx_participants tp
        JOIN addresses a ON a.address_id = tp.address_id
        WHERE tp.tx_hash = ?
    ", tx_hash)
    
    // Step 3: Create participant pairs (seed ←→ other participants)
    for participant in all_participants:
        if participant != seed_address:
            frontier.push({
                tx_hash: tx_hash,
                from: seed_address,
                to: participant,
                counterparty: participant,
                value: 0,  // Phase 1 ignores values
                depth: 1,
                block_number: block_number
            })

// Mark seed as explored
graph.nodes[seed_address].explored = true

// BFS exploration using participant-based discovery
while not frontier.empty() and graph.node_count() < MAX_NODES:
    current = frontier.pop()
    
    if current.depth > MAX_DEPTH:
        continue
        
    if current.counterparty in visited:
        continue
    
    // Get address metadata for routing decisions
    address_info = query_address_info(current.counterparty)
    
    // Add node to graph
    graph.add_node(current.counterparty, {
        entity_type: address_info.entity_type,
        is_contract: address_info.is_contract,
        explored: false,
        exploration_depth: current.depth
    })
    
    // Add UNDIRECTED edge (participant connection via tx_hash)
    graph.add_edge({
        node1: current.from,
        node2: current.to,
        tx_hash: current.tx_hash,
        value: current.value,  // Zero in phase 1
        block_number: current.block_number
    })
    
    // Decide if transaction needs deep analysis (Layer 2)
    if should_analyze_deeply(address_info):
        priority_txs.append(current.tx_hash)
    
    // Apply routing rules to decide expansion
    should_expand = routing_rules.should_expand(address_info)
    
    if should_expand and current.depth < MAX_DEPTH:
        // Get transaction hashes for this address
        address_tx_hashes = get_address_transaction_hashes(current.counterparty, limit)
        
        for (tx_hash, block_number) in address_tx_hashes:
            // Get ALL participants in this transaction
            tx_participants = get_tx_participants(tx_hash)
            
            // Create pairs with all other participants
            for participant in tx_participants:
                if participant != current.counterparty and participant not in visited:
                    frontier.push({
                        tx_hash: tx_hash,
                        from: current.counterparty,
                        to: participant,
                        counterparty: participant,
                        value: 0,  // Phase 1 ignores values
                        depth: current.depth + 1,
                        block_number: block_number
                    })
        
        graph.nodes[current.counterparty].explored = true
    
    visited.add(current.counterparty)

return graph, priority_txs
```

#### Routing Rules (Address Expansion Logic)

```
FUNCTION should_expand(address_info) -> bool
    // Dead ends - don't expand exploration
    if address_info.entity_type in ['CEX', 'CEX_DEPOSIT', 'BURN', 'NULL', 'BLACKHOLE']:
        return false
    
    // Routers and bridges - expand to trace through them
    if address_info.entity_type in ['DEX_ROUTER', 'DEX_AGGREGATOR', 'BRIDGE']:
        return true
    
    // High-value targets - always expand
    if address_info.entity_type in ['WHALE', 'PROTOCOL_TREASURY', 'LENDING_POOL']:
        return true
    
    // MEV related - important to track
    if address_info.entity_type in ['MEV_BOT', 'FLASHLOAN_PROVIDER']:
        return true
    
    // Default behavior for unknown addresses
    if address_info.entity_type is null:
        if address_info.is_contract:
            return true  // Unknown contracts worth investigating
        else:
            return true  // EOAs generally worth exploring
    
    return true  // Default: expand unless explicitly blocked

FUNCTION should_analyze_deeply(from_info, to_info, value) -> bool
    // Always analyze high-value transactions (> 10 ETH)
    if value > 10_ETH:
        return true
    
    // Always analyze if either party is unknown
    if from_info.entity_type is null or to_info.entity_type is null:
        return true
    
    // Router transactions need deep analysis to find actual flows
    if is_router(from_info) or is_router(to_info):
        return true
    
    // MEV transactions are always interesting
    if is_mev_related(from_info) or is_mev_related(to_info):
        return true
    
    // Transactions between CEX and unknown addresses
    if (is_cex(from_info) and to_info.entity_type is null) or
       (from_info.entity_type is null and is_cex(to_info)):
        return true
    
    // Medium value (> 1 ETH) involving protocols
    if value > 1_ETH and (is_protocol(from_info) or is_protocol(to_info)):
        return true
    
    return false
```

### Layer 2: Deep Analysis Algorithm

```
ALGORITHM: Process Priority Transactions
INPUT: priority_tx_list, tx_processor
OUTPUT: enriched_fund_flows

BATCH_SIZE = 10
PARALLEL_WORKERS = 4

// Process in parallel batches
batches = chunk(priority_tx_list, BATCH_SIZE)
all_flows = []

for batch in batches:
    batch_results = parallel_process(batch, PARALLEL_WORKERS, async (tx_hash) => {
        try:
            // Get detailed transaction data
            processed_tx = await tx_processor.process_transaction(tx_hash)
            
            // Extract all fund movements
            fund_flows = extract_fund_flows(processed_tx)
            
            // Identify patterns
            patterns = identify_patterns(processed_tx)
            
            return {
                tx_hash: tx_hash,
                fund_flows: fund_flows,
                patterns: patterns,
                total_value_usd: calculate_total_value(fund_flows)
            }
        } catch (e) {
            log_error("Failed to process tx", tx_hash, e)
            return null
        }
    })
    
    all_flows.extend(filter_nulls(batch_results))

return all_flows
```

#### Fund Flow Extraction

```
FUNCTION extract_fund_flows(processed_tx) -> FundFlows
    flows = FundFlows()
    
    // 1. Direct ETH transfer
    if processed_tx.value > 0:
        flows.add_eth_flow(
            from: processed_tx.from,
            to: processed_tx.to,
            amount: processed_tx.value,
            type: 'DIRECT'
        )
    
    // 2. Internal ETH transfers
    for internal_tx in processed_tx.internal_transactions:
        flows.add_eth_flow(
            from: internal_tx.from,
            to: internal_tx.to,
            amount: internal_tx.value,
            type: 'INTERNAL'
        )
    
    // 3. ERC20 transfers
    for token_transfer in processed_tx.erc20_transfers:
        flows.add_token_flow(
            from: token_transfer.from,
            to: token_transfer.to,
            token: token_transfer.token_address,
            amount: token_transfer.amount,
            usd_value: get_token_usd_value(token_transfer)
        )
    
    // 4. Gas payment
    gas_cost = processed_tx.gas_used * processed_tx.gas_price
    flows.add_eth_flow(
        from: processed_tx.from,
        to: ZERO_ADDRESS,  // Represents miner/validator
        amount: gas_cost,
        type: 'GAS'
    )
    
    return flows
```

### Network Synthesis Algorithm

```
ALGORITHM: Synthesize Final Network
INPUT: preliminary_graph, enriched_flows
OUTPUT: fund_flow_network

network = FundFlowNetwork()

// 1. Add all nodes from preliminary graph
for node in preliminary_graph.nodes:
    network.add_node(node.address, metadata=node.metadata)

// 2. Process enriched flows
flow_aggregator = FlowAggregator()

for enriched_tx in enriched_flows:
    for flow in enriched_tx.fund_flows:
        // Aggregate flows between same addresses
        flow_aggregator.add_flow(flow)
        
        // Add new addresses discovered in deep analysis
        if not network.has_node(flow.from):
            network.add_node(flow.from)
        if not network.has_node(flow.to):
            network.add_node(flow.to)

// 3. Create edges from aggregated flows
for aggregated_flow in flow_aggregator.get_aggregated_flows():
    if aggregated_flow.total_value >= MIN_EDGE_VALUE:
        network.add_edge(
            from: aggregated_flow.from,
            to: aggregated_flow.to,
            value_eth: aggregated_flow.eth_amount,
            value_usd: aggregated_flow.usd_value,
            tx_count: aggregated_flow.tx_count,
            first_block: aggregated_flow.first_block,
            last_block: aggregated_flow.last_block
        )

// 4. Calculate network properties
for node in network.nodes:
    node.net_balance = calculate_net_balance(node, network)
    node.is_intermediary = (abs(node.net_balance) < ZERO_THRESHOLD)

network.calculate_statistics()

return network
```

## Component Structure

### Core Components

```
fundflownetwork/
├── src/
│   ├── lib.rs                          # Public API
│   ├── network_builder.rs              # Main orchestrator
│   │
│   ├── exploration/                    # Layer 1 components
│   │   ├── mod.rs
│   │   ├── graph_explorer.rs          # BFS exploration logic
│   │   ├── address_evaluator.rs       # Routing rules
│   │   └── tx_prioritizer.rs          # Transaction scoring
│   │
│   ├── analysis/                       # Layer 2 components
│   │   ├── mod.rs
│   │   ├── tx_processor_client.rs     # Interface to tx_processor
│   │   ├── flow_extractor.rs          # Extract fund flows
│   │   └── pattern_detector.rs        # Identify patterns
│   │
│   ├── synthesis/                      # Integration components
│   │   ├── mod.rs
│   │   ├── flow_aggregator.rs         # Aggregate transfers
│   │   ├── network_constructor.rs     # Build final network
│   │   └── balance_calculator.rs      # Net balance calculation
│   │
│   ├── types/                          # Data structures
│   │   ├── mod.rs
│   │   ├── network.rs                 # Network, Node, Edge types
│   │   ├── flows.rs                   # FundFlow types
│   │   └── config.rs                  # Configuration types
│   │
│   ├── storage/                        # Caching and persistence
│   │   ├── mod.rs
│   │   ├── cache.rs                   # Multi-level cache
│   │   └── db_queries.rs              # Optimized queries
│   │
│   └── export/                         # Visualization formats
│       ├── mod.rs
│       ├── cytoscape.rs               # Cytoscape.js format
│       ├── visjs.rs                   # Vis.js format
│       └── graphml.rs                 # GraphML format
│
├── examples/
│   ├── basic_usage.rs                 # Simple example
│   ├── build_fund_flow_network.rs     # Complete workflow
│   └── interactive_exploration.rs     # Interactive mode
│
└── tests/
    ├── integration_tests.rs
    └── performance_tests.rs
```

### Component Responsibilities

#### 1. NetworkBuilder (Orchestrator)
```rust
pub struct NetworkBuilder {
    explorer: GraphExplorer,
    analyzer: TxAnalyzer,
    synthesizer: NetworkSynthesizer,
    cache: NetworkCache,
}

impl NetworkBuilder {
    pub async fn build_from_address(&self, address: Address, config: Config) -> Result<Network> {
        // Layer 1: Explore
        let (graph, priority_txs) = self.explorer.explore(address, config).await?;
        
        // Layer 2: Analyze
        let flows = self.analyzer.analyze_transactions(priority_txs).await?;
        
        // Synthesize
        let network = self.synthesizer.build_network(graph, flows)?;
        
        Ok(network)
    }
}
```

#### 2. GraphExplorer (Layer 1)
```rust
pub struct GraphExplorer {
    db_pool: PgPool,
    evaluator: AddressEvaluator,
    prioritizer: TxPrioritizer,
}

impl GraphExplorer {
    pub async fn explore(&self, seed: Address, config: ExplorationConfig) 
        -> Result<(PreliminaryGraph, Vec<TxHash>)> {
        // Implements Layer 1 algorithm
    }
}
```

#### 3. TxAnalyzer (Layer 2)
```rust
pub struct TxAnalyzer {
    tx_processor: TxProcessorClient,
    flow_extractor: FlowExtractor,
    pattern_detector: PatternDetector,
}

impl TxAnalyzer {
    pub async fn analyze_transactions(&self, tx_hashes: Vec<TxHash>) 
        -> Result<Vec<EnrichedFlow>> {
        // Implements Layer 2 algorithm with parallel processing
    }
}
```

#### 4. NetworkSynthesizer (Integration)
```rust
pub struct NetworkSynthesizer {
    aggregator: FlowAggregator,
    constructor: NetworkConstructor,
    calculator: BalanceCalculator,
}

impl NetworkSynthesizer {
    pub fn build_network(&self, graph: PreliminaryGraph, flows: Vec<EnrichedFlow>) 
        -> Result<FundFlowNetwork> {
        // Implements synthesis algorithm
    }
}
```

## Implementation Guide

### Phase 1: Core Infrastructure
1. Set up types and data structures
2. Implement basic graph explorer with tx_participants queries
3. Create tx_processor client wrapper
4. Build simple network constructor

### Phase 2: Smart Exploration
1. Implement address evaluation rules
2. Add transaction prioritization
3. Create value-weighted BFS
4. Add entity-aware pruning

### Phase 3: Deep Analysis
1. Implement parallel transaction processing
2. Add fund flow extraction
3. Create pattern detection
4. Build flow aggregation

### Phase 4: Optimization
1. Add multi-level caching
2. Implement batch queries
3. Add progressive loading
4. Create background pre-fetching

### Phase 5: Interactive Features
1. Add node expansion API
2. Implement filter updates
3. Create real-time updates
4. Add export formats

## Usage Examples

### Basic Usage
```rust
use qarqa_fundflownetwork::{NetworkBuilder, Config};

// Create builder
let builder = NetworkBuilder::new(db_pool, tx_processor);

// Build network from address
let config = Config {
    max_depth: 3,
    min_value_eth: 0.1,
    max_nodes: 500,
    include_gas: true,
};

let network = builder
    .build_from_address(address, config)
    .await?;

// Export for visualization
let json = network.export_cytoscape()?;
```

### Interactive Exploration
```rust
// Start with small network
let mut network = builder
    .build_from_address(address, small_config)
    .await?;

// User expands a node
network = builder
    .expand_node(&network, selected_address)
    .await?;

// Apply filters
network = network
    .filter_by_value(min_eth: 1.0)
    .filter_by_time(last_30_days);
```

### Advanced Analysis
```rust
// Find arbitrage cycles
let cycles = network.find_cycles()
    .filter(|c| c.is_profitable());

// Identify fund sources
let sources = network.trace_fund_sources(address)
    .limit(10);

// Export subgraph
let subgraph = network.extract_subgraph(
    center: address,
    depth: 2,
    min_value: 0.5
);
```