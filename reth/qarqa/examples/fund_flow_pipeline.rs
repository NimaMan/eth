//! Example demonstrating the complete fund flow pipeline
//! 
//! This example shows:
//! 1. Connecting to PostgreSQL eth_db
//! 2. Fetching transactions for an address
//! 3. Processing transactions with tx_processor
//! 4. Building fund flow networks
//! 5. Interactive network expansion

use qarqa_eth_db_fetcher::{create_pool, DbConfig, AddressFetcher, TransactionFetcher};
use qarqa_fundflownetwork::{
    InteractiveFundFlowNetwork, InteractiveConfig,
    FundFlowAnalyzer, NetworkBuilder,
    CytoscapeExporter, VisJsExporter,
};
use tx_processor::TxProcessor;
use alloy_primitives::Address;
use std::str::FromStr;
use std::env;
use tokio;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Database configuration from environment
    let db_config = DbConfig {
        host: env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()).parse()?,
        database: env::var("DB_NAME").unwrap_or_else(|_| "eth_db".to_string()),
        username: env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
        password: env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string()),
        max_connections: 10,
    };
    
    // Create database pool
    info!("Connecting to PostgreSQL database...");
    let db_pool = create_pool(&db_config).await?;
    
    // Initialize transaction processor
    let rpc_url = env::var("ETH_RPC_URL")
        .unwrap_or_else(|_| "https://eth.llamarpc.com".to_string());
    let tx_processor = TxProcessor::new(&rpc_url).await?;
    
    // Example 1: Simple fund flow analysis
    println!("\n=== Example 1: Simple Fund Flow Analysis ===");
    simple_fund_flow_example(&db_pool, &tx_processor).await?;
    
    // Example 2: Interactive network building
    println!("\n=== Example 2: Interactive Network Building ===");
    interactive_network_example(&db_pool, &tx_processor).await?;
    
    // Example 3: High value transaction network
    println!("\n=== Example 3: High Value Transaction Network ===");
    high_value_network_example(&db_pool, &tx_processor).await?;
    
    Ok(())
}

/// Example 1: Simple fund flow analysis for a single address
async fn simple_fund_flow_example(
    db_pool: &sqlx::PgPool,
    tx_processor: &TxProcessor,
) -> eyre::Result<()> {
    let address_fetcher = AddressFetcher::new(db_pool.clone());
    
    // Use a well-known address (Uniswap V3 Router)
    let target_address = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45";
    
    info!("Fetching transactions for {}", target_address);
    
    // Get recent transactions
    let addr_txs = address_fetcher.get_address_transactions(
        target_address,
        Some(10), // Just 10 transactions for demo
        None,
    ).await?;
    
    println!("Found {} total transactions, analyzing first 10...", addr_txs.total_count);
    
    // Process transactions and extract fund flows
    let mut all_fund_flows = Vec::new();
    
    for tx_hash in &addr_txs.tx_hashes {
        match tx_processor.process_transaction(tx_hash).await {
            Ok(processed) => {
                match qarqa_fundflownetwork::extract_fund_flows_from_processed_tx(&processed) {
                    Ok(flows) => {
                        println!("Transaction {}: {} ETH movements, {} token movements",
                                 &tx_hash[..10],
                                 flows.eth_movements.len(),
                                 flows.token_movements.len());
                        all_fund_flows.push(flows);
                    }
                    Err(e) => error!("Failed to extract flows: {}", e),
                }
            }
            Err(e) => error!("Failed to process tx {}: {}", tx_hash, e),
        }
    }
    
    // Analyze fund flows
    let analyzer = FundFlowAnalyzer::new()
        .with_min_value(alloy_primitives::U256::from(1e16)); // 0.01 ETH minimum
    
    let fund_flows = analyzer.analyze_fund_flows(&all_fund_flows);
    println!("\nExtracted {} unique fund flows", fund_flows.len());
    
    // Show top flows
    for (i, flow) in fund_flows.iter().take(5).enumerate() {
        println!("{}. {:?} → {:?}: {:.4} ETH ({} txs)",
                 i + 1,
                 &format!("{:?}", flow.from)[..10],
                 &format!("{:?}", flow.to)[..10],
                 flow.amount_eth,
                 flow.transaction_count);
    }
    
    // Calculate net balances
    let balances = analyzer.calculate_net_balances(&fund_flows);
    println!("\nNet balances for {} addresses", balances.len());
    
    Ok(())
}

/// Example 2: Interactive network building with expansion
async fn interactive_network_example(
    db_pool: &sqlx::PgPool,
    tx_processor: &TxProcessor,
) -> eyre::Result<()> {
    // Configure interactive builder
    let config = InteractiveConfig {
        max_txs_per_address: 50,
        min_eth_flow: 0.1, // 0.1 ETH minimum
        fetch_metadata: true,
        use_cache: true,
    };
    
    // Create interactive network
    let mut network = InteractiveFundFlowNetwork::new(
        db_pool.clone(),
        tx_processor.clone(),
        config,
    ).await?;
    
    // Start from a CEX address (example: Binance hot wallet)
    let center = Address::from_str("0x28C6c06298d514Db089934071355E5743bf21d60")?;
    
    info!("Initializing network from center address...");
    network.initialize_from_address(center).await?;
    
    // Get initial stats
    let stats = network.get_stats();
    println!("Initial network: {} nodes, {} edges", stats.node_count, stats.edge_count);
    
    // Expand network interactively
    for round in 1..=3 {
        println!("\n--- Expansion Round {} ---", round);
        
        // Get top candidates for expansion
        let candidates = network.get_expansion_candidates(5);
        
        println!("Top expansion candidates:");
        for (i, candidate) in candidates.iter().enumerate() {
            println!("{}. {:?} - {} ETH total flow ({})",
                     i + 1,
                     &format!("{:?}", candidate.address)[..10],
                     candidate.total_flow as u64,
                     candidate.reason);
        }
        
        // Expand top 2 candidates
        let new_addresses = network.expand_top_candidates(2).await?;
        println!("Added {} new addresses", new_addresses.len());
        
        // Show updated stats
        let stats = network.get_stats();
        println!("Network now has {} nodes, {} edges", stats.node_count, stats.edge_count);
        println!("Total volume: {:.2} ETH", stats.total_volume_eth);
    }
    
    // Export visualization
    let final_network = network.export_network();
    
    // Export to Cytoscape format
    let cytoscape_json = CytoscapeExporter::export(final_network)?;
    std::fs::write("fund_flow_network_cytoscape.json", 
                   serde_json::to_string_pretty(&cytoscape_json)?)?;
    
    // Export to vis.js format
    let visjs_json = VisJsExporter::export(final_network)?;
    std::fs::write("fund_flow_network_visjs.json", 
                   serde_json::to_string_pretty(&visjs_json)?)?;
    
    println!("\nExported network visualizations to JSON files");
    
    Ok(())
}

/// Example 3: High value transaction network
async fn high_value_network_example(
    db_pool: &sqlx::PgPool,
    tx_processor: &TxProcessor,
) -> eyre::Result<()> {
    let tx_fetcher = TransactionFetcher::new(db_pool.clone());
    let address_fetcher = AddressFetcher::new(db_pool.clone());
    
    // Get high value transactions (> 100 ETH)
    info!("Fetching high value transactions...");
    let high_value_txs = tx_fetcher.get_high_value_transactions(100.0, Some(20)).await?;
    
    println!("Found {} high value transactions", high_value_txs.len());
    
    // Process and build network
    let mut builder = NetworkBuilder::new()
        .with_min_eth_amount(10.0); // Only show flows > 10 ETH
    
    let mut all_fund_flows = Vec::new();
    
    for tx_record in &high_value_txs {
        // Fetch metadata for addresses
        for addr in [&tx_record.from_address, &tx_record.to_address] {
            if let Ok(Some(addr_record)) = address_fetcher.get_address(addr).await {
                builder.add_node_metadata(
                    Address::from_str(addr)?,
                    addr_record.name,
                    addr_record.is_contract,
                    addr_record.entity_category,
                );
            }
        }
        
        // Process transaction
        match tx_processor.process_transaction(&tx_record.tx_hash).await {
            Ok(processed) => {
                if let Ok(flows) = qarqa_fundflownetwork::extract_fund_flows_from_processed_tx(&processed) {
                    all_fund_flows.push(flows);
                }
            }
            Err(e) => error!("Failed to process high value tx: {}", e),
        }
    }
    
    // Analyze and build network
    let analyzer = FundFlowAnalyzer::new();
    let fund_flows = analyzer.analyze_fund_flows(&all_fund_flows);
    let network = builder.build_from_flows(&fund_flows);
    
    println!("\nHigh value network statistics:");
    let stats = network.calculate_stats();
    println!("- Nodes: {}", stats.node_count);
    println!("- Edges: {}", stats.edge_count);
    println!("- Total volume: {:.2} ETH (${:.2}M at $2500/ETH)", 
             stats.total_volume_eth,
             stats.total_volume_eth * 2500.0 / 1_000_000.0);
    println!("- Average degree: {:.2}", stats.avg_degree);
    println!("- Max balance change: {:.2} ETH", stats.max_balance_change);
    
    // Find zero-net intermediaries
    let zero_net = builder.contract_zero_net_nodes(&network);
    println!("\nFound {} potential router/intermediary contracts", zero_net.len());
    
    Ok(())
}