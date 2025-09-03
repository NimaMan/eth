/// Common patterns for using RethQueryProvider
/// 
/// This example demonstrates:
/// 1. Resource sharing with Arc
/// 2. Error handling patterns
/// 3. Batch operations
/// 4. Using with async functions
/// 
/// Run with: cargo run --example common_patterns

use reth_chain_query::{RethQueryProvider, Result};
use alloy_primitives::{Address, U256, utils::format_ether};
use std::sync::Arc;
use std::str::FromStr;
use eyre::eyre;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Common Patterns with RethQueryProvider ===\n");
    
    // === Pattern 1: Share provider across functions ===
    println!("1. Resource Sharing with Arc");
    println!("-" .repeat(40));
    
    // Wrap provider in Arc for sharing
    let provider = Arc::new(RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?);
    
    // Share with multiple async tasks
    let provider_clone1 = provider.clone();
    let provider_clone2 = provider.clone();
    
    // Spawn concurrent tasks
    let task1 = tokio::spawn(async move {
        query_balance(provider_clone1, "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await
    });
    
    let task2 = tokio::spawn(async move {
        query_balance(provider_clone2, "A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").await
    });
    
    // Wait for both
    let (balance1, balance2) = tokio::try_join!(task1, task2)?;
    println!("Vitalik's balance: {} ETH", format_ether(balance1?));
    println!("USDC balance: {} ETH", format_ether(balance2?));
    
    println!();
    
    // === Pattern 2: Error handling ===
    println!("2. Error Handling Patterns");
    println!("-" .repeat(40));
    
    // Handle missing data gracefully
    let future_block = 99_999_999;
    match provider.get_block_header(future_block).await {
        Ok(header) => println!("Found block {}", header.number),
        Err(e) => println!("Expected error for future block: {}", e),
    }
    
    // Provide defaults for missing accounts
    let random_address = Address::from_str("0000000000000000000000000000000000000001")?;
    let account = provider.get_account(random_address, None).await
        .unwrap_or_else(|_| reth_chain_query::Account {
            nonce: 0,
            balance: U256::ZERO,
            code_hash: None,
        });
    println!("Random address balance: {} ETH", format_ether(account.balance));
    
    println!();
    
    // === Pattern 3: Batch operations ===
    println!("3. Batch Operations");
    println!("-" .repeat(40));
    
    let addresses = vec![
        Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?, // Vitalik
        Address::from_str("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?, // WETH
        Address::from_str("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")?, // USDC
    ];
    
    // Query all addresses in parallel
    let mut tasks = vec![];
    for address in addresses {
        let provider_clone = provider.clone();
        tasks.push(tokio::spawn(async move {
            provider_clone.get_account(address, None).await
        }));
    }
    
    // Collect results
    let mut total_balance = U256::ZERO;
    for task in tasks {
        let account = task.await??;
        total_balance = total_balance + account.balance;
    }
    println!("Total ETH across all addresses: {} ETH", format_ether(total_balance));
    
    println!();
    
    // === Pattern 4: Querying at specific blocks ===
    println!("4. Historical Queries");
    println!("-" .repeat(40));
    
    let address = Address::from_str("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045")?;
    let blocks = vec![
        (12_000_000, "Pre-EIP-1559"),
        (15_537_393, "The Merge"),
        (17_000_000, "Post-Shanghai"),
    ];
    
    for (block, label) in blocks {
        let account = provider.get_account(address, Some(block)).await?;
        println!("{} (block {}): {} ETH", 
            label, 
            block, 
            format_ether(account.balance)
        );
    }
    
    println!();
    
    // === Pattern 5: Combining multiple data sources ===
    println!("5. Combining Data");
    println!("-" .repeat(40));
    
    // Get block and account data together
    let latest = provider.get_latest_block()?;
    let header = provider.get_block_header(latest).await?;
    let account = provider.get_account(address, Some(latest)).await?;
    
    println!("At block {} (timestamp {}):", latest, header.timestamp);
    println!("  Address 0x{} has {} ETH", address, format_ether(account.balance));
    println!("  Block base fee: {} gwei", 
        header.base_fee_per_gas.unwrap_or(0) / 1_000_000_000
    );
    
    println!("\n✅ All patterns demonstrated successfully!");
    
    Ok(())
}

// Helper function for async tasks
async fn query_balance(provider: Arc<RethQueryProvider>, address_str: &str) -> Result<U256> {
    let address = Address::from_str(address_str)?;
    let account = provider.get_account(address, None).await?;
    Ok(account.balance)
}