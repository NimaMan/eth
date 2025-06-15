/// Example: Detect Pool State Changes from Transaction
/// 
/// This example demonstrates how to:
/// 1. Take a transaction hash as input
/// 2. Detect if the transaction interacts with any known pools
/// 3. Identify the type of interaction (swap, add liquidity, remove liquidity)
/// 4. Calculate the state changes (reserve changes, price impact)
/// 
/// Run with:
/// ```
/// cargo run --example detect_pool_state_changes -- <TX_HASH>
/// ```

use ethers::prelude::*;
use mempool_processor::state_change_detector::{StateChangeDetector, PoolInteractionType};
use std::env;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber;
use eyre::Result;
use std::collections::HashMap;

// Known DEX router addresses (for better interaction detection)
const UNISWAP_V2_ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const SUSHISWAP_ROUTER: &str = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";

// Event signatures for pool interactions
const SWAP_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822"; // Swap(address,uint256,uint256,uint256,uint256,address)
const SYNC_TOPIC: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"; // Sync(uint112,uint112)
const MINT_TOPIC: &str = "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f"; // Mint(address,uint256,uint256)
const BURN_TOPIC: &str = "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496"; // Burn(address,uint256,uint256,address)

#[derive(Debug)]
struct PoolInteractionSummary {
    pool_address: Address,
    interaction_type: String,
    token0_delta: I256,
    token1_delta: I256,
    reserves_before: Option<(U256, U256)>,
    reserves_after: Option<(U256, U256)>,
    price_before: Option<f64>,
    price_after: Option<f64>,
    price_impact: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("detect_pool_state_changes=info,mempool_processor=info")
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

    let block_number = receipt.block_number.unwrap_or_default();
    
    info!("Transaction details:");
    info!("  From: {:?}", tx.from);
    info!("  To: {:?}", tx.to);
    info!("  Value: {} ETH", ethers::utils::format_ether(tx.value));
    info!("  Block: {}", block_number);
    info!("  Status: {}", if receipt.status == Some(U64::from(1)) { "Success ✅" } else { "Failed ❌" });
    
    // Check if transaction went through a known router
    if let Some(to) = tx.to {
        if to == UNISWAP_V2_ROUTER.parse::<Address>()? {
            info!("  Router: Uniswap V2 Router");
        } else if to == SUSHISWAP_ROUTER.parse::<Address>()? {
            info!("  Router: SushiSwap Router");
        }
    }
    
    // Analyze logs for pool interactions
    let mut pool_interactions = HashMap::new();
    let mut pool_addresses = Vec::new();
    
    info!("\n📊 Analyzing transaction logs ({} total)...", receipt.logs.len());
    
    for (idx, log) in receipt.logs.iter().enumerate() {
        if log.topics.is_empty() {
            continue;
        }
        
        let topic0 = log.topics[0];
        
        // Check for pool-related events
        if topic0 == H256::from_slice(&hex::decode(SWAP_TOPIC.trim_start_matches("0x"))?) {
            info!("  Log #{}: Swap event at {:?}", idx, log.address);
            pool_addresses.push(log.address);
            pool_interactions.entry(log.address).or_insert(Vec::new()).push("Swap");
            
            // Decode swap data
            if log.data.len() >= 128 {
                let amount0_in = U256::from_big_endian(&log.data[0..32]);
                let amount1_in = U256::from_big_endian(&log.data[32..64]);
                let amount0_out = U256::from_big_endian(&log.data[64..96]);
                let amount1_out = U256::from_big_endian(&log.data[96..128]);
                
                info!("    Amount0 In: {}", amount0_in);
                info!("    Amount1 In: {}", amount1_in);
                info!("    Amount0 Out: {}", amount0_out);
                info!("    Amount1 Out: {}", amount1_out);
            }
        } else if topic0 == H256::from_slice(&hex::decode(SYNC_TOPIC.trim_start_matches("0x"))?) {
            info!("  Log #{}: Sync event at {:?}", idx, log.address);
            
            // Decode new reserves
            if log.data.len() >= 64 {
                let reserve0 = U256::from_big_endian(&log.data[0..32]);
                let reserve1 = U256::from_big_endian(&log.data[32..64]);
                info!("    New Reserve0: {}", reserve0);
                info!("    New Reserve1: {}", reserve1);
            }
        } else if topic0 == H256::from_slice(&hex::decode(MINT_TOPIC.trim_start_matches("0x"))?) {
            info!("  Log #{}: Mint (Add Liquidity) event at {:?}", idx, log.address);
            pool_addresses.push(log.address);
            pool_interactions.entry(log.address).or_insert(Vec::new()).push("AddLiquidity");
        } else if topic0 == H256::from_slice(&hex::decode(BURN_TOPIC.trim_start_matches("0x"))?) {
            info!("  Log #{}: Burn (Remove Liquidity) event at {:?}", idx, log.address);
            pool_addresses.push(log.address);
            pool_interactions.entry(log.address).or_insert(Vec::new()).push("RemoveLiquidity");
        }
    }
    
    if pool_addresses.is_empty() {
        info!("\n❌ No direct pool interactions detected in this transaction");
        info!("This transaction might:");
        info!("  - Be interacting through a router or aggregator");
        info!("  - Not involve any DEX pools");
        info!("  - Be a simple transfer or contract interaction");
        return Ok(());
    }
    
    // Use StateChangeDetector to analyze interactions
    info!("\n🔄 Analyzing state changes for {} pools...", pool_addresses.len());
    
    let detector = StateChangeDetector::new((*provider).clone())?;
    
    // Get unique pool addresses
    let unique_pools: Vec<Address> = pool_addresses.into_iter().collect::<std::collections::HashSet<_>>().into_iter().collect();
    
    // Detect pool interactions using the state change detector
    let interactions = detector.detect_pool_interactions(&receipt.logs, &unique_pools);
    
    if !interactions.is_empty() {
        info!("\n💡 Detailed Pool Interactions:");
        for (idx, interaction) in interactions.iter().enumerate() {
            info!("\n  Interaction #{}:", idx + 1);
            info!("    Pool: {:?}", interaction.pool_address);
            info!("    Type: {:?}", interaction.interaction_type);
            
            match interaction.interaction_type {
                PoolInteractionType::Swap => {
                    info!("    Token In: {:?}", interaction.token_in);
                    info!("    Amount In: {} (raw)", interaction.amount_in);
                    info!("    Token Out: {:?}", interaction.token_out);
                    info!("    Amount Out: {} (raw)", interaction.amount_out);
                    
                    // Calculate price
                    if interaction.amount_in > U256::zero() && interaction.amount_out > U256::zero() {
                        let price = interaction.amount_out.as_u128() as f64 / interaction.amount_in.as_u128() as f64;
                        info!("    Price: {:.6} out/in", price);
                    }
                }
                PoolInteractionType::AddLiquidity => {
                    info!("    Token 0: {:?} Amount: {}", interaction.token_in, interaction.amount_in);
                    info!("    Token 1: {:?} Amount: {}", interaction.token_out, interaction.amount_out);
                }
                PoolInteractionType::RemoveLiquidity => {
                    info!("    Liquidity removed");
                }
            }
        }
    }
    
    // Get comprehensive state changes
    info!("\n📈 Analyzing comprehensive state changes...");
    
    let watched_addresses = unique_pools.iter().cloned().collect::<Vec<_>>();
    
    match detector.extract_comprehensive_state_changes(tx_hash, &watched_addresses).await {
        Ok(state_changes) => {
            // Summary of changes
            let total_eth_movement = state_changes.eth_changes.iter()
                .map(|c| c.change.abs())
                .fold(I256::zero(), |acc, val| acc + val);
                
            let total_token_changes = state_changes.token_changes.len();
            
            info!("\n📊 State Change Summary:");
            info!("  Total ETH movement: {} ETH", format_ether_i256(total_eth_movement));
            info!("  Total token balance changes: {}", total_token_changes);
            
            if !state_changes.eth_changes.is_empty() {
                info!("\n  ETH Changes:");
                for change in &state_changes.eth_changes {
                    if unique_pools.contains(&change.address) {
                        info!("    Pool {:?}: {} ETH", change.address, format_ether_i256(change.change));
                    }
                }
            }
            
            if !state_changes.token_changes.is_empty() {
                info!("\n  Token Changes in Pools:");
                for change in &state_changes.token_changes {
                    if unique_pools.contains(&change.address) {
                        info!("    Pool {:?}:", change.address);
                        info!("      Token {:?}: {}", change.token, change.change);
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to extract comprehensive state changes: {}", e);
        }
    }
    
    info!("\n✨ Analysis complete!");
    
    Ok(())
}

/// Helper function to format I256 as ETH
fn format_ether_i256(value: I256) -> String {
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
    fn test_event_topics() {
        // Verify event topics are correct
        let swap_expected = H256::from_slice(
            &ethers::core::utils::keccak256("Swap(address,uint256,uint256,uint256,uint256,address)")
        );
        let swap_actual = H256::from_slice(&hex::decode(SWAP_TOPIC.trim_start_matches("0x")).unwrap());
        assert_eq!(swap_expected, swap_actual);
        
        let sync_expected = H256::from_slice(
            &ethers::core::utils::keccak256("Sync(uint112,uint112)")
        );
        let sync_actual = H256::from_slice(&hex::decode(SYNC_TOPIC.trim_start_matches("0x")).unwrap());
        assert_eq!(sync_expected, sync_actual);
    }
}