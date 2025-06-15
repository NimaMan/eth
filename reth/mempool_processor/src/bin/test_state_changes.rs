// Test program to demonstrate state change detection for a specific transaction
use ethers::prelude::*;
use eyre::Result;
use mempool_fetcher::state_change_detector::{StateChangeDetector, analyze_transaction_state_changes};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("State Change Detection Test");
    println!("===========================\n");
    
    // Transaction details
    let tx_hash = "0xade7d20a159397b9a8ab79ee086a888e583751f8199b2ceb57d2d4ae647238cf"
        .parse::<H256>()?;
    let replicandy_token = "0x2e32f96a4FbB9cD7CDC751971c015E282414B956"
        .parse::<Address>()?;
    let replicandy_pool = "0xACE9FEee4072aD385d02C8A6c4b69c66D72F64D6"
        .parse::<Address>()?;
    
    println!("Analyzing transaction: {:?}", tx_hash);
    println!("Tracking token: {:?}", replicandy_token);
    println!("Tracking pool: {:?}", replicandy_pool);
    
    // Connect to local node
    let provider = Provider::<Http>::try_from("http://localhost:8545")?;
    
    // First, get basic transaction info
    if let Some(tx) = provider.get_transaction(tx_hash).await? {
        println!("\nTransaction Info:");
        println!("  From: {:?}", tx.from);
        println!("  To: {:?}", tx.to);
        println!("  Value: {} ETH", ethers::utils::format_ether(tx.value));
        println!("  Gas Price: {} gwei", ethers::utils::format_units(tx.gas_price.unwrap_or_default(), "gwei")?);
    }
    
    // Analyze state changes
    analyze_transaction_state_changes(
        provider.clone(),
        tx_hash,
        replicandy_token,
        replicandy_pool,
    ).await?;
    
    // Additional analysis: Check if this transaction went through our pool
    if let Some(receipt) = provider.get_transaction_receipt(tx_hash).await? {
        println!("\nTransaction Receipt Analysis:");
        println!("  Status: {}", if receipt.status == Some(U64::from(1)) { "Success" } else { "Failed" });
        println!("  Gas Used: {}", receipt.gas_used.unwrap_or_default());
        println!("  Logs: {}", receipt.logs.len());
        
        // Check for pool involvement
        let pool_involved = receipt.logs.iter().any(|log| 
            log.address == replicandy_pool
        );
        
        if pool_involved {
            println!("\n✓ REPLICANDY POOL INVOLVED IN THIS TRANSACTION!");
            
            // Count pool-related logs
            let pool_logs = receipt.logs.iter()
                .filter(|log| log.address == replicandy_pool)
                .count();
            println!("  Pool logs: {}", pool_logs);
            
            // Look for swap events (topic0 = Swap event signature)
            let swap_topic = H256::from_slice(
                &ethers::core::utils::keccak256("Swap(address,uint256,uint256,uint256,uint256,address)")
            );
            
            let swap_events = receipt.logs.iter()
                .filter(|log| log.address == replicandy_pool && 
                             log.topics.get(0) == Some(&swap_topic))
                .count();
                
            if swap_events > 0 {
                println!("  Swap events detected: {}", swap_events);
            }
        } else {
            println!("\n✗ Replicandy pool not directly involved in this transaction");
        }
    }
    
    Ok(())
}