//! QARQA CLI Application

use clap::{Parser, Subcommand};
use qarqa_core_types::QarqaResult;
use qarqa_network_building::NetworkCurrency;
use qarqa_api_layer::FundFlowPipeline;
use alloy_primitives::Address;
use std::str::FromStr;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "qarqa")]
#[command(about = "QARQA blockchain analytics CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate fund flow network for an address
    FundFlow {
        /// Target address
        address: String,
        
        /// Network depth (hops from center)
        #[arg(short, long, default_value = "2")]
        depth: u32,
        
        /// Currency for value calculations
        #[arg(short, long, default_value = "USD")]
        currency: String,
        
        /// Minimum value change threshold
        #[arg(short, long, default_value = "10.0")]
        min_value: f64,
        
        /// Maximum number of transactions to analyze
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
    
    /// Test database connectivity
    TestDb {
        /// Database URL
        #[arg(long, default_value = "postgresql://postgres:postgres@localhost/eth_db")]
        database_url: String,
    },
}

#[tokio::main]
async fn main() -> QarqaResult<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::FundFlow { address, depth, currency, min_value, limit } => {
            handle_fund_flow(address, depth, currency, min_value, limit).await?;
        }
        Commands::TestDb { database_url } => {
            handle_test_db(database_url).await?;
        }
    }
    
    Ok(())
}

async fn handle_fund_flow(
    address_str: String,
    _depth: u32,
    currency_str: String,
    min_value: f64,
    limit: usize,
) -> QarqaResult<()> {
    info!("Building fund flow network for address: {}", address_str);
    
    // Parse address
    let center_address = Address::from_str(&address_str)
        .map_err(|e| qarqa_core_types::QarqaError::InvalidInput(format!("Invalid address: {}", e)))?;
    
    // Parse currency
    let currency = match currency_str.to_uppercase().as_str() {
        "ETH" => NetworkCurrency::ETH,
        "USD" => NetworkCurrency::USD,
        "USDC" => NetworkCurrency::USDC,
        "USDT" => NetworkCurrency::USDT,
        _ => NetworkCurrency::USD,
    };
    
    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
    
    // Create pipeline
    let pipeline = FundFlowPipeline::new(database_url).await?;
    
    // Run analysis
    match pipeline.analyze_address_fund_flows(center_address, limit, min_value, currency).await {
        Ok(visualization_data) => {
            // Output JSON to stdout
            println!("{}", serde_json::to_string(&visualization_data).unwrap());
        }
        Err(e) => {
            error!("Analysis failed: {}", e);
            
            // Return error JSON
            let error_response = serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "data": {
                    "nodes": [],
                    "edges": [],
                    "stats": {
                        "totalAddresses": 0,
                        "totalEdges": 0,
                        "totalEthVolume": "0",
                        "networkDensity": "0"
                    }
                }
            });
            println!("{}", serde_json::to_string(&error_response).unwrap());
        }
    }
    
    Ok(())
}

async fn handle_test_db(database_url: String) -> QarqaResult<()> {
    info!("Testing database connectivity: {}", database_url);
    
    let db = qarqa_data_access::DatabaseManager::new(&database_url).await?;
    db.health_check().await?;
    
    info!("Database connection successful!");
    
    println!("{}", serde_json::json!({
        "success": true,
        "message": "Database connection successful"
    }));
    
    Ok(())
}