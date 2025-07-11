//! Transaction fetching example - demonstrates transaction data retrieval

use qarqa_core_types::*;
use qarqa_data_access::*;
use alloy_primitives::Address;
use std::str::FromStr;

#[tokio::main]
async fn main() -> QarqaResult<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== Transaction Fetching Example ===\n");
    
    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost:5432/eth_db".to_string());
    
    println!("Connecting to database: {}", database_url);
    
    match DatabaseManager::new()
        .with_connection_string(&database_url)
        .with_pool_size(5)
        .build()
        .await 
    {
        Ok(db_manager) => {
            println!("✓ Database connection established");
            demonstrate_transaction_fetching(db_manager).await?;
        }
        Err(e) => {
            println!("✗ Database connection failed: {}", e);
            println!("This example requires a PostgreSQL database with blockchain data.");
            println!("Please set DATABASE_URL environment variable or ensure localhost:5432 is available.");
            demonstrate_mock_transaction_fetching().await?;
        }
    }
    
    Ok(())
}

async fn demonstrate_transaction_fetching(db_manager: DatabaseManager) -> QarqaResult<()> {
    println!("\n1. Real Transaction Fetching:");
    
    let tx_fetcher = TransactionFetcher::new(db_manager.clone());
    let address_fetcher = AddressFetcher::new(db_manager);
    
    // Example transaction hash (famous transaction - first Bitcoin pizza transaction equivalent in ETH)
    let example_tx_hash = "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060"
        .parse()
        .unwrap_or_else(|_| {
            // Fallback to a sample hash if parsing fails
            [0u8; 32].into()
        });
    
    println!("  Fetching transaction: {}", example_tx_hash);
    
    // Try to fetch the transaction
    match tx_fetcher.get_transaction_complete(&example_tx_hash).await {
        Ok(transaction) => {
            println!("  ✓ Transaction found:");
            display_transaction_summary(&transaction);
            
            // Demonstrate related queries
            demonstrate_related_queries(&address_fetcher, &transaction).await?;
        }
        Err(e) => {
            println!("  ✗ Transaction not found: {}", e);
            println!("  This is expected if the database doesn't contain this specific transaction.");
            
            // Try to fetch any recent transaction instead
            demonstrate_recent_transactions(&tx_fetcher).await?;
        }
    }
    
    Ok(())
}

async fn demonstrate_mock_transaction_fetching() -> QarqaResult<()> {
    println!("\n1. Mock Transaction Fetching (Database Unavailable):");
    
    // Create a sample transaction for demonstration
    let sample_transaction = create_sample_transaction();
    
    println!("  Created sample transaction:");
    display_transaction_summary(&sample_transaction);
    
    // Demonstrate transaction analysis
    analyze_transaction_patterns(&sample_transaction).await?;
    
    Ok(())
}

async fn demonstrate_related_queries(
    address_fetcher: &AddressFetcher,
    transaction: &Transaction
) -> QarqaResult<()> {
    println!("\n2. Related Transaction Queries:");
    
    // Get other transactions for the sender
    println!("  Fetching other transactions from sender: {}", 
             format_address(&transaction.from_address));
    
    match address_fetcher
        .get_transaction_count(&transaction.from_address)
        .await 
    {
        Ok(count) => {
            println!("  ✓ Sender has {} total transactions", count);
            
            if count > 1 {
                // Get a few recent transactions
                match address_fetcher
                    .get_recent_transactions(&transaction.from_address, 5)
                    .await 
                {
                    Ok(recent_txs) => {
                        println!("  Recent transactions from this address:");
                        for (i, tx_hash) in recent_txs.iter().enumerate() {
                            println!("    {}. {}", i + 1, tx_hash);
                        }
                    }
                    Err(e) => println!("  ✗ Could not fetch recent transactions: {}", e),
                }
            }
        }
        Err(e) => println!("  ✗ Could not get transaction count: {}", e),
    }
    
    // If there's a recipient, check their transactions too
    if let Some(to_address) = transaction.to_address {
        println!("  Fetching transaction count for recipient: {}", 
                 format_address(&to_address));
        
        match address_fetcher.get_transaction_count(&to_address).await {
            Ok(count) => println!("  ✓ Recipient has {} total transactions", count),
            Err(e) => println!("  ✗ Could not get recipient transaction count: {}", e),
        }
    }
    
    Ok(())
}

async fn demonstrate_recent_transactions(tx_fetcher: &TransactionFetcher) -> QarqaResult<()> {
    println!("\n  Attempting to fetch recent transactions from database:");
    
    // Try to get the latest block number first
    match tx_fetcher.get_latest_block_number().await {
        Ok(latest_block) => {
            println!("  ✓ Latest block in database: {}", latest_block);
            
            // Try to get a few transactions from recent blocks
            let start_block = latest_block.saturating_sub(10);
            
            match tx_fetcher
                .get_transactions_in_block_range(start_block, latest_block)
                .await 
            {
                Ok(transactions) => {
                    println!("  ✓ Found {} transactions in recent blocks", transactions.len());
                    
                    // Display first few transactions
                    for (i, tx) in transactions.iter().take(3).enumerate() {
                        println!("    {}. Block {}: {} ETH", 
                                i + 1, 
                                tx.block_number,
                                wei_to_eth(tx.value));
                    }
                }
                Err(e) => println!("  ✗ Could not fetch recent transactions: {}", e),
            }
        }
        Err(e) => {
            println!("  ✗ Could not get latest block number: {}", e);
            println!("  The database may be empty or not properly set up.");
        }
    }
    
    Ok(())
}

fn create_sample_transaction() -> Transaction {
    Transaction {
        hash: [1u8; 32].into(),
        from_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
            .parse()
            .unwrap(),
        to_address: Some("0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap()),
        value: eth_to_wei(1.5), // 1.5 ETH
        gas_price: 20_000_000_000, // 20 gwei
        gas_limit: 21_000,
        gas_used: 21_000,
        block_number: 18_500_000,
        transaction_index: 42,
        status: TransactionStatus::Success,
        internal_transfers: vec![
            EthMovement {
                from_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
                    .parse()
                    .unwrap(),
                to_address: "0x1234567890123456789012345678901234567890"
                    .parse()
                    .unwrap(),
                amount: eth_to_wei(0.5),
                movement_type: MovementType::Call,
            }
        ],
        token_transfers: vec![
            TokenMovement {
                token_address: "0xA0b86a33E6441b8Ec1d04A13b29c50E5C3F9F15"
                    .parse()
                    .unwrap(),
                from_address: "0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7"
                    .parse()
                    .unwrap(),
                to_address: "0x1234567890123456789012345678901234567890"
                    .parse()
                    .unwrap(),
                amount: 1000_000_000u128.into(), // 1000 USDC (6 decimals)
                token_symbol: Some("USDC".to_string()),
                token_decimals: Some(6),
            }
        ],
        timestamp: 1698765432,
    }
}

fn display_transaction_summary(transaction: &Transaction) {
    println!("    • Hash: {}", transaction.hash);
    println!("    • From: {}", format_address(&transaction.from_address));
    
    if let Some(to) = transaction.to_address {
        println!("    • To: {}", format_address(&to));
    } else {
        println!("    • To: Contract Creation");
    }
    
    println!("    • Value: {} ETH", wei_to_eth(transaction.value));
    println!("    • Gas Used: {} / {} ({:.1}%)", 
             transaction.gas_used,
             transaction.gas_limit,
             (transaction.gas_used as f64 / transaction.gas_limit as f64) * 100.0);
    println!("    • Block: #{}", transaction.block_number);
    println!("    • Status: {:?}", transaction.status);
    println!("    • Internal Transfers: {}", transaction.internal_transfers.len());
    println!("    • Token Transfers: {}", transaction.token_transfers.len());
}

async fn analyze_transaction_patterns(transaction: &Transaction) -> QarqaResult<()> {
    println!("\n2. Transaction Pattern Analysis:");
    
    // Analyze transaction characteristics
    let gas_efficiency = (transaction.gas_used as f64 / transaction.gas_limit as f64) * 100.0;
    
    println!("  Gas Efficiency: {:.1}%", gas_efficiency);
    
    if gas_efficiency < 50.0 {
        println!("    → Low gas usage, likely simple transaction");
    } else if gas_efficiency > 90.0 {
        println!("    → High gas usage, complex transaction or near failure");
    } else {
        println!("    → Normal gas usage for transaction complexity");
    }
    
    // Analyze value transfer
    let has_eth_value = transaction.value > 0;
    let has_internal_transfers = !transaction.internal_transfers.is_empty();
    let has_token_transfers = !transaction.token_transfers.is_empty();
    
    println!("  Transaction Type Analysis:");
    
    match (has_eth_value, has_internal_transfers, has_token_transfers) {
        (true, false, false) => println!("    → Simple ETH transfer"),
        (false, false, true) => println!("    → Token-only transaction"),
        (true, true, false) => println!("    → ETH transaction with internal calls"),
        (false, true, true) => println!("    → Complex DeFi transaction"),
        (true, true, true) => println!("    → Multi-asset transaction (ETH + tokens)"),
        (false, false, false) => println!("    → Contract interaction (no value transfer)"),
        _ => println!("    → Complex transaction pattern"),
    }
    
    // Analyze token transfers
    if has_token_transfers {
        println!("  Token Transfer Analysis:");
        for (i, token_transfer) in transaction.token_transfers.iter().enumerate() {
            if let Some(symbol) = &token_transfer.token_symbol {
                println!("    {}. {} {}", i + 1, token_transfer.amount, symbol);
            } else {
                println!("    {}. {} (unknown token)", i + 1, token_transfer.amount);
            }
        }
    }
    
    Ok(())
}

/// Helper function to format addresses for display
fn format_address(address: &Address) -> String {
    let addr_str = format!("{:?}", address);
    if addr_str.len() >= 10 {
        format!("{}...{}", &addr_str[0..6], &addr_str[addr_str.len()-4..])
    } else {
        addr_str
    }
}