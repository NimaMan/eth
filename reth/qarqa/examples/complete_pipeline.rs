//! Complete QARQA Pipeline Example
//!
//! This demonstrates the full flow from transaction hash to fund flow network:
//! 1. Fetch transaction from database
//! 2. Extract transfers using RPC (internal + token transfers)
//! 3. Build fund flow network
//! 4. Generate visualization data

use qarqa_data_access::TransactionDataFetcher;
use qarqa_tx_simulation::{RpcTransactionSimulator, TransactionSimulator, FundFlowAnalyzer};
use qarqa_network_building::{FundFlowNetworkBuilder, NetworkBuilderConfig};
use sqlx::postgres::PgPool;
use alloy_primitives::B256;
use std::str::FromStr;
use tracing::{info, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Configuration
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    
    // Transaction to analyze
    let tx_hash = B256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
    
    println!("🚀 QARQA Complete Pipeline Demo");
    println!("================================\n");
    
    // Step 1: Fetch transaction from database
    info!("Step 1: Fetching transaction from database...");
    let pool = PgPool::connect(&database_url).await?;
    let tx_fetcher = TransactionDataFetcher::new(pool);
    
    let transaction = tx_fetcher.get_transaction_by_hash(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    
    println!("✅ Transaction found:");
    println!("   Block: {}", transaction.block_number);
    println!("   From: {:?}", transaction.from_address);
    println!("   To: {:?}", transaction.to_address);
    println!("   Value: {} wei", transaction.value);
    
    // Step 2: Extract transfers using RPC
    info!("\nStep 2: Extracting transfers via RPC...");
    let mut simulator = RpcTransactionSimulator::new(rpc_url);
    simulator.initialize().await?;
    
    let fund_flows = simulator.simulate_transaction(&transaction).await?;
    
    println!("✅ Transfers extracted:");
    println!("   ETH movements: {}", fund_flows.eth_movements.len());
    println!("   Token movements: {}", fund_flows.token_movements.len());
    
    // Display some transfers
    for movement in fund_flows.eth_movements.iter().take(3) {
        let eth_amount = movement.amount.to::<u128>() as f64 / 1e18;
        println!("   - {:?}: {:.6} ETH", movement.movement_type, eth_amount);
    }
    
    // Step 3: Build fund flow network
    info!("\nStep 3: Building fund flow network...");
    
    // Analyze fund flows
    let analyzer = FundFlowAnalyzer::new()
        .with_weth_as_eth(true)
        .with_gas_inclusion(false);
    
    let analyzed_flows = analyzer.analyze_fund_flows(&[fund_flows.clone()])?;
    let net_balances = analyzer.calculate_net_balances(&analyzed_flows);
    
    println!("✅ Network analysis complete:");
    println!("   Unique fund flows: {}", analyzed_flows.len());
    println!("   Addresses involved: {}", net_balances.len());
    
    // Build network
    let config = NetworkBuilderConfig::default()
        .with_min_value_eth(0.001)
        .with_combine_weth(true);
    
    let mut network_builder = FundFlowNetworkBuilder::new(config);
    network_builder.add_fund_flows(analyzed_flows)?;
    let network = network_builder.build()?;
    
    println!("✅ Network built:");
    println!("   Nodes: {}", network.nodes.len());
    println!("   Edges: {}", network.edges.len());
    
    // Step 4: Generate visualization data
    info!("\nStep 4: Generating visualization data...");
    
    // Convert to Cytoscape.js format
    let cytoscape_data = network.to_cytoscape_format();
    
    println!("✅ Visualization data ready:");
    println!("   Format: Cytoscape.js");
    println!("   Size: {} bytes", serde_json::to_string(&cytoscape_data)?.len());
    
    // Display network summary
    println!("\n📊 Network Summary");
    println!("==================");
    
    // Show nodes with significant activity
    println!("\nTop Active Addresses:");
    let mut node_activity: Vec<_> = network.nodes.iter()
        .map(|node| {
            let total_change = node.state_changes.values()
                .map(|changes| changes.values().map(|v| v.abs()).sum::<f64>())
                .sum::<f64>();
            (node.address.clone(), total_change)
        })
        .collect();
    
    node_activity.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    for (address, activity) in node_activity.iter().take(5) {
        println!("   {:?}: {:.4} ETH equivalent", address, activity);
    }
    
    // Show largest flows
    println!("\nLargest Fund Flows:");
    let mut edges_by_value: Vec<_> = network.edges.iter().collect();
    edges_by_value.sort_by(|a, b| b.total_value.partial_cmp(&a.total_value).unwrap());
    
    for edge in edges_by_value.iter().take(5) {
        println!("   {:?} → {:?}: {:.4} ETH", 
            edge.source, 
            edge.target, 
            edge.total_value
        );
    }
    
    // Save visualization data
    let output_file = "network_visualization.json";
    std::fs::write(output_file, serde_json::to_string_pretty(&cytoscape_data)?)?;
    println!("\n✅ Visualization data saved to: {}", output_file);
    
    println!("\n🎉 Pipeline complete!");
    println!("\nThis network can now be visualized in the web interface.");
    
    Ok(())
}