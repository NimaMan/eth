/// Example: Detect Liquidity Pool Creation from Transaction
/// 
/// This example demonstrates how to:
/// 1. Take a transaction hash as input
/// 2. Detect if the transaction creates a new liquidity pool
/// 3. Extract pool details (token addresses, initial reserves)
/// 4. Show which DEX protocol was used
/// 
/// Run with:
/// ```
/// cargo run --example detect_pool_creation -- <TX_HASH>
/// ```
///
/// Example transactions:
/// - Uniswap V2: 0x...
/// - Uniswap V3: 0x...

use ethers::prelude::*;
use ethers::utils::format_units;
use std::env;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber;
use eyre::Result;

// Known factory addresses
const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
const SUSHISWAP_FACTORY: &str = "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac";

// Event signatures
const PAIR_CREATED_TOPIC: &str = "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9"; // V2
const POOL_CREATED_TOPIC: &str = "0x783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118"; // V3

#[derive(Debug)]
struct PoolCreation {
    factory: String,
    pool_address: Address,
    token0: Address,
    token1: Address,
    dex_type: String,
    fee_tier: Option<u32>, // For V3 pools
    block_number: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("detect_pool_creation=info")
        .init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        error!("Usage: {} <TRANSACTION_HASH>", args[0]);
        error!("Example: {} 0x123...", args[0]);
        std::process::exit(1);
    }

    let tx_hash = args[1].parse::<H256>()?;
    info!("🔍 Analyzing transaction: {:?}", tx_hash);

    // Connect to Ethereum node
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545")?);
    let chain_id = provider.get_chainid().await?;
    info!("Connected to chain {}", chain_id);

    // Get transaction and receipt
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| eyre::eyre!("Transaction not found"))?;
    
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| eyre::eyre!("Transaction receipt not found"))?;

    info!("Transaction from: {:?}", tx.from);
    info!("Transaction to: {:?}", tx.to);
    info!("Block number: {:?}", receipt.block_number);
    info!("Gas used: {:?}", receipt.gas_used);
    
    // Parse event topics
    let pair_created_topic = H256::from_slice(&hex::decode(PAIR_CREATED_TOPIC.trim_start_matches("0x"))?);
    let pool_created_topic = H256::from_slice(&hex::decode(POOL_CREATED_TOPIC.trim_start_matches("0x"))?);
    
    // Look for pool creation events
    let mut pool_creations = Vec::new();
    
    for log in &receipt.logs {
        // Check Uniswap V2 PairCreated
        if log.address == UNISWAP_V2_FACTORY.parse::<Address>()? && 
           log.topics.len() >= 3 && 
           log.topics[0] == pair_created_topic {
            
            info!("🎉 Found Uniswap V2 PairCreated event!");
            
            let token0 = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let token1 = Address::from_slice(&log.topics[2].as_bytes()[12..]);
            
            // Parse pool address from data
            if log.data.len() >= 32 {
                let pool_address = Address::from_slice(&log.data[12..32]);
                
                pool_creations.push(PoolCreation {
                    factory: "Uniswap V2".to_string(),
                    pool_address,
                    token0,
                    token1,
                    dex_type: "UniswapV2".to_string(),
                    fee_tier: None,
                    block_number: receipt.block_number.unwrap().as_u64(),
                });
            }
        }
        
        // Check Uniswap V3 PoolCreated
        if log.address == UNISWAP_V3_FACTORY.parse::<Address>()? && 
           log.topics.len() >= 3 && 
           log.topics[0] == pool_created_topic {
            
            info!("🎉 Found Uniswap V3 PoolCreated event!");
            
            let token0 = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let token1 = Address::from_slice(&log.topics[2].as_bytes()[12..]);
            let fee = U256::from_big_endian(&log.topics[3].as_bytes()).as_u32();
            
            // Parse pool address from data
            if log.data.len() >= 32 {
                let pool_address = Address::from_slice(&log.data[12..32]);
                
                pool_creations.push(PoolCreation {
                    factory: "Uniswap V3".to_string(),
                    pool_address,
                    token0,
                    token1,
                    dex_type: "UniswapV3".to_string(),
                    fee_tier: Some(fee),
                    block_number: receipt.block_number.unwrap().as_u64(),
                });
            }
        }
        
        // Check SushiSwap (same as V2)
        if log.address == SUSHISWAP_FACTORY.parse::<Address>()? && 
           log.topics.len() >= 3 && 
           log.topics[0] == pair_created_topic {
            
            info!("🎉 Found SushiSwap PairCreated event!");
            
            let token0 = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let token1 = Address::from_slice(&log.topics[2].as_bytes()[12..]);
            
            if log.data.len() >= 32 {
                let pool_address = Address::from_slice(&log.data[12..32]);
                
                pool_creations.push(PoolCreation {
                    factory: "SushiSwap".to_string(),
                    pool_address,
                    token0,
                    token1,
                    dex_type: "SushiSwap".to_string(),
                    fee_tier: None,
                    block_number: receipt.block_number.unwrap().as_u64(),
                });
            }
        }
    }
    
    // Display results
    if pool_creations.is_empty() {
        warn!("❌ No pool creation events found in this transaction");
        info!("This transaction may not create a liquidity pool, or uses a different DEX");
    } else {
        info!("\n📊 Pool Creation Summary:");
        info!("========================");
        
        for (i, pool) in pool_creations.iter().enumerate() {
            info!("\nPool #{}", i + 1);
            info!("  DEX: {}", pool.factory);
            info!("  Pool Address: {:?}", pool.pool_address);
            info!("  Token 0: {:?}", pool.token0);
            info!("  Token 1: {:?}", pool.token1);
            if let Some(fee) = pool.fee_tier {
                info!("  Fee Tier: {}bps ({}%)", fee / 100, fee as f64 / 10000.0);
            }
            info!("  Block: {}", pool.block_number);
            
            // Get token information
            info!("\n  Token Details:");
            if let Ok(name0) = get_token_name(&provider, pool.token0).await {
                info!("    Token 0 Name: {}", name0);
            }
            if let Ok(symbol0) = get_token_symbol(&provider, pool.token0).await {
                info!("    Token 0 Symbol: {}", symbol0);
            }
            if let Ok(name1) = get_token_name(&provider, pool.token1).await {
                info!("    Token 1 Name: {}", name1);
            }
            if let Ok(symbol1) = get_token_symbol(&provider, pool.token1).await {
                info!("    Token 1 Symbol: {}", symbol1);
            }
            
            // Try to get initial reserves (for V2 pools)
            if pool.dex_type != "UniswapV3" {
                if let Ok((reserve0, reserve1)) = get_initial_reserves(&provider, pool.pool_address).await {
                    info!("\n  Initial Reserves:");
                    info!("    Token 0: {}", format_units(reserve0, 18)?);
                    info!("    Token 1: {}", format_units(reserve1, 18)?);
                }
            }
        }
    }
    
    Ok(())
}

// Helper functions to get token information
async fn get_token_name(provider: &Provider<Http>, token_address: Address) -> Result<String> {
    let name_sig = "0x06fdde03"; // name()
    let tx = TransactionRequest::new()
        .to(token_address)
        .data(hex::decode(name_sig)?);
    
    let result = provider.call(&tx.into(), None).await?;
    
    // Parse the result (assuming standard ERC20 encoding)
    if result.len() >= 96 {
        let length = U256::from(&result[64..96]).as_usize();
        if result.len() >= 96 + length {
            let name_bytes = &result[96..96 + length];
            return Ok(String::from_utf8_lossy(name_bytes).to_string());
        }
    }
    
    Err(eyre::eyre!("Failed to decode token name"))
}

async fn get_token_symbol(provider: &Provider<Http>, token_address: Address) -> Result<String> {
    let symbol_sig = "0x95d89b41"; // symbol()
    let tx = TransactionRequest::new()
        .to(token_address)
        .data(hex::decode(symbol_sig)?);
    
    let result = provider.call(&tx.into(), None).await?;
    
    // Parse the result
    if result.len() >= 96 {
        let length = U256::from(&result[64..96]).as_usize();
        if result.len() >= 96 + length {
            let symbol_bytes = &result[96..96 + length];
            return Ok(String::from_utf8_lossy(symbol_bytes).to_string());
        }
    }
    
    Err(eyre::eyre!("Failed to decode token symbol"))
}

async fn get_initial_reserves(provider: &Provider<Http>, pool_address: Address) -> Result<(U256, U256)> {
    let reserves_sig = "0x0902f1ac"; // getReserves()
    let tx = TransactionRequest::new()
        .to(pool_address)
        .data(hex::decode(reserves_sig)?);
    
    let result = provider.call(&tx.into(), None).await?;
    
    // Parse reserves (first 64 bytes)
    if result.len() >= 64 {
        let reserve0 = U256::from(&result[0..32]);
        let reserve1 = U256::from(&result[32..64]);
        return Ok((reserve0, reserve1));
    }
    
    Err(eyre::eyre!("Failed to get reserves"))
}