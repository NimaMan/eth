/// Test the pool viability analyzer with a real token
/// 
/// This example tests the erc20_token_trading_viability module
/// by analyzing a token pool for trading viability.

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::Address;
use tx_processor::{TxProcessor, chain_query::ChainQuery};
use tx_processor::erc20_token_trading_viability::{
    check_can_buy_sell_pool,
    PoolViabilityConfig,
    PoolType,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment - skip dotenv/env_logger if not available
    // dotenv::dotenv().ok();
    // env_logger::init();
    
    println!("Testing ERC20 Token Trading Viability Analyzer");
    println!("==============================================");
    
    // Get reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    // Create tx processor (contains shared chain_query)
    let tx_processor = Arc::new(TxProcessor::new(&reth_datadir)?);
    
    // Use shared chain_query from tx_processor (no separate DB connection)
    let chain_query = tx_processor.chain_query.clone();
    let simulator = chain_query.get_simulator();
    
    // Example token and pool addresses (you can replace with actual addresses)
    let token_address = Address::from([
        0x88, 0x5c, 0xf6, 0x5E, 0x15, 0x11, 0xD5, 0x0B,
        0xb4, 0x9e, 0x48, 0x88, 0x39, 0x52, 0x5f, 0xDE,
        0x44, 0xcD, 0xE3, 0x6b
    ]); // Example token
    
    let pool_address = Address::from([
        0x0C, 0xa5, 0xf8, 0xf9, 0x6C, 0x3a, 0x2C, 0x84,
        0x19, 0x0a, 0x41, 0x5b, 0x65, 0x8A, 0xaF, 0xc8,
        0x50, 0x1A, 0xae, 0xFF
    ]); // Example Uniswap V2 pool
    
    // Create configuration
    let config = PoolViabilityConfig::new(
        token_address,
        pool_address,
        PoolType::UniswapV2,
    )
    .with_test_amount(alloy_primitives::U256::from(10_000_000_000_000_000u64)); // 0.01 ETH
    
    println!("Configuration:");
    println!("  Token: {:?}", token_address);
    println!("  Pool: {:?}", pool_address);
    println!("  Type: {:?}", config.pool_type);
    println!("  Test Amount: {} wei", config.test_amount);
    println!();
    
    // Run the analysis
    println!("Running pool viability analysis...");
    let result = check_can_buy_sell_pool(
        simulator,
        tx_processor,
        config,
    ).await?;
    
    // Display results
    println!("\nAnalysis Results:");
    println!("=================");
    println!("Pool Type: {:?}", result.pool_type);
    println!("Is Tradeable: {}", result.is_tradeable);
    println!("Block Number: {}", result.block_number);
    println!();
    
    if result.is_tradeable {
        println!("Tax Analysis:");
        println!("  Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  Sell Tax: {:.2}%", result.sell_tax_percent);
        println!();
        
        println!("Trade Details:");
        println!("  ETH Spent: {} wei", result.eth_spent);
        println!("  Tokens Received: {}", result.tokens_received);
        println!("  ETH Received: {} wei", result.eth_received);
        
        let net_loss = result.eth_spent.saturating_sub(result.eth_received);
        let loss_percent = if result.eth_spent > alloy_primitives::U256::ZERO {
            (net_loss.to_string().parse::<f64>().unwrap_or(0.0) / 
             result.eth_spent.to_string().parse::<f64>().unwrap_or(1.0)) * 100.0
        } else {
            0.0
        };
        
        println!("  Net Loss: {} wei ({:.2}%)", net_loss, loss_percent);
    } else {
        println!("Trading failed!");
        if let Some(reason) = &result.failure_reason {
            println!("Failure reason: {}", reason);
        }
    }
    
    println!("\nTransaction Details:");
    println!("  Buy TX: {} ({})", 
        result.buy_transaction.hash, 
        if result.buy_transaction.status == "1" { "Success" } else { "Failed" }
    );
    println!("  Approve TX: {} ({})", 
        result.approve_transaction.hash,
        if result.approve_transaction.status == "1" { "Success" } else { "Failed" }
    );
    println!("  Sell TX: {} ({})", 
        result.sell_transaction.hash,
        if result.sell_transaction.status == "1" { "Success" } else { "Failed" }
    );
    
    if let Some(prior_tx) = &result.prior_transaction {
        println!("  Prior TX: {} ({})", 
            prior_tx.hash,
            if prior_tx.status == "1" { "Success" } else { "Failed" }
        );
    }
    
    Ok(())
}