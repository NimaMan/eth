/// State Change Extraction from Mempool Transactions
/// 
/// This example shows how to extract detailed state changes from mempool
/// transactions using Direct Reth's implementation of debug_traceCall.

use mempool_processor::mempool_fetcher::{
    full_transaction_ipc_client::FullTransactionIpcClient,
};
use reth_tx_simulator::RethDirectTxSimulator;
use reth_primitives::TransactionSigned;
use alloy_rlp::Decodable;
use eyre::Result;
use tracing::{info, error};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("\n🔍 State Change Extraction from Mempool");
    println!("======================================\n");

    // Initialize
    let simulator = RethDirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    let mempool_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    mempool_client.start_monitoring().await?;
    
    info!("✅ Systems initialized\n");

    // Process a few transactions to show state changes
    let mut processed = 0;
    let target = 3;

    while processed < target {
        let transactions = mempool_client.get_full_transactions(1).await?;
        
        if transactions.is_empty() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        for tx in transactions {
            processed += 1;
            
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Transaction {}/{}: {}", processed, target, tx.hash);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            // Get and decode transaction
            let raw_tx = get_raw_tx(&tx.hash).await?;
            let signed_tx = decode_transaction(&raw_tx)?;

            // Extract transaction info
            let from = tx.tx_data["from"].as_str().unwrap_or("unknown");
            let to = tx.tx_data["to"].as_str().unwrap_or("contract_creation");
            let value = tx.tx_data["value"].as_str().unwrap_or("0x0");
            let input = tx.tx_data["input"].as_str().unwrap_or("0x");
            
            println!("\n📋 Transaction Info:");
            println!("  From:  {}", from);
            println!("  To:    {}", to);
            println!("  Value: {}", value);
            println!("  Data:  {} bytes", (input.len() - 2) / 2);

            // Simulate and extract state changes
            let start = Instant::now();
            match simulator.simulate_with_state_changes(&signed_tx).await {
                Ok(state_changes) => {
                    let elapsed = start.elapsed();
                    println!("\n✅ State extraction completed in {:?}", elapsed);
                    
                    // Analyze the state changes
                    analyze_state_changes(&state_changes);
                }
                Err(e) => {
                    error!("❌ Failed to extract state changes: {}", e);
                }
            }
            
            println!();
            
            if processed >= target {
                break;
            }
        }
    }

    println!("✅ Analysis complete!");
    Ok(())
}

fn analyze_state_changes(state_changes: &serde_json::Value) {
    if let Some(obj) = state_changes.as_object() {
        // Check for diff mode (pre/post format)
        if let (Some(pre), Some(post)) = (obj.get("pre"), obj.get("post")) {
            println!("\n📊 State Changes (Diff Mode):");
            
            let pre_accounts = pre.as_object().map(|o| o.len()).unwrap_or(0);
            let post_accounts = post.as_object().map(|o| o.len()).unwrap_or(0);
            
            println!("  Pre-state accounts:  {}", pre_accounts);
            println!("  Post-state accounts: {}", post_accounts);
            println!("  New accounts:        {}", post_accounts.saturating_sub(pre_accounts));
            
            // Analyze specific changes
            if let Some(post_obj) = post.as_object() {
                let mut total_storage_changes = 0;
                let mut contracts_with_storage = 0;
                
                for (addr, account) in post_obj.iter() {
                    if let Some(account_obj) = account.as_object() {
                        // Count storage changes
                        if let Some(storage) = account_obj.get("storage").and_then(|s| s.as_object()) {
                            if !storage.is_empty() {
                                contracts_with_storage += 1;
                                total_storage_changes += storage.len();
                            }
                        }
                        
                        // Show first few accounts in detail
                        if contracts_with_storage <= 2 && total_storage_changes > 0 {
                            println!("\n  📍 Account: {}", addr);
                            
                            if let Some(balance) = account_obj.get("balance") {
                                println!("     Balance: {}", balance);
                            }
                            
                            if let Some(storage) = account_obj.get("storage").and_then(|s| s.as_object()) {
                                println!("     Storage slots: {}", storage.len());
                                for (i, (slot, value)) in storage.iter().enumerate() {
                                    if i < 3 {
                                        println!("       {}: {}", slot, value);
                                    }
                                }
                            }
                        }
                    }
                }
                
                println!("\n  📈 Summary:");
                println!("     Contracts with storage changes: {}", contracts_with_storage);
                println!("     Total storage slots modified:   {}", total_storage_changes);
            }
        } else {
            // Simple format - just touched accounts
            println!("\n📊 Touched Accounts: {}", obj.len());
            for (i, (addr, _)) in obj.iter().enumerate() {
                if i < 5 {
                    println!("  - {}", addr);
                }
            }
            if obj.len() > 5 {
                println!("  ... and {} more", obj.len() - 5);
            }
        }
    }
}

fn decode_transaction(raw_tx: &str) -> Result<TransactionSigned> {
    let hex_str = raw_tx.strip_prefix("0x").unwrap_or(raw_tx);
    let raw_bytes = hex::decode(hex_str)?;
    Ok(TransactionSigned::decode(&mut raw_bytes.as_slice())?)
}

async fn get_raw_tx(hash: &str) -> Result<String> {
    use jsonrpsee::http_client::{HttpClientBuilder, HttpClient};
    use jsonrpsee::core::client::ClientT;
    use jsonrpsee::rpc_params;
    
    let client: HttpClient = HttpClientBuilder::default()
        .build("http://127.0.0.1:8545")?;
    
    let raw_tx: String = client.request(
        "eth_getRawTransactionByHash",
        rpc_params![hash]
    ).await?;
    
    Ok(raw_tx)
}