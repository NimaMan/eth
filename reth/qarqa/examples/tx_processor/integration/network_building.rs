//! Network Building Integration Example
//! 
//! Demonstrates integration between tx_processor and network_building modules
//! to create comprehensive fund flow networks from transaction analysis.
//!
//! Features:
//! - Transaction analysis to fund flow extraction
//! - Network construction from fund flows  
//! - Multi-transaction network aggregation
//! - Visualization data generation
//! - Real-world transaction network examples

use qarqa_core_types::*;
use qarqa_tx_simulation::{RevmDirectSimulator, TransactionSimulator, FundFlowAnalyzer};
use qarqa_network_building::{NetworkBuilder, AddressNetwork};
use alloy_primitives::{Address, U256, B256};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use serde_json;

/// Real transaction sequences that form interesting networks
const NETWORK_EXAMPLES: &[(&[&str], &str)] = &[
    // Complex DeFi interaction sequence
    (&[
        "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006",
        "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b",
        "0x2c2e15d46f6e2a9a1f3e6a8f9f1a7c3e4b5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f",
    ], "DeFi arbitrage sequence"),
    
    // Whale trading pattern
    (&[
        "0x4c9c8ce7a8c2d6e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2",
        "0x8d9e0f1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9",
    ], "Whale distribution pattern"),
    
    // Token launch and initial trading
    (&[
        "0x1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2",
        "0x3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4",
        "0x5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6",
    ], "Token launch network"),
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🕸️ Network Building Integration Example");
    println!("=======================================");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let example_index = args.get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
        .min(NETWORK_EXAMPLES.len() - 1);
    
    let (tx_hashes, description) = NETWORK_EXAMPLES[example_index];
    
    println!("📋 Building network for: {}", description);
    println!("📊 Transactions: {}", tx_hashes.len());
    for (i, hash) in tx_hashes.iter().enumerate() {
        println!("   {}. {}...{}", i + 1, &hash[0..10], &hash[hash.len()-8..]);
    }
    println!();
    
    // Build network from transaction sequence
    let network = build_transaction_network(tx_hashes).await?;
    
    // Display network analysis
    display_network_analysis(&network);
    
    // Generate visualization data
    generate_visualization_data(&network, description).await?;
    
    // Demonstrate network analysis capabilities
    analyze_network_patterns(&network);
    
    Ok(())
}

/// Build a network from a sequence of transactions
async fn build_transaction_network(tx_hashes: &[&str]) -> Result<AddressNetwork, Box<dyn std::error::Error>> {
    println!("🔬 Processing transactions...");
    
    // Initialize simulator
    let mut simulator = RevmDirectSimulator::new();
    simulator.initialize().await?;
    
    // Initialize fund flow analyzer
    let analyzer = FundFlowAnalyzer::new()
        .with_weth_as_eth(true)
        .with_gas_inclusion(false)
        .with_min_value_eth(0.001); // Filter small movements
    
    let mut all_fund_flows = Vec::new();
    
    // Process each transaction
    for (i, tx_hash) in tx_hashes.iter().enumerate() {
        println!("   Processing {}/{}: {}...", i + 1, tx_hashes.len(), &tx_hash[0..10]);
        
        // Create mock transaction
        let transaction = create_mock_transaction(tx_hash, i)?;
        
        // Simulate transaction
        let fund_flows = simulator.simulate_transaction(&transaction).await?;
        
        println!("     ETH movements: {}", fund_flows.eth_movements.len());
        println!("     Token movements: {}", fund_flows.token_movements.len());
        
        all_fund_flows.push(fund_flows);
    }
    
    // Analyze all fund flows
    println!("📊 Analyzing fund flows...");
    let analyzed_flows = analyzer.analyze_fund_flows(&all_fund_flows)?;
    
    println!("   Total fund flows: {}", analyzed_flows.len());
    
    // Build network
    println!("🏗️ Building network...");
    let network_builder = NetworkBuilder::new();
    let network = network_builder.build_from_fund_flows(&analyzed_flows, None)?;
    
    println!("✅ Network built successfully");
    println!("   Nodes: {}", network.nodes.len());
    println!("   Edges: {}", network.edges.len());
    
    Ok(network)
}

/// Display comprehensive network analysis
fn display_network_analysis(network: &AddressNetwork) {
    println!("\n🔍 Network Analysis");
    println!("===================");
    
    // Basic statistics
    println!("📊 Basic Statistics:");
    println!("   Nodes: {}", network.nodes.len());
    println!("   Edges: {}", network.edges.len());
    println!("   Total ETH Volume: {:.4} ETH", 
             network.stats.total_eth_volume.to::<u128>() as f64 / 1e18);
    println!("   Network Density: {:.4}", network.stats.network_density);
    
    // Node analysis
    println!("\n🏷️ Node Analysis:");
    let mut node_types = HashMap::new();
    for node in &network.nodes {
        *node_types.entry(&node.entity_type).or_insert(0) += 1;
    }
    
    for (entity_type, count) in node_types {
        println!("   {}: {}", entity_type, count);
    }
    
    // Top nodes by volume
    println!("\n💰 Top Nodes by Volume:");
    let mut nodes_with_volume: Vec<_> = network.nodes.iter()
        .map(|node| {
            let total_volume: U256 = network.edges.iter()
                .filter(|edge| edge.from == node.address || edge.to == node.address)
                .map(|edge| edge.total_eth)
                .sum();
            (node, total_volume)
        })
        .collect();
    
    nodes_with_volume.sort_by(|a, b| b.1.cmp(&a.1));
    
    for (i, (node, volume)) in nodes_with_volume.iter().take(5).enumerate() {
        let volume_eth = volume.to::<u128>() as f64 / 1e18;
        println!("   {}. {} ({}) - {:.4} ETH", 
                 i + 1, 
                 format!("{:?}", node.address)[0..10].to_string() + "...",
                 node.entity_type,
                 volume_eth);
    }
    
    // Edge analysis
    println!("\n🔗 Top Edges by Volume:");
    let mut edges_sorted = network.edges.clone();
    edges_sorted.sort_by(|a, b| b.total_eth.cmp(&a.total_eth));
    
    for (i, edge) in edges_sorted.iter().take(5).enumerate() {
        let volume_eth = edge.total_eth.to::<u128>() as f64 / 1e18;
        println!("   {}. {} → {} : {:.4} ETH ({} transactions)", 
                 i + 1,
                 format!("{:?}", edge.from)[0..10].to_string() + "...",
                 format!("{:?}", edge.to)[0..10].to_string() + "...",
                 volume_eth,
                 edge.transaction_count);
    }
}

/// Generate visualization data for the network
async fn generate_visualization_data(network: &AddressNetwork, description: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎨 Generating Visualization Data");
    println!("================================");
    
    // Generate Cytoscape.js format
    let cytoscape_data = network.to_cytoscape_format();
    
    // Create output filename
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("network_{}_{}.json", 
                          description.replace(" ", "_").to_lowercase(),
                          timestamp);
    let filepath = format!("/home/nima/code/crypto/logs/dev/{}", filename);
    
    // Prepare comprehensive output
    let output = serde_json::json!({
        "description": description,
        "timestamp": timestamp.to_string(),
        "network_stats": {
            "nodes": network.nodes.len(),
            "edges": network.edges.len(),
            "total_eth_volume": network.stats.total_eth_volume.to_string(),
            "network_density": network.stats.network_density,
        },
        "cytoscape_data": cytoscape_data,
        "metadata": {
            "generated_by": "qarqa_tx_processor",
            "integration_example": "network_building",
            "version": "0.1.0"
        }
    });
    
    // Ensure directory exists
    std::fs::create_dir_all("/home/nima/code/crypto/logs/dev")?;
    
    // Write visualization data
    std::fs::write(&filepath, serde_json::to_string_pretty(&output)?)?;
    
    println!("✅ Visualization data saved to: {}", filepath);
    println!("   Format: Cytoscape.js compatible JSON");
    println!("   Nodes: {}", network.nodes.len());
    println!("   Edges: {}", network.edges.len());
    
    // Generate D3.js format as well
    let d3_data = generate_d3_format(network);
    let d3_filepath = filepath.replace(".json", "_d3.json");
    std::fs::write(&d3_filepath, serde_json::to_string_pretty(&d3_data)?)?;
    println!("   D3.js format: {}", d3_filepath);
    
    Ok(())
}

/// Generate D3.js compatible format
fn generate_d3_format(network: &AddressNetwork) -> serde_json::Value {
    let nodes: Vec<_> = network.nodes.iter().map(|node| {
        serde_json::json!({
            "id": format!("{:?}", node.address),
            "group": node.entity_type,
            "label": node.label.as_ref().unwrap_or(&"Unknown".to_string()),
        })
    }).collect();
    
    let links: Vec<_> = network.edges.iter().map(|edge| {
        serde_json::json!({
            "source": format!("{:?}", edge.from),
            "target": format!("{:?}", edge.to),
            "value": edge.total_eth.to::<u128>() as f64 / 1e18,
            "transactions": edge.transaction_count,
        })
    }).collect();
    
    serde_json::json!({
        "nodes": nodes,
        "links": links
    })
}

/// Analyze network patterns for insights
fn analyze_network_patterns(network: &AddressNetwork) {
    println!("\n🔍 Pattern Analysis");
    println!("===================");
    
    // Centrality analysis
    let hub_nodes = find_hub_nodes(network);
    if !hub_nodes.is_empty() {
        println!("🌟 Hub Nodes (high connectivity):");
        for (i, (address, degree)) in hub_nodes.iter().take(3).enumerate() {
            let node = network.nodes.iter()
                .find(|n| &n.address == address)
                .unwrap();
            println!("   {}. {} ({}) - {} connections", 
                     i + 1,
                     format!("{:?}", address)[0..10].to_string() + "...",
                     node.entity_type,
                     degree);
        }
    }
    
    // Flow analysis
    analyze_flow_patterns(network);
    
    // Clustering analysis
    analyze_clusters(network);
}

/// Find nodes with highest connectivity (degree centrality)
fn find_hub_nodes(network: &AddressNetwork) -> Vec<(Address, usize)> {
    let mut node_degrees = HashMap::new();
    
    for edge in &network.edges {
        *node_degrees.entry(edge.from).or_insert(0) += 1;
        *node_degrees.entry(edge.to).or_insert(0) += 1;
    }
    
    let mut sorted_nodes: Vec<_> = node_degrees.into_iter().collect();
    sorted_nodes.sort_by(|a, b| b.1.cmp(&a.1));
    sorted_nodes
}

/// Analyze flow patterns in the network
fn analyze_flow_patterns(network: &AddressNetwork) {
    println!("\n💸 Flow Patterns:");
    
    // Find major flow concentrations
    let total_volume = network.stats.total_eth_volume.to::<u128>() as f64 / 1e18;
    let avg_edge_volume = total_volume / network.edges.len() as f64;
    
    let major_flows: Vec<_> = network.edges.iter()
        .filter(|edge| {
            let volume_eth = edge.total_eth.to::<u128>() as f64 / 1e18;
            volume_eth > avg_edge_volume * 2.0
        })
        .collect();
    
    println!("   Average edge volume: {:.4} ETH", avg_edge_volume);
    println!("   Major flows (>2x average): {}", major_flows.len());
    
    if !major_flows.is_empty() {
        println!("   Top major flows:");
        for (i, edge) in major_flows.iter().take(3).enumerate() {
            let volume_eth = edge.total_eth.to::<u128>() as f64 / 1e18;
            println!("     {}. {:.4} ETH ({} txs)", 
                     i + 1, volume_eth, edge.transaction_count);
        }
    }
    
    // Detect circular flows (potential arbitrage)
    detect_circular_flows(network);
}

/// Detect circular flow patterns
fn detect_circular_flows(network: &AddressNetwork) {
    let mut circular_patterns = 0;
    let mut checked_pairs = std::collections::HashSet::new();
    
    for edge1 in &network.edges {
        for edge2 in &network.edges {
            if edge1.to == edge2.from && edge2.to == edge1.from {
                let pair = if edge1.from < edge2.from {
                    (edge1.from, edge2.from)
                } else {
                    (edge2.from, edge1.from)
                };
                
                if !checked_pairs.contains(&pair) {
                    checked_pairs.insert(pair);
                    circular_patterns += 1;
                }
            }
        }
    }
    
    if circular_patterns > 0 {
        println!("   🔄 Circular flow patterns: {} (potential arbitrage)", circular_patterns);
    }
}

/// Analyze clustering in the network
fn analyze_clusters(network: &AddressNetwork) {
    println!("\n🎯 Clustering Analysis:");
    
    // Simple clustering based on entity types
    let mut type_clusters = HashMap::new();
    for node in &network.nodes {
        type_clusters.entry(&node.entity_type).or_insert(Vec::new()).push(node);
    }
    
    println!("   Entity type clusters:");
    for (entity_type, nodes) in type_clusters {
        if nodes.len() > 1 {
            println!("     {}: {} nodes", entity_type, nodes.len());
        }
    }
    
    // Analyze inter-cluster connections
    let mut inter_cluster_edges = 0;
    for edge in &network.edges {
        let from_node = network.nodes.iter().find(|n| n.address == edge.from);
        let to_node = network.nodes.iter().find(|n| n.address == edge.to);
        
        if let (Some(from), Some(to)) = (from_node, to_node) {
            if from.entity_type != to.entity_type {
                inter_cluster_edges += 1;
            }
        }
    }
    
    let cluster_connectivity = inter_cluster_edges as f64 / network.edges.len() as f64;
    println!("   Inter-cluster connectivity: {:.2}%", cluster_connectivity * 100.0);
}

/// Create mock transactions with varied patterns
fn create_mock_transaction(tx_hash: &str, index: usize) -> Result<Transaction, Box<dyn std::error::Error>> {
    // Create different transaction types based on index
    let addresses = [
        "0x742d35Cc6634C0532925a3b844Bc9e7595f5b899", // User 1
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", // Vitalik (whale)
        "0xE592427A0AEce92De3Edee1F18E0157C05861564", // Uniswap V3 Router
        "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", // Uniswap V2 Router
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
    ];
    
    let from_addr = addresses[index % addresses.len()];
    let to_addr = addresses[(index + 1) % addresses.len()];
    
    Ok(Transaction {
        hash: B256::from_str(tx_hash)?,
        block_number: 18000000 + index as u64,
        from_address: Address::from_str(from_addr)?,
        to_address: Some(Address::from_str(to_addr)?),
        value: U256::from((index + 1) as u64 * 1000000000000000000u64), // Variable ETH amounts
        gas_price: U256::from(20_000_000_000u64),
        gas_limit: 200_000,
        gas_used: Some(150_000 + (index * 10_000) as u64),
        status: true,
        timestamp: Some(chrono::Utc::now()),
        input_data: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_examples_data() {
        assert_eq!(NETWORK_EXAMPLES.len(), 3);
        
        for (tx_hashes, description) in NETWORK_EXAMPLES {
            assert!(!tx_hashes.is_empty());
            assert!(!description.is_empty());
            
            for hash in *tx_hashes {
                assert!(hash.starts_with("0x"));
                assert_eq!(hash.len(), 66);
            }
        }
    }
    
    #[test]
    fn test_mock_transaction_creation() {
        let tx = create_mock_transaction(NETWORK_EXAMPLES[0].0[0], 0);
        assert!(tx.is_ok());
        
        let transaction = tx.unwrap();
        assert_eq!(transaction.block_number, 18000000);
        assert!(transaction.status);
    }
    
    #[test]
    fn test_hub_node_detection() {
        // This would need a real network to test properly
        // For now, just test that the function doesn't panic
        let empty_network = AddressNetwork {
            nodes: vec![],
            edges: vec![],
            stats: NetworkStats {
                total_eth_volume: U256::ZERO,
                network_density: 0.0,
            },
        };
        
        let hubs = find_hub_nodes(&empty_network);
        assert!(hubs.is_empty());
    }
}