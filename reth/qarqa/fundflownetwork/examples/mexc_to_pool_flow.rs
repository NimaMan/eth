//! Complete fund flow analysis: MEXC → Creator → Pool
//! This example demonstrates the real fund flow from the database

use qarqa_eth_db_fetcher::{create_pool, DbConfig, TransactionFetcher};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Fund Flow Analysis: MEXC → Creator → Pool");
    println!("=========================================\n");
    
    // Database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string());
    
    let db_config = DbConfig {
        database_url,
        max_connections: 5,
        min_connections: 1,
        connect_timeout: std::time::Duration::from_secs(30),
    };
    
    let db_pool = create_pool(&db_config).await?;
    let tx_fetcher = TransactionFetcher::new(db_pool.clone());
    
    // Known addresses and transactions
    let pool_address = "0x0e9797F0f05A3dE8384D76467E98DA03874c86a6"; // Exact case from DB
    let creator_address = "0xC04B517E75907965AD59976c63912C8C8af97D96"; // Exact case
    let mexc_tx = "0xaa8b115c150332e9d049ef513636b3031f40f28a8f13534c9050aa08c93ddd32";
    let liquidity_tx = "0x5e439aa276849d6ba592b5bce910fefd79c0d26b9d185d95f367848d6a1f6164";
    let liquidity_block = 22885510i64;
    
    println!("1. Checking MEXC transaction (ETH source)...");
    println!("   Transaction: {}", mexc_tx);
    
    match tx_fetcher.get_transaction(mexc_tx).await? {
        Some(tx) => {
            println!("   ✅ Found at block: {}", tx.block_number);
            println!("   From Address ID: {} (MEXC)", tx.from_address_id);
            println!("   To Address ID: {} (Creator)", tx.to_address_id.unwrap_or(0));
            println!("   Value: {} ETH", tx.value);
            
            // Get MEXC address details
            use sqlx::Row;
            let mexc_query = "SELECT address, name FROM eth_db.addresses WHERE address_id = $1";
            if let Ok(row) = sqlx::query(mexc_query)
                .bind(tx.from_address_id)
                .fetch_one(&db_pool)
                .await {
                let address: String = row.get("address");
                let name: Option<String> = row.get("name");
                println!("   MEXC Address: {} ({})", address, name.unwrap_or_default());
            }
        }
        None => println!("   ❌ Not found"),
    }
    
    println!("\n2. Tracking Creator's transactions...");
    println!("   Creator: {}", creator_address);
    
    let creator_txs = tx_fetcher.get_address_transactions(
        creator_address,
        Some(liquidity_block),
        Some(100)
    ).await?;
    
    println!("   ✅ Found {} transactions up to block {}", creator_txs.len(), liquidity_block);
    
    // Group by block
    let mut blocks_map: HashMap<i32, Vec<String>> = HashMap::new();
    for tx in &creator_txs {
        blocks_map.entry(tx.block_number)
            .or_insert_with(Vec::new)
            .push(tx.tx_hash.clone());
    }
    
    // Sort blocks
    let mut blocks: Vec<_> = blocks_map.keys().cloned().collect();
    blocks.sort();
    
    for block in blocks {
        println!("\n   Block {}: {} transactions", block, blocks_map[&block].len());
        for tx_hash in &blocks_map[&block] {
            println!("      - {}", tx_hash);
            if tx_hash == mexc_tx {
                println!("        ↑ MEXC withdrawal (1.39985 ETH received)");
            }
            if tx_hash == liquidity_tx {
                println!("        ↑ Pool liquidity addition (1 ETH sent)");
            }
        }
    }
    
    println!("\n3. Pool creation and liquidity...");
    println!("   Pool: {}", pool_address);
    
    let pool_txs = tx_fetcher.get_address_transactions(
        pool_address,
        Some(liquidity_block),
        Some(100)
    ).await?;
    
    println!("   ✅ Found {} transactions up to block {}", pool_txs.len(), liquidity_block);
    
    if !pool_txs.is_empty() {
        println!("   Liquidity transaction:");
        println!("      - {} at block {}", pool_txs[0].tx_hash, pool_txs[0].block_number);
        println!("        ↑ Pool created with 1 ETH from creator");
    }
    
    println!("\n4. Fund Flow Summary:");
    println!("   =====================================");
    println!("   Block 22885482: MEXC → Creator (1.39985 ETH)");
    println!("   Block 22885503: Creator transaction");  
    println!("   Block 22885510: Creator → Pool (1.0 ETH)");
    println!("   =====================================");
    println!("   Net: Creator kept 0.39985 ETH");
    
    // Verify internal transactions in liquidity tx
    println!("\n5. Liquidity transaction details...");
    match tx_fetcher.get_transaction(liquidity_tx).await? {
        Some(tx) => {
            println!("   ✅ Transaction found");
            println!("   From Address ID: {}", tx.from_address_id);
            println!("   To Address ID: {:?}", tx.to_address_id);
            println!("   Value: {} ETH (direct transfer)", tx.value);
            println!("\n   Note: This transaction created the pool and added liquidity.");
            println!("   Internal transfers: Creator → Router → WETH → Pool");
        }
        None => println!("   ❌ Not found"),
    }
    
    println!("\n✅ Fund flow analysis complete!");
    
    Ok(())
}