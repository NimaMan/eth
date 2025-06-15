/// Monitor Pool State Changes from Mempool
/// 
/// This example demonstrates how to:
/// 1. Subscribe to Python pool data via ZeroMQ
/// 2. Monitor mempool for transactions involving those pools
/// 3. Calculate comprehensive state changes including complex contract interactions
/// 4. Log all state changes for pool-related transactions
///
/// Run with: cargo run --example monitor_pool_state_changes

use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
use mempool_processor::pool_subscriber::PoolSubscriber;
use mempool_processor::tx_simulator::debug_tracecall_state_diff_calculator::{
    DebugTraceCallStateDiffCalculator, EthTransfer, Erc20Transfer
};
use ethers::prelude::*;
use ethers::providers::{Provider, Http};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::fs::File;
use std::io::Write;
use chrono::Local;
use eyre::Result;
use tracing::{info, warn, debug};
use std::collections::HashSet;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("monitor_pool_state_changes=info,mempool_processor=info")
        .init();

    println!("🏊 Pool State Change Monitor");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    let zmq_url = "tcp://localhost:5557";
    
    // Connect to provider
    println!("Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(http_url)?);
    
    // Get current block
    let block_number = http_provider.get_block_number().await?;
    println!("Connected! Current block: {}", block_number);
    
    // Initialize pool subscriber
    println!("Initializing pool subscriber...");
    let mut pool_subscriber = PoolSubscriber::with_endpoint(0.1, zmq_url);
    let pool_cache = pool_subscriber.get_pool_cache();
    
    // Start pool subscriber in background
    let pool_handle = tokio::spawn(async move {
        info!("Starting pool subscriber listener...");
        if let Err(e) = pool_subscriber.start_listening().await {
            warn!("Pool subscriber error: {}", e);
        }
    });
    
    // Wait a bit for initial pool data
    println!("Waiting for pool data...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let pool_count = pool_cache.get_pool_count();
    println!("Received {} pools from Python", pool_count);
    
    if pool_count == 0 {
        println!("⚠️  No pools received. Make sure Python pool publisher is running!");
        println!("   Run: python /home/nima/code/crypto/py/eth_db_loader/send_pools_to_rust.py");
        return Ok(());
    }
    
    // Get all pool addresses (checksummed)
    let all_pools = pool_cache.get_all_pools();
    let pool_addresses: HashSet<String> = all_pools.keys().cloned().collect();
    
    // Show some example pools
    println!("\nExample pools being monitored:");
    for (i, (addr, pool)) in all_pools.iter().enumerate().take(10) {
        println!("  {}: {} - {:.4} ETH liquidity", i+1, addr, pool.eth_reserve);
    }
    println!("\nTotal pools: {}", pool_addresses.len());
    
    // Debug: show first pool address format
    if let Some(first_addr) = pool_addresses.iter().next() {
        println!("First pool address format: {} (length: {})", first_addr, first_addr.len());
    }
    
    // Initialize debug_traceCall State Diff Calculator
    println!("\nInitializing state diff calculator...");
    let mut state_calculator = DebugTraceCallStateDiffCalculator::default();
    
    // Connect to mempool
    println!("Connecting to mempool via WebSocket...");
    let ws_client = WebSocketClient::new(ws_url, http_url)?;
    ws_client.start_monitoring().await?;
    
    // Create output file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let output_file = format!("/home/nima/code/crypto/logs/pool_state_changes_{}.log", timestamp);
    let mut file = File::create(&output_file)?;
    writeln!(file, "Pool State Changes from Mempool")?;
    writeln!(file, "Started at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "Monitoring {} pools", pool_count)?;
    writeln!(file, "=")?;
    writeln!(file)?;
    
    println!("\n🚀 Starting monitoring for 5 minutes...\n");
    
    let start_time = Instant::now();
    let mut total_count = 0;
    let mut pool_tx_count = 0;
    let mut transactions_with_pools = 0;
    let mut complex_tx_count = 0;
    
    // Run for 5 minutes
    while start_time.elapsed() < Duration::from_secs(300) {
        match tokio::time::timeout(
            Duration::from_millis(100),
            ws_client.get_transactions(10)
        ).await {
            Ok(Ok(transactions)) => {
                for ws_tx in transactions {
                    total_count += 1;
                    let tx_hash = ws_tx.hash.clone();
                    
                    // Progress update every 100 transactions
                    if total_count % 100 == 0 {
                        let elapsed = start_time.elapsed().as_secs();
                        println!("⏱️  Progress: {} txs processed in {}s - {} pool txs, {} complex txs", 
                                 total_count, elapsed, pool_tx_count, complex_tx_count);
                    }
                    
                    // Fetch full transaction
                    let tx_hash_h256 = match tx_hash.parse::<H256>() {
                        Ok(h) => h,
                        Err(_) => continue,
                    };
                    
                    match http_provider.get_transaction(tx_hash_h256).await {
                        Ok(Some(tx)) => {
                            
                            // Use debug_traceCall to simulate and get all state changes
                            let call_request = serde_json::json!({
                                "from": format!("{:#x}", tx.from),
                                "to": tx.to.map(|addr| format!("{:#x}", addr)),
                                "value": format!("{:#x}", tx.value),
                                "data": format!("0x{}", hex::encode(&tx.input)),
                                "gas": format!("{:#x}", tx.gas),
                                "gasPrice": format!("{:#x}", tx.gas_price.unwrap_or_default())
                            });
                            
                            match http_provider.request::<_, serde_json::Value>(
                                "debug_traceCall",
                                (call_request, "latest", serde_json::json!({"tracer": "callTracer", "tracerConfig": {"withLog": true}}))
                            ).await {
                                Ok(trace_result) => {
                                    // Parse all logs and transfers recursively
                                    let mut eth_transfers = Vec::new();
                                    let mut erc20_transfers = Vec::new();
                                    
                                    // Helper function to parse logs from any level of the trace
                                    fn parse_logs_recursive(
                                        trace: &serde_json::Value,
                                        eth_transfers: &mut Vec<EthTransfer>,
                                        erc20_transfers: &mut Vec<Erc20Transfer>,
                                        depth: u32,
                                    ) {
                                        // Parse logs at this level
                                        if let Some(logs) = trace.get("logs").and_then(|l| l.as_array()) {
                                            let transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
                                            
                                            for (log_index, log_entry) in logs.iter().enumerate() {
                                                if let (Some(topics), Some(data), Some(address)) = (
                                                    log_entry.get("topics").and_then(|t| t.as_array()),
                                                    log_entry.get("data").and_then(|d| d.as_str()),
                                                    log_entry.get("address").and_then(|a| a.as_str())
                                                ) {
                                                    if topics.len() >= 3 
                                                        && topics[0].as_str().unwrap_or("") == transfer_topic {
                                                        
                                                        let from_addr: Address = topics[1].as_str().unwrap_or("0x0000000000000000000000000000000000000000")[26..].parse().unwrap_or_default();
                                                        let to_addr: Address = topics[2].as_str().unwrap_or("0x0000000000000000000000000000000000000000")[26..].parse().unwrap_or_default();
                                                        let token_addr: Address = address.parse().unwrap_or_default();
                                                        
                                                        if let Ok(amount_bytes) = hex::decode(data.trim_start_matches("0x")) {
                                                            let amount_raw = U256::from_big_endian(&amount_bytes);
                                                            
                                                            erc20_transfers.push(Erc20Transfer {
                                                                token_address: token_addr,
                                                                from_address: from_addr,
                                                                to_address: to_addr,
                                                                amount: amount_raw.as_u128() as f64,
                                                                log_index: (depth * 1000 + log_index as u32) as u64,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Check for ETH transfers in this call
                                        if let (Some(from), Some(to), Some(value)) = (
                                            trace.get("from").and_then(|f| f.as_str()),
                                            trace.get("to").and_then(|t| t.as_str()),
                                            trace.get("value").and_then(|v| v.as_str())
                                        ) {
                                            if let (Ok(from_addr), Ok(to_addr)) = (from.parse::<Address>(), to.parse::<Address>()) {
                                                if let Ok(value_u256) = U256::from_str_radix(value.trim_start_matches("0x"), 16) {
                                                    if !value_u256.is_zero() {
                                                        let value_eth = value_u256.as_u128() as f64 / 1e18;
                                                        eth_transfers.push(EthTransfer {
                                                            from_address: from_addr,
                                                            to_address: to_addr,
                                                            amount: value_eth,
                                                            log_index: Some(999990 + depth as u64),
                                                            depth: Some(depth as u64),
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Recursively parse internal calls
                                        if let Some(calls) = trace.get("calls").and_then(|c| c.as_array()) {
                                            for call in calls {
                                                parse_logs_recursive(call, eth_transfers, erc20_transfers, depth + 1);
                                            }
                                        }
                                    }
                                    
                                    // Add top-level value transfer if any
                                    if !tx.value.is_zero() {
                                        let value_eth = tx.value.as_u128() as f64 / 1e18;
                                        if let Some(to_addr) = tx.to {
                                            eth_transfers.push(EthTransfer {
                                                from_address: tx.from,
                                                to_address: to_addr,
                                                amount: value_eth,
                                                log_index: Some(999998),
                                                depth: Some(0),
                                            });
                                        }
                                    }
                                    
                                    // Parse all logs and transfers recursively
                                    parse_logs_recursive(&trace_result, &mut eth_transfers, &mut erc20_transfers, 0);
                                    
                                    // Calculate state changes
                                    match state_calculator.calculate_state_changes_from_transfers(
                                        tx.from,
                                        block_number.as_u64(),
                                        pool_tx_count,
                                        &eth_transfers,
                                        &erc20_transfers,
                                    ) {
                                        Ok(state_changes) => {
                                            if !state_changes.is_empty() {
                                                // Check which addresses in state changes are pools
                                                let mut pool_addresses_affected = Vec::new();
                                                let mut involves_pool = false;
                                                
                                                for (addr, _) in &state_changes {
                                                    let addr_checksum = ethers::utils::to_checksum(addr, None);
                                                    if pool_addresses.contains(&addr_checksum) {
                                                        pool_addresses_affected.push(addr_checksum);
                                                        involves_pool = true;
                                                    }
                                                }
                                                
                                                // Also check if any token transfers involve pool tokens
                                                for (_, changes) in &state_changes {
                                                    for (token_addr, _) in &changes.token_net {
                                                        // Check if this token belongs to any of our pools
                                                        for pool in all_pools.values() {
                                                            if pool.token_address.eq_ignore_ascii_case(token_addr) {
                                                                involves_pool = true;
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                // Log all transactions with >3 addresses affected
                                                if state_changes.len() > 3 {
                                                    if involves_pool {
                                                        pool_tx_count += 1;
                                                        transactions_with_pools += 1;
                                                        
                                                        // Write to log
                                                        writeln!(file, "Transaction #{}: {} [POOL INVOLVED]", transactions_with_pools, tx_hash)?;
                                                    } else {
                                                        complex_tx_count += 1;
                                                        // Still log complex transactions even without pools for debugging
                                                        writeln!(file, "Transaction (Complex, No Pool): {}", tx_hash)?;
                                                    }
                                                    
                                                    writeln!(file, "  From: {:#x}", tx.from)?;
                                                    writeln!(file, "  To: {}", 
                                                             tx.to.map(|a| format!("{:#x}", a))
                                                                  .unwrap_or_else(|| "Contract Creation".to_string()))?;
                                                    writeln!(file, "  Value: {} ETH", 
                                                             ethers::utils::format_units(tx.value, "ether")
                                                                 .unwrap_or_else(|_| "0".to_string()))?;
                                                    writeln!(file, "  Pool addresses in state changes: {:?}", pool_addresses_affected)?;
                                                    writeln!(file, "  Total addresses affected: {}", state_changes.len())?;
                                                    writeln!(file, "  State Changes:")?;
                                                
                                                // Log state changes with pool indicators
                                                for (address, changes) in &state_changes {
                                                    let addr_checksum = ethers::utils::to_checksum(address, None);
                                                    let is_pool = pool_addresses.contains(&addr_checksum);
                                                    
                                                    writeln!(file, "    Address: {:#x} {}", 
                                                             address, 
                                                             if is_pool { "🏊 [POOL]" } else { "" })?;
                                                    
                                                    // Show pool info if it's a pool
                                                    if is_pool {
                                                        if let Some(pool_info) = pool_cache.get_pool(&addr_checksum) {
                                                            writeln!(file, "      Pool liquidity: {:.4} ETH", pool_info.eth_reserve)?;
                                                        }
                                                    }
                                                    
                                                    // Log ETH balance changes
                                                    if changes.eth_net != 0.0 {
                                                        writeln!(file, "      ETH: {:+.18} ETH", changes.eth_net)?;
                                                    }
                                                    
                                                    // Log token balance changes
                                                    for (token_key, token_change) in &changes.token_net {
                                                        writeln!(file, "      Token {}: {:+.6}", token_key, token_change)?;
                                                    }
                                                }
                                                
                                                writeln!(file)?;
                                                
                                                    // Console output
                                                    if involves_pool {
                                                        println!("🏊 Pool TX #{}: {} - {} pools, {} addresses affected", 
                                                                 transactions_with_pools, 
                                                                 &tx_hash[..10], 
                                                                 pool_addresses_affected.len(),
                                                                 state_changes.len());
                                                    } else {
                                                        println!("📊 Complex TX: {} - {} addresses affected (no pool)", 
                                                                 &tx_hash[..10], 
                                                                 state_changes.len());
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            debug!("Failed to calculate state changes: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Failed debug_traceCall: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {} // Timeout, continue
        }
    }
    
    // Write summary
    writeln!(file)?;
    writeln!(file, "=")?;
    writeln!(file, "Summary:")?;
    writeln!(file, "  Total transactions seen: {}", total_count)?;
    writeln!(file, "  Complex transactions (>3 addresses): {}", pool_tx_count + complex_tx_count)?;
    writeln!(file, "    - With pool involvement: {}", pool_tx_count)?;
    writeln!(file, "    - Without pool involvement: {}", complex_tx_count)?;
    writeln!(file, "  Pools monitored: {}", pool_count)?;
    writeln!(file, "  Duration: 5 minutes")?;
    writeln!(file, "  Ended at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    
    println!("\n📊 MONITORING COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total transactions seen: {}", total_count);
    println!("Complex transactions (>3 addresses): {}", pool_tx_count + complex_tx_count);
    println!("  - With pool involvement: {}", pool_tx_count);
    println!("  - Without pool involvement: {}", complex_tx_count);
    println!("Pools monitored: {}", pool_count);
    println!("\nLog file saved to: {}", output_file);
    
    Ok(())
}