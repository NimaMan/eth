/// Log State Changes for One Minute
/// 
/// Simulates transactions for 60 seconds and logs all state changes to a file
///
/// Run with: cargo run --example log_state_changes_one_minute

use ethers::prelude::*;
use ethers::providers::{Provider, Http, Ws};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::fs::File;
use std::io::Write;
use chrono::Local;
use serde_json::json;
use ethers::utils::format_units;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Logging Transaction State Changes for 1 Minute");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    // Connect providers
    println!("Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(http_url)?);
    let ws_provider = Provider::<Ws>::connect(ws_url).await?;
    
    // Verify connection
    let block = http_provider.get_block_number().await?;
    println!("Connected! Current block: {}", block);
    
    // Create output file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let output_file = format!("/home/nima/code/crypto/rust/mempool_processor/src/tx_simulator/examples/state_changes_{}.log", timestamp);
    let mut file = File::create(&output_file)?;
    writeln!(file, "Transaction State Changes Log")?;
    writeln!(file, "Started at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "=")?;
    writeln!(file)?;
    
    // Subscribe to pending transactions
    println!("Subscribing to mempool...");
    let mut stream = ws_provider.subscribe_pending_txs().await?;
    println!("Subscribed! Logging transactions for 60 seconds...\n");
    
    let start_time = Instant::now();
    let mut total_count = 0;
    let mut simulated_count = 0;
    let mut failed_count = 0;
    
    // Run for 60 seconds
    while start_time.elapsed() < Duration::from_secs(60) {
        if let Some(tx_hash) = stream.next().await {
            total_count += 1;
            
            // Fetch full transaction
            match http_provider.get_transaction(tx_hash).await {
                Ok(Some(tx)) => {
                    // Create call request for simulation
                    let mut call_request = TransactionRequest::new()
                        .from(tx.from)
                        .to(tx.to.unwrap_or_default())
                        .value(tx.value)
                        .data(tx.input.clone())
                        .gas(tx.gas);
                    
                    if let Some(gas_price) = tx.gas_price {
                        call_request = call_request.gas_price(gas_price);
                    }
                    
                    // Simulate with prestateTracer to get state changes
                    match http_provider.request::<_, serde_json::Value>(
                        "debug_traceCall",
                        (call_request, "latest", json!({"tracer": "prestateTracer", "diffMode": true}))
                    ).await {
                        Ok(trace_result) => {
                            simulated_count += 1;
                            
                            // Write transaction header
                            writeln!(file, "Transaction #{}: {}", simulated_count, format!("{:#x}", tx_hash))?;
                            writeln!(file, "  From: {:#x}", tx.from)?;
                            writeln!(file, "  To: {}", tx.to.map(|a| format!("{:#x}", a)).unwrap_or_else(|| "Contract Creation".to_string()))?;
                            writeln!(file, "  Value: {} ETH", format_units(tx.value, "ether").unwrap_or_else(|_| "0".to_string()))?;
                            
                            // Parse and log state changes
                            writeln!(file, "  State Changes:")?;
                            
                            if let Some(pre) = trace_result.get("pre") {
                                if let Some(pre_obj) = pre.as_object() {
                                    for (address, account_diff) in pre_obj {
                                        writeln!(file, "    Address: {}", address)?;
                                        
                                        // Balance changes
                                        if let (Some(pre_balance), Some(post_balance)) = (
                                            account_diff.get("balance").and_then(|b| b.as_str()),
                                            trace_result.get("post")
                                                .and_then(|p| p.get(address))
                                                .and_then(|a| a.get("balance"))
                                                .and_then(|b| b.as_str())
                                        ) {
                                            let pre_bal = U256::from_str_radix(pre_balance.trim_start_matches("0x"), 16).unwrap_or_default();
                                            let post_bal = U256::from_str_radix(post_balance.trim_start_matches("0x"), 16).unwrap_or_default();
                                            
                                            if pre_bal != post_bal {
                                                let change = if post_bal > pre_bal {
                                                    format!("+{}", format_units(post_bal - pre_bal, "ether").unwrap_or_else(|_| "0".to_string()))
                                                } else {
                                                    format!("-{}", format_units(pre_bal - post_bal, "ether").unwrap_or_else(|_| "0".to_string()))
                                                };
                                                writeln!(file, "      Balance: {} ETH", change)?;
                                            }
                                        }
                                        
                                        // Storage changes
                                        if let Some(storage) = account_diff.get("storage") {
                                            if let Some(storage_obj) = storage.as_object() {
                                                if !storage_obj.is_empty() {
                                                    writeln!(file, "      Storage slots modified: {}", storage_obj.len())?;
                                                    for (slot, _) in storage_obj.iter().take(5) {
                                                        writeln!(file, "        Slot: {}", slot)?;
                                                    }
                                                    if storage_obj.len() > 5 {
                                                        writeln!(file, "        ... and {} more slots", storage_obj.len() - 5)?;
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Nonce changes
                                        if let (Some(pre_nonce), Some(post_nonce)) = (
                                            account_diff.get("nonce").and_then(|n| n.as_str()),
                                            trace_result.get("post")
                                                .and_then(|p| p.get(address))
                                                .and_then(|a| a.get("nonce"))
                                                .and_then(|n| n.as_str())
                                        ) {
                                            if pre_nonce != post_nonce {
                                                writeln!(file, "      Nonce: {} → {}", pre_nonce, post_nonce)?;
                                            }
                                        }
                                    }
                                }
                            } else if let Some(post) = trace_result.get("post") {
                                // Sometimes we only get post state for new contracts
                                if let Some(post_obj) = post.as_object() {
                                    for (address, account) in post_obj {
                                        writeln!(file, "    Address: {} (NEW)", address)?;
                                        if let Some(balance) = account.get("balance").and_then(|b| b.as_str()) {
                                            let bal = U256::from_str_radix(balance.trim_start_matches("0x"), 16).unwrap_or_default();
                                            if !bal.is_zero() {
                                                writeln!(file, "      Balance: {} ETH", format_units(bal, "ether").unwrap_or_else(|_| "0".to_string()))?;
                                            }
                                        }
                                    }
                                }
                            }
                            
                            writeln!(file)?;
                            
                            // Progress update
                            if simulated_count % 10 == 0 {
                                println!("Progress: {} transactions simulated, {} seconds elapsed", 
                                         simulated_count, start_time.elapsed().as_secs());
                            }
                        }
                        Err(e) => {
                            failed_count += 1;
                            writeln!(file, "Transaction #{}: {} - FAILED", total_count, format!("{:#x}", tx_hash))?;
                            writeln!(file, "  Error: {}", e)?;
                            writeln!(file)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    // Write summary
    writeln!(file)?;
    writeln!(file, "=")?;
    writeln!(file, "Summary:")?;
    writeln!(file, "  Total transactions seen: {}", total_count)?;
    writeln!(file, "  Successfully simulated: {}", simulated_count)?;
    writeln!(file, "  Failed simulations: {}", failed_count)?;
    writeln!(file, "  Duration: 60 seconds")?;
    writeln!(file, "  Ended at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    
    println!("\n📊 LOGGING COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total transactions seen: {}", total_count);
    println!("Successfully simulated: {}", simulated_count);
    println!("Failed simulations: {}", failed_count);
    println!("\nLog file saved to: {}", output_file);
    
    Ok(())
}