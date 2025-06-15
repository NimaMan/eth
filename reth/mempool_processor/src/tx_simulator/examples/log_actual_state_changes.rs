/// Log Actual State Changes with debug_traceCall State Diff Calculator
/// 
/// Uses the debug_traceCall State Diff Calculator to get real state changes for each address
/// Treats WETH transfers as ETH balance changes (exact same logic as Python version)
///
/// Run with: cargo run --example log_actual_state_changes

use mempool_processor::mempool_fetcher::{WebSocketClient, TransactionView};
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("mempool_processor=info")
        .init();

    println!("🚀 Logging ACTUAL Transaction State Changes for 5 Minutes");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let ws_url = "ws://127.0.0.1:8546";
    let http_url = "http://127.0.0.1:8545";
    
    // Connect to provider
    println!("Connecting to Ethereum node...");
    let http_provider = Arc::new(Provider::<Http>::try_from(http_url)?);
    
    // Get current block
    let block_number = http_provider.get_block_number().await?;
    let latest_block = http_provider.get_block(ethers::types::BlockNumber::Latest).await?
        .ok_or_else(|| eyre::eyre!("Failed to get latest block"))?;
    
    println!("Connected! Current block: {}", block_number);
    
    // Initialize debug_traceCall State Diff Calculator
    println!("Initializing debug_traceCall State Diff Calculator (WETH=ETH)...");
    let mut state_calculator = DebugTraceCallStateDiffCalculator::default();
    
    // Connect to mempool
    println!("Connecting to mempool via WebSocket...");
    let ws_client = WebSocketClient::new(ws_url, http_url)?;
    ws_client.start_monitoring().await?;
    
    // Create output file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let output_file = format!("/home/nima/code/crypto/rust/mempool_processor/src/tx_simulator/examples/actual_state_changes_{}.log", timestamp);
    let mut file = File::create(&output_file)?;
    writeln!(file, "ACTUAL Transaction State Changes Log")?;
    writeln!(file, "Started at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file, "=")?;
    writeln!(file)?;
    
    println!("Starting simulation for 5 minutes...\n");
    
    let start_time = Instant::now();
    let mut total_count = 0;
    let mut simulated_count = 0;
    let mut total_latency_ms = 0.0;
    let mut latency_count = 0;
    
    // Run for 5 minutes (300 seconds)
    while start_time.elapsed() < Duration::from_secs(300) {
        match tokio::time::timeout(
            Duration::from_millis(100),
            ws_client.get_transactions(5)
        ).await {
            Ok(Ok(transactions)) => {
                for ws_tx in transactions {
                    total_count += 1;
                    let tx_hash = ws_tx.hash.clone();
                    
                    // Start timing from when we receive the transaction
                    let tx_arrival_time = Instant::now();
                    
                    // Fetch full transaction
                    let tx_hash_h256 = match tx_hash.parse::<H256>() {
                        Ok(h) => h,
                        Err(_) => continue,
                    };
                    
                    match http_provider.get_transaction(tx_hash_h256).await {
                        Ok(Some(tx)) => {
                            // Convert to TransactionView
                            let tx_view = TransactionView {
                                hash: tx.hash.as_bytes().to_vec(),
                                from: tx.from.as_bytes().to_vec(),
                                to: tx.to.map(|addr| addr.as_bytes().to_vec()),
                                value: tx.value,
                                gas_price: tx.gas_price,
                                gas_limit: Some(tx.gas),
                                nonce: Some(U256::from(tx.nonce.as_u64())),
                                input_data: Some(tx.input.to_vec()),
                            };
                            
                            // Use debug_traceCall to simulate the pending transaction and get logs
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
                                    // Parse logs from trace result
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
                                    
                                    // Skip gas fee calculation - we only care about token transfers
                                    
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
                                    
                                    // Calculate state changes using our calculator (with WETH=ETH logic)
                                    match state_calculator.calculate_state_changes_from_transfers(
                                        tx.from,
                                        block_number.as_u64(),
                                        total_count,
                                        &eth_transfers,
                                        &erc20_transfers,
                                    ) {
                                        Ok(state_changes) => {
                                            if !state_changes.is_empty() {
                                                simulated_count += 1;
                                                
                                                // Calculate latency
                                                let processing_time = tx_arrival_time.elapsed();
                                                let latency_ms = processing_time.as_secs_f64() * 1000.0;
                                                total_latency_ms += latency_ms;
                                                latency_count += 1;
                                                
                                                // Write transaction header
                                                writeln!(file, "Transaction #{}: {}", simulated_count, tx_hash)?;
                                                writeln!(file, "  From: {:#x}", tx.from)?;
                                                writeln!(file, "  To: {}", 
                                                         tx.to.map(|a| format!("{:#x}", a))
                                                              .unwrap_or_else(|| "Contract Creation".to_string()))?;
                                                writeln!(file, "  Value: {} ETH", 
                                                         ethers::utils::format_units(tx.value, "ether")
                                                             .unwrap_or_else(|_| "0".to_string()))?;
                                                writeln!(file, "  Processing Latency: {:.2} ms", latency_ms)?;
                                                
                                                writeln!(file, "  State Changes ({} addresses affected):", state_changes.len())?;
                                                
                                                // Log state changes for each address
                                                for (address, changes) in &state_changes {
                                                    writeln!(file, "    Address: {:#x}", address)?;
                                                    
                                                    // Log ETH balance changes (including WETH treated as ETH)
                                                    if changes.eth_net != 0.0 {
                                                        writeln!(file, "      ETH: {:+.18} ETH", changes.eth_net)?;
                                                    }
                                                    
                                                    // Log token balance changes
                                                    for (token_key, token_change) in &changes.token_net {
                                                        writeln!(file, "      Token {}: {:+.6}", token_key, token_change)?;
                                                    }
                                                }
                                                
                                                writeln!(file)?;
                                                
                                                // Progress update
                                                if simulated_count <= 5 || simulated_count % 10 == 0 {
                                                    println!("TX {}: {} - {} addresses affected - {:.2} ms", 
                                                             simulated_count, &tx_hash[..10], state_changes.len(), latency_ms);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            writeln!(file, "Transaction #{}: {} - FAILED: {}", total_count, tx_hash, e)?;
                                            writeln!(file)?;
                                        }
                                    }
                                }
                                Err(e) => {
                                    writeln!(file, "Transaction #{}: {} - FAILED debug_traceCall: {}", total_count, tx_hash, e)?;
                                    writeln!(file)?;
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
    writeln!(file, "  Successfully simulated: {}", simulated_count)?;
    writeln!(file, "  Duration: 5 minutes")?;
    if latency_count > 0 {
        let avg_latency = total_latency_ms / latency_count as f64;
        writeln!(file, "  Average processing latency: {:.2} ms", avg_latency)?;
        writeln!(file, "  Min/Max latency: (shown in individual transactions)")?;
    }
    writeln!(file, "  Ended at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    
    println!("\n📊 LOGGING COMPLETE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total transactions seen: {}", total_count);
    println!("Successfully simulated: {}", simulated_count);
    if latency_count > 0 {
        let avg_latency = total_latency_ms / latency_count as f64;
        println!("Average processing latency: {:.2} ms", avg_latency);
    }
    println!("\nLog file saved to: {}", output_file);
    
    Ok(())
}