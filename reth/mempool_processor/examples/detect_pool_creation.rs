/// Example: Detect Uniswap V2 Pool Creation from Transaction
/// 
/// This example demonstrates how to:
/// 1. Take a transaction hash as input
/// 2. Detect if the transaction creates a new Uniswap V2 pool
/// 3. Extract pool details (token addresses, initial reserves)
/// 4. Analyze the state changes involved
/// 
/// Run with:
/// ```
/// cargo run --example detect_pool_creation -- <TX_HASH>
/// ```

use ethers::prelude::*;
use mempool_processor::state_change_detector::StateChangeDetector;
use mempool_processor::mempool_fetcher::pools::{PoolTracker, PoolState, PoolType};
use std::env;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber;
use eyre::Result;

// Uniswap V2 Factory address on mainnet
const UNISWAP_V2_FACTORY: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";

// PairCreated event signature
// event PairCreated(address indexed token0, address indexed token1, address pair, uint);
const PAIR_CREATED_TOPIC: &str = "0x0d3648bd0f6ba80134a33ba9275ac585d9d315f0ad8355cddefde31afa28d0e9";

#[derive(Debug)]
struct PoolCreationDetails {
    pool_address: Address,
    token0: Address,
    token1: Address,
    pair_index: U256,
    creator: Address,
    block_number: U64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("detect_pool_creation=info,mempool_processor=info")
        .init();

    // Get transaction hash from command line
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
    
    // Check if this transaction interacts with Uniswap V2 Factory
    let factory_address = UNISWAP_V2_FACTORY.parse::<Address>()?;
    let pair_created_topic = H256::from_slice(&hex::decode(PAIR_CREATED_TOPIC.trim_start_matches("0x"))?);
    
    // Look for PairCreated events
    let mut pool_creations = Vec::new();
    
    for log in &receipt.logs {
        // Check if this is a PairCreated event from the Uniswap V2 Factory
        if log.address == factory_address && 
           log.topics.len() >= 3 && 
           log.topics[0] == pair_created_topic {
            
            info!("🎉 Found PairCreated event!");
            
            // Extract event data
            // topics[1] = token0 (indexed)
            // topics[2] = token1 (indexed)
            // data = pair address (20 bytes) + pair index (uint256)
            
            let token0 = Address::from_slice(&log.topics[1].as_bytes()[12..]);
            let token1 = Address::from_slice(&log.topics[2].as_bytes()[12..]);
            
            // Parse data field
            if log.data.len() >= 32 {
                let pool_address = Address::from_slice(&log.data[12..32]);
                let pair_index = if log.data.len() >= 64 {
                    U256::from_big_endian(&log.data[32..64])
                } else {
                    U256::zero()
                };
                
                let creation = PoolCreationDetails {
                    pool_address,
                    token0,
                    token1,
                    pair_index,
                    creator: tx.from,
                    block_number: receipt.block_number.unwrap_or_default(),
                };
                
                pool_creations.push(creation);
            }
        }
    }
    
    if pool_creations.is_empty() {
        info!("❌ No pool creation detected in this transaction");
        
        // Check if transaction even went to the factory
        if tx.to == Some(factory_address) {
            info!("Transaction was sent to Uniswap V2 Factory but no PairCreated event found");
            info!("Possible reasons:");
            info!("  - Pool already exists for this token pair");
            info!("  - Transaction reverted");
            info!("  - Invalid token addresses");
        }
        
        return Ok(());
    }
    
    // Analyze each pool creation
    for (idx, creation) in pool_creations.iter().enumerate() {
        info!("\n📊 Pool Creation #{}", idx + 1);
        info!("  Pool Address: {:?}", creation.pool_address);
        info!("  Token 0: {:?}", creation.token0);
        info!("  Token 1: {:?}", creation.token1);
        info!("  Pair Index: {}", creation.pair_index);
        info!("  Created by: {:?}", creation.creator);
        
        // Get token details
        if let Ok(token0_name) = get_token_symbol(&provider, creation.token0).await {
            info!("  Token 0 Symbol: {}", token0_name);
        }
        
        if let Ok(token1_name) = get_token_symbol(&provider, creation.token1).await {
            info!("  Token 1 Symbol: {}", token1_name);
        }
        
        // Analyze state changes for this pool
        info!("\n🔄 Analyzing state changes...");
        
        // Initialize state change detector
        let detector = StateChangeDetector::new((*provider).clone())?;
        
        // Get state changes for the pool and tokens
        let watched_addresses = vec![
            creation.pool_address,
            creation.token0,
            creation.token1,
            creation.creator,
        ];
        
        match detector.extract_comprehensive_state_changes(
            tx_hash,
            &watched_addresses,
        ).await {
            Ok(state_changes) => {
                info!("State changes detected:");
                
                // Show ETH changes
                if !state_changes.eth_changes.is_empty() {
                    info!("\n💰 ETH Changes:");
                    for change in &state_changes.eth_changes {
                        info!("  Address: {:?}", change.address);
                        info!("  Change: {} ETH", 
                              format_ether_value(change.change));
                    }
                }
                
                // Show token changes
                if !state_changes.token_changes.is_empty() {
                    info!("\n🪙 Token Changes:");
                    for change in &state_changes.token_changes {
                        info!("  Address: {:?}", change.address);
                        info!("  Token: {:?}", change.token);
                        info!("  Before: {}", change.balance_before);
                        info!("  After: {}", change.balance_after);
                        info!("  Change: {}", change.change);
                    }
                }
                
                // Check for initial liquidity
                let pool_token_changes: Vec<_> = state_changes.token_changes.iter()
                    .filter(|c| c.address == creation.pool_address)
                    .collect();
                    
                if pool_token_changes.len() >= 2 {
                    info!("\n💧 Initial Liquidity Added:");
                    for change in &pool_token_changes {
                        if change.token == creation.token0 {
                            info!("  Token 0 Reserve: {}", change.balance_after);
                        } else if change.token == creation.token1 {
                            info!("  Token 1 Reserve: {}", change.balance_after);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to extract state changes: {}", e);
            }
        }
        
        // Create PoolTracker and register the new pool
        let pool_tracker = PoolTracker::new(0.01); // 0.01 ETH minimum
        
        // Get initial reserves (would need to query the pool contract)
        info!("\n📈 Registering pool in tracker...");
        
        let pool_state = PoolState {
            pool_address: creation.pool_address,
            token0_address: creation.token0,
            token1_address: creation.token1,
            reserve0: U256::zero(), // Would need to query actual reserves
            reserve1: U256::zero(), // Would need to query actual reserves
            total_supply: U256::zero(),
            block_number: creation.block_number.as_u64(),
            timestamp: 0, // Would need block timestamp
            pool_type: PoolType::UniswapV2,
        };
        
        if pool_tracker.register_pool(pool_state) {
            info!("✅ Pool registered successfully!");
        } else {
            info!("⚠️  Pool registration failed (might be below liquidity threshold)");
        }
    }
    
    info!("\n✨ Analysis complete!");
    info!("Found {} pool creation(s) in transaction", pool_creations.len());
    
    Ok(())
}

/// Helper function to get token symbol
async fn get_token_symbol(provider: &Provider<Http>, token_address: Address) -> Result<String> {
    // ERC20 symbol() function selector
    let symbol_selector = ethers::abi::Function {
        name: "symbol".to_string(),
        inputs: vec![],
        outputs: vec![ethers::abi::Param {
            name: "".to_string(),
            kind: ethers::abi::ParamType::String,
            internal_type: None,
        }],
        constant: None,
        state_mutability: ethers::abi::StateMutability::View,
    };
    
    let data = symbol_selector.encode_input(&[])?;
    
    let tx = ethers::types::transaction::eip2718::TypedTransaction::Legacy(
        ethers::types::TransactionRequest::new()
            .to(token_address)
            .data(data)
    );
    
    match provider.call(&tx, None).await {
        Ok(result) => {
            if let Ok(decoded) = symbol_selector.decode_output(&result) {
                if let Some(ethers::abi::Token::String(symbol)) = decoded.get(0) {
                    return Ok(symbol.clone());
                }
            }
            Ok("UNKNOWN".to_string())
        }
        Err(_) => Ok("UNKNOWN".to_string()),
    }
}

/// Helper function to format ETH values
fn format_ether_value(value: I256) -> String {
    let is_negative = value < I256::zero();
    let abs_value = if is_negative { -value } else { value };
    
    // Convert to ETH (divide by 10^18)
    let eth_value = abs_value.as_u128() as f64 / 1e18;
    
    if is_negative {
        format!("-{:.6}", eth_value)
    } else {
        format!("+{:.6}", eth_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pair_created_topic() {
        // Verify the PairCreated event topic is correct
        let expected = H256::from_slice(
            &ethers::core::utils::keccak256("PairCreated(address,address,address,uint256)")
        );
        let actual = H256::from_slice(&hex::decode(PAIR_CREATED_TOPIC.trim_start_matches("0x")).unwrap());
        assert_eq!(expected, actual);
    }
}