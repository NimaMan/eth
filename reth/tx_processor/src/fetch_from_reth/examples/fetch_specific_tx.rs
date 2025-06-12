//! Fetch and log specific transaction details
//!
//! This example fetches transaction 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae
//! and logs all available information about it.

use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use alloy_primitives::{B256, U256};
use eyre::Result;
use revm_tx_simulator_lib::fetch_from_reth::{
    RethDatabaseProvider, RethDataProvider, RethDataConfig
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    println!("🔍 Fetching Specific Transaction Details");
    println!("=======================================\n");
    
    // Get Reth data directory
    let datadir = get_reth_datadir()?;
    println!("📂 Using Reth data directory: {}", datadir.display());
    
    // Create provider
    let config = RethDataConfig::new(&datadir);
    let provider = RethDatabaseProvider::with_config(config)?;
    
    println!("✅ Connected to Reth database\n");
    
    // The specific transaction to fetch
    let tx_hash = B256::from_str("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")?;
    
    println!("🎯 Target Transaction: 0x{:x}", tx_hash);
    println!("{}", "=".repeat(80));
    
    // Fetch the transaction
    match provider.fetch_transaction(tx_hash) {
        Ok(tx_data) => {
            println!("\n✅ TRANSACTION FOUND!\n");
            
            // Basic Information
            println!("📋 BASIC INFORMATION:");
            println!("  Hash:         0x{:x}", tx_data.hash);
            println!("  Block Number: {} (0x{:x})", tx_data.block_number, tx_data.block_number);
            println!("  Block Hash:   0x{:x}", tx_data.block_hash);
            println!("  TX Index:     {} (position in block)", tx_data.transaction_index);
            println!("  Status:       {}", if tx_data.receipt_status { "✅ SUCCESS" } else { "❌ FAILED" });
            
            // Addresses
            println!("\n👥 ADDRESSES:");
            println!("  From:         0x{:x}", tx_data.from);
            if let Some(to) = tx_data.to {
                println!("  To:           0x{:x}", to);
                println!("  Type:         Regular Transaction");
            } else {
                println!("  To:           None (Contract Creation)");
                if let Some(contract_addr) = tx_data.contractaddress {
                    println!("  Created:      0x{:x}", contract_addr);
                }
            }
            
            // Value Transfer
            println!("\n💰 VALUE TRANSFER:");
            println!("  Value (Wei):  {}", tx_data.value);
            println!("  Value (ETH):  {:.18} ETH", wei_to_eth(tx_data.value));
            println!("  Value (USD):  (would need price oracle)");
            
            // Gas Information
            println!("\n⛽ GAS INFORMATION:");
            println!("  Gas Limit:    {} units", tx_data.gas_limit);
            println!("  Gas Used:     {} units ({:.2}% of limit)", 
                     tx_data.gas_used, 
                     (tx_data.gas_used as f64 / tx_data.gas_limit as f64) * 100.0);
            println!("  Gas Price:    {} wei ({:.2} gwei)", 
                     tx_data.gas_price, 
                     wei_to_gwei(tx_data.gas_price));
            let gas_cost_wei = U256::from(tx_data.gas_used) * tx_data.gas_price;
            println!("  Gas Cost:     {} wei ({:.6} ETH)", gas_cost_wei, wei_to_eth(gas_cost_wei));
            
            // Transaction Details
            println!("\n📝 TRANSACTION DETAILS:");
            println!("  Nonce:        {}", tx_data.nonce);
            println!("  Input Size:   {} bytes", tx_data.input.len());
            
            if !tx_data.input.is_empty() {
                // Check if it's a simple transfer or contract interaction
                if tx_data.input.len() == 0 {
                    println!("  Input Type:   Simple ETH Transfer");
                } else if tx_data.input.len() >= 4 {
                    let method_id = &tx_data.input[0..4];
                    println!("  Method ID:    0x{}", hex::encode(method_id));
                    
                    // Common method IDs
                    match hex::encode(method_id).as_str() {
                        "a9059cbb" => println!("  Method:       transfer(address,uint256) - ERC20 Transfer"),
                        "095ea7b3" => println!("  Method:       approve(address,uint256) - ERC20 Approval"),
                        "23b872dd" => println!("  Method:       transferFrom(address,address,uint256)"),
                        "18160ddd" => println!("  Method:       totalSupply()"),
                        "70a08231" => println!("  Method:       balanceOf(address)"),
                        "dd62ed3e" => println!("  Method:       allowance(address,address)"),
                        _ => println!("  Method:       Unknown (0x{})", hex::encode(method_id)),
                    }
                    
                    // Show first 100 bytes of input data
                    let display_len = std::cmp::min(100, tx_data.input.len());
                    println!("  Input Data:   0x{}{}",
                             hex::encode(&tx_data.input[..display_len]),
                             if tx_data.input.len() > 100 { "..." } else { "" });
                } else {
                    println!("  Input Data:   0x{}", hex::encode(&tx_data.input));
                }
            }
            
            // Events/Logs
            println!("\n📜 EVENTS/LOGS: {} total", tx_data.logs.len());
            for (i, log) in tx_data.logs.iter().enumerate() {
                println!("\n  Event #{}:", i + 1);
                println!("    Address:    0x{:x}", log.address);
                println!("    Topics:     {} topics", log.topics().len());
                
                for (j, topic) in log.topics().iter().enumerate() {
                    if j == 0 {
                        println!("      Topic[0]: 0x{:x} (Event Signature)", topic);
                        // Common event signatures
                        match format!("{:x}", topic).as_str() {
                            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef" => {
                                println!("                Transfer(address,address,uint256)")
                            },
                            "8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925" => {
                                println!("                Approval(address,address,uint256)")
                            },
                            _ => {}
                        }
                    } else {
                        println!("      Topic[{}]: 0x{:x}", j, topic);
                    }
                }
                
                // Log data display - simplified for now
                println!("    Data:       (log data present)");
            }
            
            // Additional Analysis
            println!("\n🔬 ADDITIONAL ANALYSIS:");
            
            // Check if it's a token transfer
            if tx_data.logs.iter().any(|log| {
                log.topics().len() >= 1 && 
                format!("{:x}", log.topics()[0]) == "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
            }) {
                println!("  ✅ Contains ERC20 Transfer event(s)");
            }
            
            // Transaction cost analysis
            let total_cost = tx_data.value + gas_cost_wei;
            println!("  Total Cost:   {} wei ({:.6} ETH)", total_cost, wei_to_eth(total_cost));
            
            // Efficiency analysis
            if tx_data.gas_used < tx_data.gas_limit / 2 {
                println!("  Efficiency:   ⚠️  Low - Used less than 50% of gas limit");
            } else if tx_data.gas_used > tx_data.gas_limit * 95 / 100 {
                println!("  Efficiency:   ⚠️  Risky - Used over 95% of gas limit");
            } else {
                println!("  Efficiency:   ✅ Good - Reasonable gas usage");
            }
            
            println!("\n{}", "=".repeat(80));
            println!("✅ Analysis complete!");
            
        }
        Err(e) => {
            println!("\n❌ TRANSACTION NOT FOUND!");
            println!("\nError: {}", e);
            println!("\nPossible reasons:");
            println!("  1. Transaction doesn't exist on mainnet");
            println!("  2. Your Reth node hasn't synced to this block yet");
            println!("  3. Database access issue");
            
            // Try to get current sync status
            if let Ok(latest_block) = provider.latest_block_number() {
                println!("\n📊 Your Reth node status:");
                println!("  Latest block: {}", latest_block);
                println!("\nIf the transaction is in a newer block, wait for sync to complete.");
            }
        }
    }
    
    Ok(())
}

/// Get Reth data directory from environment or use default
fn get_reth_datadir() -> Result<PathBuf> {
    if let Ok(datadir) = env::var("RETH_DATADIR") {
        Ok(PathBuf::from(datadir))
    } else {
        let default_dir = dirs::data_dir()
            .ok_or_else(|| eyre::eyre!("Could not determine data directory"))?
            .join("reth")
            .join("mainnet");
        
        println!("💡 RETH_DATADIR not set, using default: {}", default_dir.display());
        Ok(default_dir)
    }
}

/// Convert wei to ETH
fn wei_to_eth(wei: U256) -> f64 {
    let eth_divisor = U256::from(10u64.pow(18));
    if wei == U256::ZERO {
        return 0.0;
    }
    
    let eth_part = wei / eth_divisor;
    let remainder = wei % eth_divisor;
    
    let eth_value = eth_part.to_string().parse::<f64>().unwrap_or(0.0);
    let fractional_part = remainder.to_string().parse::<f64>().unwrap_or(0.0) / (10u64.pow(18) as f64);
    
    eth_value + fractional_part
}

/// Convert wei to gwei
fn wei_to_gwei(wei: U256) -> f64 {
    let gwei_divisor = U256::from(10u64.pow(9));
    let gwei_amount = wei / gwei_divisor;
    gwei_amount.to_string().parse::<f64>().unwrap_or(0.0)
}