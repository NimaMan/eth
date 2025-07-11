//! Step 1: Get real transactions for pool address from database
//! 
//! Pool: 0x0e9797f0f05a3de8384d76467e98da03874c86a6
//! Liquidity added at block: 22885510
//! We want all transactions involving this pool up to that block

use qarqa_eth_db_fetcher::{create_pool, DbConfig, TransactionFetcher};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    println!("Step 1: Fetching real pool transactions from database");
    println!("====================================================");
    
    // Database connection from environment or default
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/eth_db".to_string());
    
    let db_config = DbConfig {
        database_url,
        max_connections: 5,
        min_connections: 1,
        connect_timeout: std::time::Duration::from_secs(30),
    };
    
    println!("Connecting to database...");
    let db_pool = create_pool(&db_config).await?;
    let tx_fetcher = TransactionFetcher::new(db_pool.clone());
    
    // Pool address from the transaction
    let pool_address = "0x0e9797f0f05a3de8384d76467e98da03874c86a6";
    let liquidity_block = 22885510i64;
    
    println!("Pool address: {}", pool_address);
    println!("Liquidity added at block: {}", liquidity_block);
    println!();
    
    // First, let's check what columns exist in the addresses table
    println!("1. Checking addresses table schema...");
    use sqlx::Row;
    let schema_query = r#"
        SELECT column_name, data_type 
        FROM information_schema.columns 
        WHERE table_schema = 'eth_db' AND table_name = 'addresses'
        ORDER BY ordinal_position
    "#;
    
    let rows = sqlx::query(schema_query)
        .fetch_all(&db_pool)
        .await?;
    
    println!("   Addresses table columns:");
    for row in rows {
        let col_name: String = row.get("column_name");
        let data_type: String = row.get("data_type");
        println!("   - {}: {}", col_name, data_type);
    }
    println!();
    
    // Simple address check  
    println!("2. Checking if pool address exists...");
    let simple_query = "SELECT address_id, address, name, entity_category FROM eth_db.addresses WHERE LOWER(address) = LOWER($1)";
    match sqlx::query(simple_query)
        .bind(pool_address)
        .fetch_optional(&db_pool)
        .await? {
        Some(row) => {
            let address_id: i64 = row.get("address_id");
            let address: String = row.get("address");
            println!("✅ Pool found in database:");
            println!("   Address ID: {}", address_id);
            println!("   Address: {}", address);
            println!();
        }
        None => {
            println!("❌ Pool not found in database");
            return Ok(());
        }
    }
    
    // Get all transactions for this pool using the new fetcher
    println!("3. Fetching transactions from tx_participants table...");
    let address_txs = tx_fetcher.get_address_transactions(
        pool_address,
        Some(liquidity_block),
        Some(1000)
    ).await?;
    
    println!("✅ Found {} transactions for pool (block ≤ {})", address_txs.len(), liquidity_block);
    println!();
    
    let mut relevant_txs: Vec<(String, i64)> = address_txs.iter()
        .map(|tx| (tx.tx_hash.clone(), tx.block_number as i64))
        .collect();
    
    println!();
    println!("4. Results:");
    println!("   Relevant (block ≤ {}): {}", liquidity_block, relevant_txs.len());
    println!();
    
    // Sort by block number
    relevant_txs.sort_by(|a, b| a.1.cmp(&b.1));
    
    println!("5. Relevant transactions (sorted by block):");
    for (i, (tx_hash, block_num)) in relevant_txs.iter().enumerate() {
        println!("   {}. Block {}: {}", i + 1, block_num, tx_hash);
        
        // Highlight our target transaction
        if tx_hash == "0x5e439aa276849d6ba592b5bce910fefd79c0d26b9d185d95f367848d6a1f6164" {
            println!("      ↑ THIS IS THE LIQUIDITY ADDITION TRANSACTION");
        }
    }
    
    // Also check for the MEXC transaction
    println!();
    println!("6. Checking for MEXC transaction...");
    let mexc_tx = "0xaa8b115c150332e9d049ef513636b3031f40f28a8f13534c9050aa08c93ddd32";
    match tx_fetcher.get_transaction(mexc_tx).await? {
        Some(tx_record) => {
            println!("✅ MEXC transaction found:");
            println!("   Block: {}", tx_record.block_number);
            println!("   From Address ID: {}", tx_record.from_address_id);
            println!("   To Address ID: {:?}", tx_record.to_address_id);
            println!("   Value: {}", tx_record.value);
            println!("   Status: {:?}", tx_record.status);
        }
        None => {
            println!("❌ MEXC transaction not found in database");
        }
    }
    
    println!();
    println!("✅ Step 1 complete. Found {} relevant transactions.", relevant_txs.len());
    
    Ok(())
}