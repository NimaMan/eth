use std::env;
use std::fs::File;
use std::io::Read;
use serde_json;
use tokio;
use tx_processor::tx_processor::TxProcessor;
use alloy_primitives::B256;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <tx_hashes.json>", args[0]);
        return Ok(());
    }
    
    // Read transaction hashes
    let mut file = File::open(&args[1])?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let tx_hashes: Vec<String> = serde_json::from_str(&contents)?;
    
    // Initialize processor
    let reth_datadir = env::var("RETH_DATADIR").unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let processor = TxProcessor::new(&reth_datadir)?;
    
    // Process each transaction
    let mut results = serde_json::Map::new();
    
    for (i, tx_hash) in tx_hashes.iter().enumerate() {
        if i % 50 == 0 {
            eprintln!("Rust: Processing {}/{}", i + 1, tx_hashes.len());
        }
        
        let hash_bytes = match hex::decode(tx_hash.trim_start_matches("0x")) {
            Ok(bytes) if bytes.len() == 32 => bytes,
            _ => {
                results.insert(tx_hash.clone(), serde_json::json!({"error": "Invalid hash: must be 32 bytes"}));
                continue;
            }
        };
        let hash = B256::from_slice(&hash_bytes);
        
        match processor.process_transaction_by_hash(hash).await {
            Ok(processed_tx) => {
                let result = serde_json::json!({
                    "tx_hash": tx_hash,
                    "status": processed_tx.status,
                    "block_number": processed_tx.block_number,
                    "txn_type": processed_tx.txn_type,
                    "erc20_transfers_count": processed_tx.erc20_transfers.len(),
                    "internal_transactions_count": processed_tx.internal_transactions.len(),
                    "uniswap_v2_swaps_count": processed_tx.uniswap_v2_swaps.len(),
                    "uniswap_v3_swaps_count": processed_tx.uniswap_v3_swaps.len(),
                    "gas_used": processed_tx.fees.gas_used,
                    "unique_addresses_count": processed_tx.unique_addresses.len(),
                });
                results.insert(tx_hash.clone(), result);
            }
            Err(e) => {
                results.insert(tx_hash.clone(), serde_json::json!({"error": format!("Processing error: {}", e)}));
            }
        }
    }
    
    // Output results as JSON
    println!("{}", serde_json::to_string(&results)?);
    
    Ok(())
}
