/// Simple test that fetch_block_transactions_and_receipts functionality works
/// 
/// Run with: cargo run --example fetch_block_tx_and_receipts_simple

use reth_chain_query::{RethQueryProvider, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing fetch_block_transactions_and_receipts functionality...\n");
    
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    let block_number = provider.get_latest_block()? - 10;
    
    // Test fetching block with transactions
    let block = provider.get_block_with_txs(block_number).await?;
    println!("✅ Block #{} has {} transactions", block_number, block.transactions.len());
    
    // Test fetching metadata
    let metadata = provider.fetch_block_tx_metadata_only(block_number).await?;
    println!("✅ Got {} transaction metadata", metadata.len());
    
    // Test fetching receipts
    let receipts = provider.fetch_block_receipts_only(block_number).await?;
    println!("✅ Got {} receipts", receipts.len());
    
    // Basic analysis
    let failed = receipts.iter().filter(|r| !r.status).count();
    println!("✅ Success rate: {}/{}", receipts.len() - failed, receipts.len());
    
    Ok(())
}