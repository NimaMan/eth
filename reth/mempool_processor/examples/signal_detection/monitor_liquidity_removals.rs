/// Monitor Liquidity Removal Transactions in Real-Time
/// 
/// This example connects to the mempool and monitors for liquidity removal
/// transactions based on their function signatures.
///
/// Usage:
///    cargo run --example monitor_liquidity_removals --release

use std::time::Instant;
use std::collections::HashMap;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};
use hex;
use chrono::Local;

use mempool_processor::mempool_fetcher::NonBlockingIpcClient;
use ethers::types::{H256, Address, U256};

/// Check if transaction is a liquidity removal based on function signature
fn is_liquidity_removal(input_data: &Option<Vec<u8>>) -> Option<&'static str> {
    if let Some(data) = input_data {
        if data.len() >= 4 {
            let selector = hex::encode(&data[0..4]);
            match selector.as_str() {
                "02751cec" => Some("removeLiquidityETH"),
                "baa2abde" => Some("removeLiquidity"),
                "af2979eb" => Some("removeLiquidityETHSupportingFeeOnTransferTokens"),
                "5b0d5984" => Some("removeLiquidityETHWithPermit"),
                "ded9382a" => Some("removeLiquidityETHWithPermitSupportingFeeOnTransferTokens"),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Extract router address from transaction
fn extract_router_info(tx_data: &serde_json::Value) -> Option<(Address, &'static str)> {
    let to_str = tx_data["to"].as_str()?;
    let to_addr = to_str.parse::<Address>().ok()?;
    
    // Known Uniswap/DEX routers
    let router_str = format!("{:?}", to_addr).to_lowercase();
    match router_str.as_str() {
        "0x7a250d5630b4cf539739df2c5dacb4c659f2488d" => Some((to_addr, "Uniswap V2 Router")),
        "0xd9e1ce17f2641f24ae83637ab66a2cca9c378b9f" => Some((to_addr, "SushiSwap Router")),
        "0xe592427a0aece92de3edee1f18e0157c05861564" => Some((to_addr, "Uniswap V3 Router")),
        "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45" => Some((to_addr, "Uniswap Universal Router")),
        _ => Some((to_addr, "Unknown Router"))
    }
}

/// Parse transaction value
fn parse_tx_value(tx_data: &serde_json::Value) -> U256 {
    tx_data["value"].as_str()
        .and_then(|v| v.parse::<U256>().ok())
        .unwrap_or_default()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();
    
    info!("Liquidity Removal Monitor");
    info!("========================");
    info!("Monitoring mempool for liquidity removal transactions...\n");
    
    // Initialize IPC client
    let ipc_client = NonBlockingIpcClient::new(Some("/tmp/reth.ipc"))?;
    ipc_client.start().await?;
    
    // Statistics
    let mut total_txs = 0u64;
    let mut liquidity_removals = 0u64;
    let mut removal_types: HashMap<String, u64> = HashMap::new();
    let mut router_stats: HashMap<String, u64> = HashMap::new();
    
    let start_time = Instant::now();
    let mut last_summary = Instant::now();
    let mut ticker = interval(Duration::from_millis(100));
    
    loop {
        ticker.tick().await;
        
        // Get transactions
        match ipc_client.get_transactions(100).await {
            Ok(transactions) => {
                for tx in transactions {
                    total_txs += 1;
                    
                    // Parse input data
                    let input_data = if let Some(input_str) = tx.data["input"].as_str() {
                        let hex_str = input_str.strip_prefix("0x").unwrap_or(input_str);
                        hex::decode(hex_str).ok()
                    } else {
                        None
                    };
                    
                    // Check if it's a liquidity removal
                    if let Some(function_name) = is_liquidity_removal(&input_data) {
                        liquidity_removals += 1;
                        *removal_types.entry(function_name.to_string()).or_insert(0) += 1;
                        
                        // Extract router info
                        if let Some((router_addr, router_name)) = extract_router_info(&tx.data) {
                            *router_stats.entry(router_name.to_string()).or_insert(0) += 1;
                            
                            // Get transaction value
                            let value = parse_tx_value(&tx.data);
                            let eth_value = value.as_u128() as f64 / 1e18;
                            
                            // Log the detection
                            info!(
                                "🚨 LIQUIDITY REMOVAL DETECTED: {} {} {:.4} ETH via {}",
                                tx.hash,
                                function_name,
                                eth_value,
                                router_name
                            );
                            
                            // Decode parameters for removeLiquidityETH
                            if function_name == "removeLiquidityETH" && input_data.as_ref().unwrap().len() >= 196 {
                                let data = input_data.as_ref().unwrap();
                                let token_addr = format!("0x{}", hex::encode(&data[16..36]));
                                info!("   Token: {}", token_addr);
                                info!("   Detection latency: {}μs", tx.detection_ns / 1000);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                if !e.to_string().contains("Channel closed") {
                    error!("Error getting transactions: {}", e);
                }
            }
        }
        
        // Print summary every 30 seconds
        if last_summary.elapsed() > Duration::from_secs(30) {
            let runtime = start_time.elapsed();
            let rate = total_txs as f64 / runtime.as_secs_f64();
            
            info!("\n=== SUMMARY ===");
            info!("Runtime: {:.1} minutes", runtime.as_secs_f64() / 60.0);
            info!("Total transactions: {}", total_txs);
            info!("Transaction rate: {:.1} tx/sec", rate);
            info!("Liquidity removals detected: {} ({:.3}%)", 
                liquidity_removals, 
                (liquidity_removals as f64 / total_txs as f64) * 100.0
            );
            
            if !removal_types.is_empty() {
                info!("\nRemoval types:");
                for (func_name, count) in &removal_types {
                    info!("  {}: {}", func_name, count);
                }
            }
            
            if !router_stats.is_empty() {
                info!("\nRouter usage:");
                for (router_name, count) in &router_stats {
                    info!("  {}: {}", router_name, count);
                }
            }
            
            info!("");
            last_summary = Instant::now();
        }
    }
}