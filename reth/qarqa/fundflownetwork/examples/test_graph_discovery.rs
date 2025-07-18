//! Test graph discovery with the pool → creator → MEXC example

use qarqa_fundflownetwork::graph_discovery::{
    GraphExplorer, DiscoveryConfig, DiscoveryOutput
};
use qarqa_eth_db_fetcher::{create_pool, DbConfig};
use alloy_primitives::{Address, U256};
use std::str::FromStr;
use std::collections::HashSet;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    println!("🔍 Testing Graph Discovery Module");
    println!("=================================\n");
    
    // Known addresses from the example
    let pool_address = Address::from_str("0x0e9797F0f05A3dE8384D76467E98DA03874c86a6")?; // Pool
    let creator_address = Address::from_str("0xC04B517E75907965AD59976c63912C8C8af97D96")?; // Creator
    let mexc_address = Address::from_str("0x9642b23Ed1E01Df1092B92641051881a322F5D4E")?; // MEXC
    
    println!("DEBUG: Address format check:");
    println!("Pool address formatted: '{}'", format!("{:?}", pool_address));
    
    println!("Test case: Pool → Creator → MEXC fund flow");
    println!("Pool:    {:?}", pool_address);
    println!("Creator: {:?}", creator_address);
    println!("MEXC:    {:?}", mexc_address);
    println!();
    
    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string());
    
    let db_config = DbConfig {
        database_url,
        max_connections: 5,
        min_connections: 1,
        connect_timeout: std::time::Duration::from_secs(30),
    };
    
    println!("Connecting to database...");
    let db_pool = create_pool(&db_config).await?;
    
    // Create graph explorer
    let explorer = GraphExplorer::new(db_pool);
    
    // Configure discovery
    let config = DiscoveryConfig {
        max_depth: 2,                                          // Should reach MEXC in 2 hops
        max_nodes: 50,                                         // Plenty for this example
        min_value_wei: U256::ZERO,                            // Include 0-value transactions
        max_txs_per_address: 20,                              // Limit for testing
        time_window_blocks: None,                             // No time limit
    };
    
    println!("\nStarting graph discovery from pool address...");
    println!("Config: max_depth={}, min_value=0 ETH (include all transactions)", config.max_depth);
    
    // Run discovery
    let discovery_result = explorer.explore(
        pool_address,
        config,
        &HashSet::new(), // No previously explored addresses
    ).await?;
    
    // Analyze results
    analyze_discovery_results(&discovery_result, pool_address, creator_address, mexc_address);
    
    Ok(())
}

fn analyze_discovery_results(
    result: &DiscoveryOutput,
    pool: Address,
    creator: Address,
    mexc: Address,
) {
    println!("\n📊 Discovery Results");
    println!("===================");
    
    // Stats
    println!("\nStatistics:");
    println!("  Nodes discovered: {}", result.graph.node_count());
    println!("  Edges found: {}", result.graph.edge_count());
    println!("  Priority transactions: {}", result.priority_transactions.len());
    println!("  Stop reason: {:?}", result.stop_reason);
    println!("  Time taken: {}ms", result.discovery_stats.time_taken_ms);
    println!("  Total txs examined: {}", result.discovery_stats.total_txs_examined);
    println!("  Addresses explored: {}", result.discovery_stats.addresses_explored);
    println!("  CEX addresses found: {}", result.discovery_stats.cex_addresses_found);
    println!("  DEX addresses found: {}", result.discovery_stats.dex_addresses_found);
    
    // Check if we found our expected nodes
    println!("\n🎯 Expected Nodes Check:");
    
    if let Some(pool_node) = result.graph.nodes.get(&pool) {
        println!("✅ Pool found: entity_type={:?}, depth={}", 
                 pool_node.entity_type, pool_node.exploration_depth);
    } else {
        println!("❌ Pool not found!");
    }
    
    if let Some(creator_node) = result.graph.nodes.get(&creator) {
        println!("✅ Creator found: entity_type={:?}, depth={}", 
                 creator_node.entity_type, creator_node.exploration_depth);
    } else {
        println!("❌ Creator not found!");
    }
    
    if let Some(mexc_node) = result.graph.nodes.get(&mexc) {
        println!("✅ MEXC found: entity_type={:?}, depth={}, explored={}", 
                 mexc_node.entity_type, mexc_node.exploration_depth, mexc_node.explored);
        
        // MEXC should NOT be explored (CEX routing rule)
        if !mexc_node.explored {
            println!("✅ MEXC correctly not explored (CEX routing rule)");
        }
    } else {
        println!("❌ MEXC not found!");
    }
    
    // Show all discovered nodes
    println!("\n📍 All Discovered Nodes:");
    for (addr, node) in &result.graph.nodes {
        println!("  {:?}", addr);
        println!("    Type: {:?}", node.entity_type);
        println!("    Depth: {}", node.exploration_depth);
        println!("    Contract: {}", node.is_contract);
        println!("    Explored: {}", node.explored);
        if let Some(name) = &node.name {
            println!("    Name: {}", name);
        }
    }
    
    // Show edges
    println!("\n🔗 Discovered Edges:");
    for edge in &result.graph.edges {
        let value_eth = edge.value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  {:?} ←→ {:?}", edge.node1, edge.node2);
        println!("    TX: {:?}", edge.tx_hash);
        println!("    Value: {:.4} ETH", value_eth);
        println!("    Block: {}", edge.block_number);
    }
    
    // Show priority transactions
    println!("\n⚡ Priority Transactions for Deep Analysis:");
    for (i, tx) in result.priority_transactions.iter().enumerate() {
        let value_eth = tx.value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  {}. TX: {:?}", i + 1, tx.tx_hash);
        println!("     From: {:?}", tx.from_address);
        println!("     To: {:?}", tx.to_address);
        println!("     Value: {:.4} ETH", value_eth);
        println!("     Score: {:.2}", tx.priority_score);
        println!("     Involves unknown: {}", tx.involves_unknown);
        println!("     Involves router: {}", tx.involves_router);
    }
}