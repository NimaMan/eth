//! Example: Simulate a transaction using RPC to extract all transfers
//!
//! This shows how to use the RPC-based simulator to get internal transfers
//! and token transfers from a real transaction.

use qarqa_tx_simulation::{RpcTransactionSimulator, TransactionSimulator};
use qarqa_data_access::TransactionDataFetcher;
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
    
    // Transaction to analyze (the complex arbitrage transaction)
    let tx_hash = B256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
    
    info!("Connecting to database...");
    let pool = PgPool::connect(&database_url).await?;
    
    info!("Fetching transaction data...");
    let tx_fetcher = TransactionDataFetcher::new(pool);
    let transaction = tx_fetcher.get_transaction_by_hash(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    
    info!("Transaction found: block {}, from {:?}", 
        transaction.block_number, 
        transaction.from_address
    );
    
    // Create RPC simulator
    let mut simulator = RpcTransactionSimulator::new(rpc_url);
    simulator.initialize().await?;
    
    info!("Simulating transaction to extract transfers...");
    let fund_flows = simulator.simulate_transaction(&transaction).await?;
    
    // Display results
    println!("\n=== Transaction Fund Flows ===");
    println!("Transaction: {:?}", fund_flows.transaction_hash);
    println!("Block: {}", fund_flows.block_number);
    println!("Status: {}", if fund_flows.status { "Success" } else { "Failed" });
    println!("Gas Used: {}", fund_flows.gas_used);
    
    println!("\n=== ETH Movements ===");
    for (i, movement) in fund_flows.eth_movements.iter().enumerate() {
        let eth_amount = movement.amount.to::<u128>() as f64 / 1e18;
        println!("{}. {:?}", i + 1, movement.movement_type);
        println!("   From: {:?}", movement.from);
        println!("   To: {:?}", movement.to);
        println!("   Amount: {:.6} ETH", eth_amount);
    }
    
    println!("\n=== Token Movements ===");
    for (i, movement) in fund_flows.token_movements.iter().enumerate() {
        println!("{}. Token: {:?}", i + 1, movement.token_address);
        println!("   From: {:?}", movement.from);
        println!("   To: {:?}", movement.to);
        println!("   Amount: {} (raw)", movement.amount);
        if let Some(symbol) = &movement.symbol {
            println!("   Symbol: {}", symbol);
        }
    }
    
    // Now we can build a network from this data
    println!("\n=== Building Fund Flow Network ===");
    
    use qarqa_tx_simulation::FundFlowAnalyzer;
    let analyzer = FundFlowAnalyzer::new()
        .with_weth_as_eth(true)
        .with_gas_inclusion(false);
    
    let fund_flows_vec = vec![fund_flows];
    let network_flows = analyzer.analyze_fund_flows(&fund_flows_vec)?;
    
    println!("\nExtracted {} unique fund flows", network_flows.len());
    for flow in network_flows.iter().take(10) {
        println!("  {:?} → {:?}: {:.6} ETH", 
            flow.from, 
            flow.to, 
            flow.amount_eth
        );
    }
    
    Ok(())
}