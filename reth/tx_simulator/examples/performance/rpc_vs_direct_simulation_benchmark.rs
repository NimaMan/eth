/// Real Performance Benchmark: RPC vs Direct Reth
///
/// This benchmark compares identical unsigned simulation operations:
/// - RPC: debug_traceCall with call tracer (unsigned simulation)
/// - Direct: simulate_unsigned_transaction_with_trace (unsigned simulation)
/// 
/// Both methods simulate unsigned calls with automatic nonce resolution,
/// providing a true apples-to-apples performance comparison.

use tx_simulator::{TxSimulator, CallRequest};
use eyre::Result;
use tracing::{info, warn};
use std::time::{Duration, Instant};
use std::env;

// For fetching transactions and RPC calls
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;
use reth_primitives::{TransactionSigned, transaction::SignedTransaction};
use alloy_consensus::transaction::Transaction;
use alloy_rlp::Decodable;
use serde_json::{Value, json};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🎯 Real Performance Benchmark: RPC vs Direct Reth");
    info!("=================================================");
    info!("Comparing IDENTICAL operations: state change extraction");
    
    // Initialize Direct Reth simulator
    let start = Instant::now();
    let simulator = TxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let init_time = start.elapsed();
    info!("\n✅ Direct Reth initialized in {:?}", init_time);
    
    let latest_block = simulator.get_latest_block()?;
    info!("📊 Latest block: {} (from local DB)", latest_block);
    
    // Connect to RPC 
    let rpc_client = HttpClientBuilder::default()
        .build("http://127.0.0.1:8545")?;
    
    // Collect real transactions from recent blocks
    info!("\n📥 Collecting real transactions...");
    let collect_start = Instant::now();
    
    let mut transactions = Vec::new();
    let mut block_num = latest_block;
    // Get transaction count from command line argument, default to 1000
    let target_count = env::args()
        .nth(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);
    
    info!("📊 Target transaction count: {}", target_count);
    
    while transactions.len() < target_count && block_num > latest_block - 100 {
        let block_result: Value = rpc_client.request(
            "eth_getBlockByNumber",
            rpc_params![format!("0x{:x}", block_num), true]
        ).await?;
        
        if let Some(txs) = block_result["transactions"].as_array() {
            info!("   Block {}: {} transactions", block_num, txs.len());
            
            for tx in txs.iter() { // Take all transactions from each block
                if let Some(hash) = tx["hash"].as_str() {
                    // Get raw transaction for Direct simulation
                    let raw_tx: String = rpc_client.request(
                        "eth_getRawTransactionByHash",
                        rpc_params![hash]
                    ).await?;
                    
                    // Decode to TransactionSigned
                    let hex_str = raw_tx.strip_prefix("0x").unwrap_or(&raw_tx);
                    if let Ok(raw_bytes) = hex::decode(hex_str) {
                        if let Ok(signed_tx) = TransactionSigned::decode(&mut raw_bytes.as_slice()) {
                            // Store both transaction data and block info
                            transactions.push((
                                hash.to_string(),
                                signed_tx,
                                tx.clone(), // Original tx data for RPC call
                                block_num - 1, // Use parent block for simulation
                            ));
                            
                            if transactions.len() >= target_count {
                                break;
                            }
                        }
                    }
                }
            }
        }
        block_num -= 1;
    }
    
    let collect_time = collect_start.elapsed();
    info!("✅ Collected {} transactions in {:?}", transactions.len(), collect_time);
    
    if transactions.is_empty() {
        return Err(eyre::eyre!("No valid transactions found"));
    }

    // Test 1: RPC debug_traceCall with call tracer
    info!("\n📊 RPC Benchmark (debug_traceCall with call tracer)");
    info!("   Testing {} transactions...", transactions.len());
    
    let mut rpc_times = Vec::new();
    let mut rpc_successes = 0;
    
    for (i, (_hash, _signed_tx, tx_data, sim_block)) in transactions.iter().enumerate() {
        let start = Instant::now();
        
        // Create call request from transaction data, handling different tx types
        let mut call_request = json!({
            "from": tx_data["from"],
            "to": tx_data["to"],
            "gas": tx_data["gas"],
            "value": tx_data["value"],
            "data": tx_data["input"],
            "nonce": tx_data["nonce"]
        });
        
        // Handle gas pricing based on transaction type
        if tx_data["type"].as_str() == Some("0x2") {
            // EIP-1559 transaction
            call_request["maxFeePerGas"] = tx_data["maxFeePerGas"].clone();
            call_request["maxPriorityFeePerGas"] = tx_data["maxPriorityFeePerGas"].clone();
        } else {
            // Legacy transaction
            call_request["gasPrice"] = tx_data["gasPrice"].clone();
        }
        
        // RPC call with call tracer (same as our Direct method)
        let rpc_result: Result<Value, _> = rpc_client.request(
            "debug_traceCall",
            rpc_params![
                call_request,
                format!("0x{:x}", sim_block),
                json!({"tracer": "callTracer"})
            ]
        ).await;
        
        let elapsed = start.elapsed();
        
        match rpc_result {
            Ok(_) => {
                rpc_times.push(elapsed);
                rpc_successes += 1;
                if i < 3 {
                    info!("   TX {}: {:?} ✅", i + 1, elapsed);
                }
            }
            Err(e) => {
                warn!("   TX {}: Failed - {}", i + 1, e);
            }
        }
    }
    
    let avg_rpc_time = if !rpc_times.is_empty() {
        rpc_times.iter().sum::<Duration>() / rpc_times.len() as u32
    } else {
        Duration::from_millis(0)
    };
    
    info!("   RPC Results: {} successes, average: {:?}", rpc_successes, avg_rpc_time);
    
    // Test 2: Direct Reth with call tracer (same functionality)
    info!("\n🚀 Direct Reth Benchmark (simulate_transaction_with_call_trace)");
    info!("   Testing {} transactions...", transactions.len());
    
    let mut direct_times = Vec::new();
    let mut direct_successes = 0;
    
    for (i, (_hash, signed_tx, tx_data, sim_block)) in transactions.iter().enumerate() {
        let start = Instant::now();
        
        // Convert signed transaction to CallRequest for unsigned simulation (like RPC does)
        let call_request = match signed_tx.recover_signer() {
            Ok(sender) => {
                // Create CallRequest without nonce - let it be determined automatically
                let mut request = CallRequest {
                    from: Some(sender),
                    to: signed_tx.to(),
                    data: Some(signed_tx.input().clone()),
                    value: Some(signed_tx.value()),
                    // Get gas from tx_data JSON
                    gas: tx_data["gas"].as_str().and_then(|g| u64::from_str_radix(g.trim_start_matches("0x"), 16).ok()),
                    // Don't set nonce - let it be determined from DB
                    nonce: None,
                    ..Default::default()
                };
                
                // Handle gas pricing based on transaction type
                if tx_data["type"].as_str() == Some("0x2") {
                    // EIP-1559 transaction
                    if let Some(max_fee) = tx_data["maxFeePerGas"].as_str() {
                        request.max_fee_per_gas = Some(u128::from_str_radix(max_fee.trim_start_matches("0x"), 16).unwrap_or(0));
                    }
                    if let Some(priority_fee) = tx_data["maxPriorityFeePerGas"].as_str() {
                        request.max_priority_fee_per_gas = Some(u128::from_str_radix(priority_fee.trim_start_matches("0x"), 16).unwrap_or(0));
                    }
                } else {
                    // Legacy transaction
                    if let Some(gas_price) = tx_data["gasPrice"].as_str() {
                        request.gas_price = Some(u128::from_str_radix(gas_price.trim_start_matches("0x"), 16).unwrap_or(0));
                    }
                }
                
                Some(request)
            }
            Err(e) => {
                warn!("   TX {}: Failed to recover signer - {}", i + 1, e);
                None
            }
        };
        
        // Direct simulation with unsigned call (matching RPC behavior)
        let direct_result = if let Some(call_request) = call_request {
            simulator
                .simulate_unsigned_transaction_with_trace(call_request, Some(*sim_block))
                .await
        } else {
            continue;
        };
        
        let elapsed = start.elapsed();
        
        match direct_result {
            Ok(result) => {
                direct_times.push(elapsed);
                direct_successes += 1;
                if i < 3 {
                    info!("   TX {}: {:?} ✅ (gas: {}, logs: {})", i + 1, elapsed, result.gas_used, result.call_trace.logs.len());
                }
            }
            Err(e) => {
                warn!("   TX {}: Failed - {}", i + 1, e);
            }
        }
    }
    
    let avg_direct_time = if !direct_times.is_empty() {
        direct_times.iter().sum::<Duration>() / direct_times.len() as u32
    } else {
        Duration::from_millis(0)
    };
    
    info!("   Direct Results: {} successes, average: {:?}", direct_successes, avg_direct_time);
    
    // Performance Analysis
    info!("\n📈 PERFORMANCE ANALYSIS");
    info!("======================");
    
    if rpc_successes > 0 && direct_successes > 0 {
        info!("Successful Transactions:");
        info!("   RPC (debug_traceCall):   {} of {}", rpc_successes, transactions.len());
        info!("   Direct Reth:             {} of {}", direct_successes, transactions.len());
        
        info!("\nLatency Comparison:");
        info!("   RPC Average:    {:?}", avg_rpc_time);
        info!("   Direct Average: {:?}", avg_direct_time);
        
        if avg_rpc_time.as_micros() > 0 && avg_direct_time.as_micros() > 0 {
            let speedup = avg_rpc_time.as_micros() as f64 / avg_direct_time.as_micros() as f64;
            info!("   Measured Speedup: {:.1}x", speedup);
            
            if speedup > 1.0 {
                info!("   ✅ Direct Reth is faster");
            } else if speedup < 1.0 {
                info!("   ⚠️  RPC is faster");
            } else {
                info!("   ⚖️  Similar performance");
            }
        }
        
        // Latency distribution for Direct (more detailed)
        if direct_times.len() > 5 {
            let mut sorted_direct = direct_times.clone();
            sorted_direct.sort();
            
            let min_direct = sorted_direct.first().unwrap();
            let p50_direct = &sorted_direct[sorted_direct.len() / 2];
            let p95_direct = &sorted_direct[sorted_direct.len() * 95 / 100];
            let max_direct = sorted_direct.last().unwrap();
            
            info!("\nDirect Reth Latency Distribution:");
            info!("   Min:  {:?}", min_direct);
            info!("   P50:  {:?}", p50_direct);
            info!("   P95:  {:?}", p95_direct);
            info!("   Max:  {:?}", max_direct);
        }
        
        // Throughput calculation
        let rpc_throughput = 1_000_000.0 / avg_rpc_time.as_micros() as f64;
        let direct_throughput = 1_000_000.0 / avg_direct_time.as_micros() as f64;
        
        info!("\nThroughput (transactions/second):");
        info!("   RPC:    {:.0} tx/sec", rpc_throughput);
        info!("   Direct: {:.0} tx/sec", direct_throughput);
        
    } else {
        warn!("Insufficient successful transactions for comparison");
        if rpc_successes == 0 {
            warn!("All RPC calls failed - check if Reth debug RPC is enabled");
        }
        if direct_successes == 0 {
            warn!("All Direct simulations failed - check database access");
        }
    }
    
    info!("\n✅ Benchmark Complete!");
    info!("\n📝 Test Conditions:");
    info!("   - Same {} transactions tested via both methods", target_count);
    info!("   - Both methods simulate unsigned calls (no signature/nonce validation)");
    info!("   - RPC: debug_traceCall with callTracer");
    info!("   - Direct: simulate_unsigned_transaction_with_trace");
    info!("   - Simulator initialized once ({}ms), then reused", init_time.as_millis());
    info!("   - Measured: Network + processing time vs processing only");
    
    Ok(())
}