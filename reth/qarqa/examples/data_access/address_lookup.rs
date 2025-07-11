//! Address lookup example - demonstrates O(1) address transaction lookups

use qarqa_core_types::*;
use qarqa_core_types::utils::*;
use qarqa_data_access::{DatabaseManager, AddressDataFetcher};
use alloy_primitives::Address;
use std::str::FromStr;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Address Lookup Example ===\n");
    
    if let Err(e) = run_example().await {
        eprintln!("Example failed: {}", e);
        std::process::exit(1);
    }
}

async fn run_example() -> QarqaResult<()> {
    info!("Starting address lookup example");
    
    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
    
    println!("Connecting to database...");
    let db = match DatabaseManager::new(&database_url).await {
        Ok(db) => {
            println!("✓ Database connection successful");
            db
        }
        Err(e) => {
            println!("✗ Database connection failed: {}", e);
            println!("Note: This example requires a PostgreSQL database with blockchain data");
            return demonstrate_concepts_without_db().await;
        }
    };
    
    // Create address data fetcher
    let fetcher = AddressDataFetcher::new(db.pool_cloned());
    
    // Test addresses to lookup
    let test_addresses = vec![
        // Some common Ethereum addresses (these might not exist in your database)
        ("Vitalik's Address", "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
        ("Uniswap V3 Router", "0xE592427A0AEce92De3Edee1F18E0157C05861564"),
        ("Random Address", "0x1111111111111111111111111111111111111111"),
        ("Another Random", "0x2222222222222222222222222222222222222222"),
    ];
    
    for (name, addr_str) in test_addresses {
        println!("\n{}. Testing {}:", name.chars().nth(0).unwrap(), name);
        
        let address = match Address::from_str(addr_str) {
            Ok(addr) => addr,
            Err(e) => {
                println!("   ✗ Invalid address format: {}", e);
                continue;
            }
        };
        
        println!("   Address: {}", format_address(address));
        
        // Test transaction count lookup
        match fetcher.get_transaction_count(address, None, None).await {
            Ok(count) => {
                println!("   ✓ Transaction count: {}", count);
                
                if count > 0 {
                    // Get some transaction hashes
                    test_transaction_lookups(&fetcher, address, count).await;
                } else {
                    println!("   ⚠ No transactions found for this address");
                }
            }
            Err(e) => {
                warn!("Transaction count lookup failed for {}: {}", name, e);
                println!("   ✗ Transaction count lookup failed: {}", e);
                
                if e.to_string().contains("participants") {
                    println!("   → This likely means the 'participants' table doesn't exist");
                    println!("   → The database needs to be populated with blockchain data first");
                }
            }
        }
    }
    
    // Demonstrate lookup performance concepts
    demonstrate_lookup_performance().await;
    
    println!("\n=== Address lookup example completed! ===");
    
    Ok(())
}

async fn test_transaction_lookups(fetcher: &AddressDataFetcher, address: Address, total_count: u64) {
    println!("   Testing transaction hash lookups:");
    
    // Get first 5 transactions
    let limit = std::cmp::min(5, total_count);
    match fetcher.get_address_transactions(address, None, None, Some(limit)).await {
        Ok(tx_hashes) => {
            println!("   ✓ Retrieved {} transaction hashes:", tx_hashes.len());
            for (i, hash) in tx_hashes.iter().enumerate() {
                println!("     {}. {}", i + 1, format_hash(hash));
            }
            
            if total_count > 5 {
                println!("     ... and {} more transactions", total_count - 5);
            }
        }
        Err(e) => {
            println!("   ✗ Transaction hash lookup failed: {}", e);
        }
    }
    
    // Test block range filtering
    println!("   Testing block range filtering:");
    match fetcher.get_transaction_count(address, Some(1), Some(1000000)).await {
        Ok(count) => {
            println!("   ✓ Transactions in blocks 1-1,000,000: {}", count);
        }
        Err(e) => {
            println!("   ✗ Block range filtering failed: {}", e);
        }
    }
}

async fn demonstrate_lookup_performance() {
    println!("\nAddress Lookup Performance Concepts:");
    println!("  • Traditional approach: Scan entire transaction table");
    println!("    - Time complexity: O(n) where n = total transactions");
    println!("    - For 100M transactions: ~seconds per lookup");
    println!("  ");
    println!("  • Our approach: Use 'participants' table index");
    println!("    - Time complexity: O(1) with proper indexing");
    println!("    - For 100M transactions: ~milliseconds per lookup");
    println!("  ");
    println!("  • Index structure:");
    println!("    - PRIMARY KEY: (address, transaction_hash, block_number)");
    println!("    - Enables instant lookups by address");
    println!("    - Supports efficient block range filtering");
    println!("    - Memory efficient with PostgreSQL B-tree indexes");
    println!("  ");
    println!("  • Query optimization:");
    println!("    - Use LIMIT for pagination");
    println!("    - Filter by block range when possible");
    println!("    - Leverage covering indexes for count queries");
}

async fn demonstrate_concepts_without_db() -> QarqaResult<()> {
    println!("Demonstrating concepts without database connection:");
    println!();
    
    // Show what the participant table structure would look like
    demonstrate_participant_table_structure();
    
    // Show example queries
    demonstrate_example_queries();
    
    // Show performance characteristics
    demonstrate_lookup_performance().await;
    
    println!("\n=== Address lookup concepts completed! ===");
    
    Ok(())
}

fn demonstrate_participant_table_structure() {
    println!("Participants Table Structure:");
    println!("  CREATE TABLE participants (");
    println!("    address          BYTEA NOT NULL,");
    println!("    transaction_hash BYTEA NOT NULL,");
    println!("    block_number     BIGINT NOT NULL,");
    println!("    direction        TEXT NOT NULL,  -- 'in', 'out', 'both'");
    println!("    value_change     BIGINT,");
    println!("    token_transfers  JSONB,");
    println!("    PRIMARY KEY (address, transaction_hash, block_number)");
    println!("  );");
    println!();
    println!("  -- Additional indexes for performance");
    println!("  CREATE INDEX idx_participants_block ON participants(block_number);");
    println!("  CREATE INDEX idx_participants_tx ON participants(transaction_hash);");
    println!();
}

fn demonstrate_example_queries() {
    println!("Example Queries:");
    println!();
    
    println!("1. Get transaction count for address:");
    println!("   SELECT COUNT(*) FROM participants");
    println!("   WHERE address = $1;");
    println!();
    
    println!("2. Get transactions for address with block range:");
    println!("   SELECT transaction_hash FROM participants");
    println!("   WHERE address = $1");
    println!("     AND block_number BETWEEN $2 AND $3");
    println!("   ORDER BY block_number DESC");
    println!("   LIMIT $4;");
    println!();
    
    println!("3. Get transaction count in block range:");
    println!("   SELECT COUNT(*) FROM participants");
    println!("   WHERE address = $1");
    println!("     AND block_number BETWEEN $2 AND $3;");
    println!();
}

fn format_hash(hash: &TransactionHash) -> String {
    format!("0x{}...{}", 
            &hash.to_string()[2..8], 
            &hash.to_string()[hash.to_string().len()-6..])
}