//! Test graph discovery with the pool → creator → MEXC example

use tx_fund_flow_fundflownetwork::graph_discovery::{
    GraphExplorer, DiscoveryConfig, DiscoveryOutput
};
use tx_fund_flow_eth_db_fetcher::{create_pool, DbConfig};
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
    
    // Test which pool to use
    let test_new_pool = std::env::var("TEST_NEW_POOL").is_ok();
    
    let (pool_address, creator_address, mexc_address, liquidity_block) = if test_new_pool {
        // New pool from Jul-17-2025
        println!("Testing NEW pool: DOGEINA");
        (
            Address::from_str("0x5423621C6D3E22465876deb92aD4f40Dc25b62A3")?, // Pool
            Address::from_str("0x62eD63Ce328665488914992B247b1EF8F78199A1")?, // Creator
            Address::from_str("0x0000000000000000000000000000000000000000")?, // Unknown destination
            22939562u64, // Liquidity added block
        )
    } else {
        // Original pool
        println!("Testing ORIGINAL pool");
        (
            Address::from_str("0x0e9797F0f05A3dE8384D76467E98DA03874c86a6")?, // Pool
            Address::from_str("0xC04B517E75907965AD59976c63912C8C8af97D96")?, // Creator
            Address::from_str("0x9642b23Ed1E01Df1092B92641051881a322F5D4E")?, // MEXC
            22885510u64, // Liquidity added block
        )
    };
    
    println!("DEBUG: Address format check:");
    println!("Pool address formatted: '{}'", format!("{:?}", pool_address));
    
    println!("Test case: Pool → Creator → ? fund flow");
    println!("Pool:            {:?}", pool_address);
    println!("Creator:         {:?}", creator_address);
    println!("Destination:     {:?}", mexc_address);
    println!("Liquidity Block: {}", liquidity_block);
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
        max_depth: 2,                                          // Should reach destination in 2 hops
        max_nodes: 50,                                         // Plenty for this example
        min_value_wei: U256::ZERO,                            // Include 0-value transactions
        max_txs_per_address: 20,                              // Limit for testing
        max_block_number: Some(liquidity_block),              // Only transactions before liquidity was added
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
    analyze_discovery_results(&discovery_result, pool_address, creator_address, mexc_address, liquidity_block);
    
    Ok(())
}

fn analyze_discovery_results(
    result: &DiscoveryOutput,
    pool: Address,
    creator: Address,
    mexc: Address,
    liquidity_block: u64,
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
    
    if let Some(dest_node) = result.graph.nodes.get(&mexc) {
        println!("✅ Destination found: entity_type={:?}, depth={}, explored={}", 
                 dest_node.entity_type, dest_node.exploration_depth, dest_node.explored);
        
        // Check routing rules
        if dest_node.entity_type.as_deref() == Some("CEX") && !dest_node.explored {
            println!("✅ CEX correctly not explored (CEX routing rule)");
        }
    } else {
        println!("❌ Destination not found!");
    }
    
    // Check for specific transaction
    let liquidity_tx = "0xf4ab34d859d7aab8a33522364737f5fb8c57715b4e4d5929cdd29b4a06ced644";
    println!("\n🔍 Looking for liquidity addition tx: {}", liquidity_tx);
    
    let found_liquidity_tx = result.graph.edges.iter()
        .any(|edge| format!("{:?}", edge.tx_hash).to_lowercase().contains(&liquidity_tx[2..]));
    
    if found_liquidity_tx {
        println!("✅ Liquidity addition transaction found in edges!");
    } else {
        println!("❌ Liquidity addition transaction NOT found in edges!");
        
        // Check priority txs
        let in_priority = result.priority_transactions.iter()
            .any(|tx| format!("{:?}", tx.tx_hash).to_lowercase().contains(&liquidity_tx[2..]));
        
        if in_priority {
            println!("   But it's in priority transactions for analysis");
        }
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
        
        // Check if this is at the liquidity block
        if edge.block_number == liquidity_block {
            println!("    ⚠️  This tx is at the liquidity addition block!");
        }
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