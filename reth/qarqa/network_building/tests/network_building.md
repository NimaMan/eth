# Network Building Testing Documentation

## Overview

The `network_building` module constructs fund flow networks from analyzed transactions, creating graph representations for visualization and analysis. This document specifies comprehensive testing for graph construction, layout algorithms, and visualization formats.

## What We Are Testing

### 1. Graph Construction

#### Node Creation
- **Address deduplication**: Each address appears once
- **Node attributes**: Correct labels, types, and metadata
- **State aggregation**: Combined balance changes
- **Entity classification**: EOA vs Contract detection
- **Risk scoring**: Accurate risk level assignment

#### Edge Creation
- **Flow aggregation**: Multiple transfers combined
- **Direction handling**: Proper source/target assignment
- **Weight calculation**: Accurate total values
- **Edge attributes**: Transfer count, types, timing
- **Self-edges**: Handle A → A transfers

### 2. Network Analysis

#### Graph Metrics
- **Degree centrality**: In-degree and out-degree
- **Betweenness centrality**: Intermediary importance
- **Clustering coefficient**: Network density
- **Connected components**: Subgraph detection
- **Path finding**: Shortest paths between nodes

#### Pattern Detection
- **Hub detection**: High-degree nodes
- **Mixing services**: Obfuscation patterns
- **Circular flows**: Money laundering detection
- **Bridge nodes**: Key connectors
- **Isolated clusters**: Separate fund networks

### 3. Layout Algorithms

#### Force-Directed Layout
- **Convergence**: Layout stabilizes
- **Node spacing**: No overlapping nodes
- **Edge routing**: Clear edge paths
- **Performance**: < 1s for 1000 nodes
- **Deterministic**: Consistent results

#### Hierarchical Layout
- **Level assignment**: Proper node hierarchy
- **Minimized crossings**: Clean visualization
- **Compact representation**: Space efficiency
- **Directional flow**: Top-to-bottom/left-to-right
- **Subgraph handling**: Nested layouts

### 4. Visualization Formats

#### Cytoscape.js Format
- **Node data structure**: Required fields present
- **Edge data structure**: Source/target/weight
- **Style attributes**: Colors, sizes, labels
- **Metadata preservation**: All custom fields
- **JSON validity**: Proper serialization

#### GraphML Export
- **XML structure**: Valid GraphML schema
- **Attribute encoding**: Proper type handling
- **Unicode support**: International addresses
- **Large graphs**: Streaming for big networks
- **Import compatibility**: Works with Gephi/yEd

### 5. Performance and Scalability

#### Large Networks
- **1,000 nodes**: < 100ms construction
- **10,000 nodes**: < 1s construction
- **100,000 nodes**: < 10s with streaming
- **Memory usage**: Linear growth
- **Query performance**: Efficient lookups

#### Real-time Updates
- **Incremental building**: Add new flows
- **Node updates**: Merge new state
- **Edge updates**: Aggregate values
- **Layout updates**: Partial recalculation
- **Streaming output**: Progressive rendering

## How We Test It

### Unit Tests for Core Components

```rust
#[test]
fn test_node_deduplication() {
    // Arrange
    let fund_flows = vec![
        create_fund_flow("0xA", "0xB", 1.0),
        create_fund_flow("0xB", "0xC", 0.5),
        create_fund_flow("0xA", "0xC", 0.3),
    ];
    
    // Act
    let builder = NetworkBuilder::new();
    let network = builder.build_from_fund_flows(&fund_flows, None).unwrap();
    
    // Assert
    assert_eq!(network.nodes.len(), 3); // A, B, C
    assert_eq!(network.edges.len(), 3);
    
    // Verify node attributes
    let node_a = network.nodes.iter().find(|n| n.address == "0xA").unwrap();
    assert_eq!(node_a.total_out_value, 1.3); // 1.0 + 0.3
}
```

### Integration Tests with Real Data

```rust
#[tokio::test]
async fn test_complex_defi_network() {
    // Arrange - Load real transaction network
    let fund_flows = load_test_transaction_flows(
        "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006"
    ).await;
    
    // Act
    let config = NetworkBuilderConfig::default()
        .with_min_value_eth(0.01)
        .with_combine_weth(true);
    
    let builder = NetworkBuilder::with_config(config);
    let network = builder.build_from_fund_flows(&fund_flows, None).unwrap();
    
    // Assert
    assert!(network.nodes.len() >= 5); // Complex transaction
    assert!(network.edges.len() >= 8);
    
    // Verify no duplicate nodes
    let addresses: HashSet<_> = network.nodes.iter()
        .map(|n| &n.address)
        .collect();
    assert_eq!(addresses.len(), network.nodes.len());
}
```

### Layout Algorithm Tests

```rust
#[test]
fn test_force_directed_layout_convergence() {
    // Arrange
    let network = create_test_network(100); // 100 nodes
    let mut layout = ForceDirectedLayout::new();
    
    // Act
    let start = Instant::now();
    let positions = layout.calculate(&network);
    let duration = start.elapsed();
    
    // Assert
    assert!(duration < Duration::from_secs(1));
    assert_eq!(positions.len(), network.nodes.len());
    
    // Verify no overlapping nodes
    for (i, pos1) in positions.iter().enumerate() {
        for (j, pos2) in positions.iter().enumerate() {
            if i != j {
                let distance = ((pos1.x - pos2.x).powi(2) + 
                               (pos1.y - pos2.y).powi(2)).sqrt();
                assert!(distance > MIN_NODE_DISTANCE);
            }
        }
    }
}
```

### Visualization Format Tests

```rust
#[test]
fn test_cytoscape_format_validity() {
    // Arrange
    let network = create_sample_network();
    
    // Act
    let cytoscape_data = convert_to_visualization_data(
        &network,
        Some(&create_positions(&network)),
        NetworkCurrency::USD
    );
    
    // Assert
    // Verify JSON structure
    let json = serde_json::to_value(&cytoscape_data).unwrap();
    assert!(json["nodes"].is_array());
    assert!(json["edges"].is_array());
    assert!(json["stats"].is_object());
    
    // Verify required fields
    for node in json["nodes"].as_array().unwrap() {
        assert!(node["data"]["id"].is_string());
        assert!(node["data"]["label"].is_string());
        assert!(node["position"]["x"].is_number());
        assert!(node["position"]["y"].is_number());
    }
}
```

## Test Cases

### Network Construction Tests

1. **Simple Networks**
   - Linear chain: A → B → C
   - Star pattern: Central hub
   - Circular: A → B → C → A
   - Disconnected: Multiple components
   - Single node: No edges

2. **Complex Networks**
   - DeFi protocol interaction
   - Token distribution network
   - Exchange deposit/withdrawal
   - Mixing service patterns
   - Flash loan networks

3. **Edge Cases**
   - Self-transfers (A → A)
   - Bidirectional flows
   - Zero-value edges
   - Duplicate edges
   - Very large values

### Analysis Algorithm Tests

1. **Centrality Metrics**
   - Degree centrality ranking
   - Betweenness on paths
   - Eigenvector centrality
   - PageRank scores
   - Closeness centrality

2. **Pattern Detection**
   - Triangle detection
   - Cycle enumeration
   - Bridge identification
   - Community detection
   - Anomaly scoring

3. **Path Finding**
   - Shortest path
   - All paths enumeration
   - Maximum flow
   - Minimum cut
   - Reachability queries

### Visualization Tests

1. **Layout Quality**
   - Node overlap check
   - Edge crossing minimization
   - Aspect ratio optimization
   - Label placement
   - Hierarchical organization

2. **Format Conversion**
   - JSON serialization
   - GraphML export
   - CSV edge list
   - DOT format
   - Custom formats

3. **Performance Scaling**
   - 10 nodes: < 1ms
   - 100 nodes: < 10ms
   - 1,000 nodes: < 100ms
   - 10,000 nodes: < 1s
   - 100,000 nodes: < 10s

## Test Data

### Sample Networks
```rust
pub fn create_test_networks() -> Vec<FundFlowNetwork> {
    vec![
        // Simple transfer chain
        create_chain_network(5),
        
        // Hub and spoke
        create_hub_network(10),
        
        // Complex DeFi
        create_defi_network(),
        
        // Mixing pattern
        create_mixer_network(20),
    ]
}
```

### Real Transaction Networks
```rust
pub const NETWORK_TEST_CASES: &[(&str, usize, usize)] = &[
    // (tx_hash, expected_nodes, expected_edges)
    ("0x5c504...", 2, 1),    // Simple transfer
    ("0xf7bd6...", 8, 15),   // Complex DeFi
    ("0x7b944...", 5, 8),    // Token swap
    ("0x434b5...", 3, 2),    // Contract creation
];
```

## Expected Test Outcomes

### Construction Accuracy
- Node count matches unique addresses
- Edge count matches unique flows
- Total value conservation
- Proper direction assignment
- Accurate metadata

### Analysis Correctness
- Centrality metrics within bounds
- Valid path finding results
- Correct pattern detection
- Proper component separation
- Accurate risk scores

### Visualization Quality
- Valid JSON/XML output
- No rendering artifacts
- Readable layouts
- Proper scaling
- Complete data export

### Performance Benchmarks
- Construction: O(n + m) complexity
- Layout: O(n²) worst case
- Serialization: O(n + m)
- Memory: O(n + m) usage
- No memory leaks

## Running the Tests

```bash
# Run all network_building tests
cargo test -p qarqa-network-building

# Run layout algorithm tests
cargo test -p qarqa-network-building layout -- --nocapture

# Run performance benchmarks
cargo bench -p qarqa-network-building

# Run with visualization output
SAVE_TEST_OUTPUTS=1 cargo test -p qarqa-network-building

# Python tests for network logic
cd network_building/tests && python -m pytest
```

## Visualization Debugging

### Save Test Networks
```rust
#[test]
fn test_with_output() {
    let network = build_test_network();
    
    // Save for manual inspection
    if std::env::var("SAVE_TEST_OUTPUTS").is_ok() {
        let json = serde_json::to_string_pretty(&network).unwrap();
        std::fs::write("test_network.json", json).unwrap();
        
        let graphml = network.to_graphml().unwrap();
        std::fs::write("test_network.graphml", graphml).unwrap();
    }
}
```

### Visualization Tools
- Cytoscape desktop for network inspection
- Gephi for large network analysis
- D3.js for web visualization
- GraphViz for automated layouts
- Python NetworkX for verification

## Test Maintenance

- Add new pattern tests monthly
- Update real transaction tests
- Profile layout performance
- Verify visualization compatibility
- Document any visual artifacts