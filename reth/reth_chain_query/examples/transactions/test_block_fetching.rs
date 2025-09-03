/// Simple test of block transaction fetching methods
/// 
/// Tests the core functionality:
/// 1. Fetch block with transactions
/// 2. Fetch receipts only
/// 3. Fetch metadata only
/// 4. Fetch hashes only (fastest)
/// 
/// Run with: cargo run --example test_block_fetching

use reth_chain_query::{RethQueryProvider, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing Block Transaction Fetching Methods ===\n");
    
    // Initialize provider
    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Get a recent block number
    let latest_block = provider.get_latest_block()?;
    let test_block = latest_block.saturating_sub(10); // Use block from 10 blocks ago
    
    println!("Testing with block #{}\n", test_block);
    
    // 1. Test fetching block with transactions
    println!("1. Testing get_block_with_txs()...");
    let block = provider.get_block_with_txs(test_block).await?;
    println!("   ✅ Block #{} has {} transactions", block.header.number, block.transactions.len());
    println!("   Timestamp: {}", block.header.timestamp);
    println!("   Gas used: {}/{}", block.header.gas_used, block.header.gas_limit);
    
    // 2. Test fetching receipts only
    println!("\n2. Testing fetch_block_receipts_only()...");
    let receipts = provider.fetch_block_receipts_only(test_block).await?;
    println!("   ✅ Got {} receipts", receipts.len());
    if let Some(first_receipt) = receipts.first() {
        println!("   First receipt: gas_used={}, status={}", first_receipt.gas_used, first_receipt.status);
    }
    
    // 3. Test fetching transaction metadata only
    println!("\n3. Testing fetch_block_tx_metadata_only()...");
    let metadata = provider.fetch_block_tx_metadata_only(test_block).await?;
    println!("   ✅ Got {} transaction metadata entries", metadata.len());
    if let Some(first_tx) = metadata.first() {
        println!("   First tx: from=0x{:x}, value={}", first_tx.from, first_tx.value);
    }
    
    // 4. Test fetching hashes only (most efficient)
    println!("\n4. Testing fetch_block_tx_hashes_using_indices()...");
    let hashes = provider.fetch_block_tx_hashes_using_indices(test_block).await?;
    println!("   ✅ Got {} transaction hashes", hashes.len());
    if let Some(first_hash) = hashes.first() {
        println!("   First hash: 0x{:x}", first_hash);
    }
    
    // 5. Test combined metadata and receipts
    println!("\n5. Testing fetch_block_tx_metadata_and_receipts()...");
    let combined = provider.fetch_block_tx_metadata_and_receipts(test_block).await?;
    println!("   ✅ Got {} paired (metadata, receipt) entries", combined.len());
    if let Some((tx, receipt)) = combined.first() {
        println!("   First pair: tx.from=0x{:x}, receipt.gas_used={}", tx.from, receipt.gas_used);
    }
    
    // Verify consistency
    println!("\n6. Verifying consistency...");
    let all_same_count = block.transactions.len() == receipts.len() 
        && receipts.len() == metadata.len() 
        && metadata.len() == hashes.len();
    
    if all_same_count {
        println!("   ✅ All methods returned the same transaction count: {}", block.transactions.len());
    } else {
        println!("   ❌ Inconsistent counts!");
        println!("      Block transactions: {}", block.transactions.len());
        println!("      Receipts: {}", receipts.len());
        println!("      Metadata: {}", metadata.len());
        println!("      Hashes: {}", hashes.len());
    }
    
    println!("\n✅ All block fetching methods tested successfully!");
    
    Ok(())
}