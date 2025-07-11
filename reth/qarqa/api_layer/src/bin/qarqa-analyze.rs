//! QARQA Transaction Analyzer CLI
//!
//! Analyzes Ethereum transactions to extract fund flows and build networks.

use clap::{Command, Arg};
use qarqa_data_access::TransactionDataFetcher;
use qarqa_tx_simulation::{FastPathSimulator, TransactionSimulator, FundFlowAnalyzer};
use qarqa_network_building::NetworkBuilder;
use sqlx::postgres::PgPool;
use alloy_primitives::B256;
use std::str::FromStr;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let matches = Command::new("qarqa-analyze")
        .version("0.1.0")
        .about("Analyzes Ethereum transactions to extract fund flows")
        .arg(Arg::new("tx-hash")
            .help("Transaction hash to analyze")
            .required(true)
            .index(1))
        .arg(Arg::new("database-url")
            .long("database-url")
            .value_name("URL")
            .help("PostgreSQL database URL")
            .env("DATABASE_URL")
            .default_value("postgresql://postgres:postgres@localhost/eth_db"))
        .arg(Arg::new("rpc-url")
            .long("rpc-url")
            .value_name("URL")
            .help("Ethereum RPC endpoint URL")
            .env("RPC_URL")
            .default_value("http://127.0.0.1:8545"))
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_name("FILE")
            .help("Output file for network data")
            .default_value("network.json"))
        .get_matches();
    
    let tx_hash_str = matches.get_one::<String>("tx-hash").unwrap();
    let database_url = matches.get_one::<String>("database-url").unwrap();
    let _rpc_url = matches.get_one::<String>("rpc-url").unwrap();
    let output_file = matches.get_one::<String>("output").unwrap();
    
    // Parse transaction hash
    let tx_hash = B256::from_str(tx_hash_str)?;
    
    println!("🔍 QARQA Transaction Analyzer");
    println!("============================");
    println!("Transaction: {}", tx_hash_str);
    
    // Step 1: Database connection
    info!("Connecting to database...");
    let pool = PgPool::connect(database_url).await?;
    let tx_fetcher = TransactionDataFetcher::new(pool);
    
    // Step 2: Fetch transaction
    info!("Fetching transaction...");
    let transaction = tx_fetcher.get_transaction_by_hash(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found in database"))?;
    
    println!("✓ Transaction found in block {}", transaction.block_number);
    
    // Step 3: Extract transfers via REVM simulation
    info!("Extracting transfers via REVM simulation...");
    let mut simulator = FastPathSimulator::new();
    simulator.initialize().await?;
    
    let fund_flows = simulator.simulate_transaction(&transaction).await?;
    
    println!("✓ Found {} ETH movements", fund_flows.eth_movements.len());
    println!("✓ Found {} token movements", fund_flows.token_movements.len());
    
    // Step 4: Build network
    info!("Building fund flow network...");
    
    let analyzer = FundFlowAnalyzer::new()
        .with_weth_as_eth(true)
        .with_gas_inclusion(false);
    
    let analyzed_flows = analyzer.analyze_fund_flows(&[fund_flows])?;
    
    let network_builder = NetworkBuilder::new();
    let network = network_builder.build_from_fund_flows(&analyzed_flows, None)?;
    
    println!("✓ Network built with {} nodes and {} edges", 
        network.nodes.len(), 
        network.edges.len()
    );
    
    // Step 5: Save output
    info!("Saving network data...");
    let cytoscape_data = network.to_cytoscape_format();
    std::fs::write(output_file, serde_json::to_string_pretty(&cytoscape_data)?)?;
    
    println!("✓ Network saved to: {}", output_file);
    
    // Display summary
    println!("\n📊 Network Summary:");
    println!("─────────────────");
    
    // Show top flows
    let mut edges: Vec<_> = network.edges.iter().collect();
    edges.sort_by(|a, b| b.total_eth.cmp(&a.total_eth));
    
    println!("\nTop Fund Flows:");
    for edge in edges.iter().take(5) {
        let eth_amount = edge.total_eth.to::<u128>() as f64 / 1e18;
        println!("  {:?} → {:?}: {:.4} ETH", 
            edge.from, 
            edge.to, 
            eth_amount
        );
    }
    
    println!("\n✅ Analysis complete!");
    
    Ok(())
}