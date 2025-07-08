/// Test that our call tracer captures ALL state changes including from internal calls
/// 
/// This example simulates a known liquidity removal to verify we see all affected addresses

use mempool_processor::tx_simulator::DirectTxSimulator;
use mempool_processor::common::address::alloy_address_to_checksum;
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::H256;
use eyre::Result;
use std::str::FromStr;
use tracing::{info, error, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .init();
    
    // Recent liquidity removal that we know affects specific pools
    let test_tx = "0x2438aa70af416c5d2e0fb82145142804a1587cf32a8b64c71793128deea8db9c";
    let expected_pool = "0x6ce3a16d1dd783addce2a140c5a9e6b92b84a1a3"; // From our analysis
    
    info!("Testing call trace completeness with transaction: {}", test_tx);
    info!("Expected to see state changes for pool: {}", expected_pool);
    
    // Connect to Ethereum node
    let provider = Provider::<Http>::try_from("http://localhost:8545")?;
    
    // Fetch the transaction
    let tx_hash = H256::from_str(test_tx)?;
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| eyre::eyre!("Transaction not found"))?;
    
    let block_number = tx.block_number
        .ok_or_else(|| eyre::eyre!("Transaction has no block number"))?
        .as_u64();
    
    info!("Transaction found in block {}", block_number);
    
    // Initialize Direct simulator
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    
    // Convert to the format our simulator expects
    let tx_json = serde_json::json!({
        "hash": format!("0x{}", hex::encode(tx.hash.as_bytes())),
        "from": format!("0x{}", hex::encode(tx.from.as_bytes())),
        "to": tx.to.map(|addr| format!("0x{}", hex::encode(addr.as_bytes()))),
        "value": format!("0x{:x}", tx.value),
        "gas": format!("0x{:x}", tx.gas),
        "gasPrice": tx.gas_price.map(|p| format!("0x{:x}", p)),
        "input": format!("0x{}", hex::encode(&tx.input)),
        "nonce": format!("0x{:x}", tx.nonce),
        "v": format!("0x{:x}", tx.v),
        "r": format!("0x{:x}", tx.r),
        "s": format!("0x{:x}", tx.s),
        "blockNumber": format!("0x{:x}", block_number - 1), // Simulate as if in mempool
    });
    
    let full_tx = mempool_processor::mempool_fetcher::FullTransaction {
        tx_data: tx_json,
        hash: format!("0x{}", hex::encode(tx.hash.as_bytes())),
        detection_time: std::time::Instant::now(),
        latency_ns: 0,
    };
    
    // Try to simulate at current block (will fail due to nonce)
    info!("\nAttempting simulation with call trace...");
    match simulator.simulate_with_call_trace(&full_tx).await {
        Ok(result) => {
            info!("✅ Simulation successful! Found {} address state changes", result.detailed_changes.len());
            
            let mut found_expected_pool = false;
            
            // Log all addresses with state changes
            info!("\nAll addresses with state changes:");
            for (i, (address, changes)) in result.detailed_changes.iter().enumerate() {
                let checksum_addr = alloy_address_to_checksum(*address);
                info!("\n{}. Address: {}", i + 1, checksum_addr);
                
                if checksum_addr.to_lowercase() == expected_pool.to_lowercase() {
                    found_expected_pool = true;
                    info!("   ✅ THIS IS THE EXPECTED POOL!");
                }
                
                if changes.eth_net.abs() > 0.0001 {
                    info!("   ETH change: {:.6} ETH", changes.eth_net);
                }
                
                if !changes.token_net.is_empty() {
                    info!("   Token changes: {} tokens", changes.token_net.len());
                }
            }
            
            if found_expected_pool {
                info!("\n✅ SUCCESS: Found the expected pool in state changes!");
            } else {
                error!("\n❌ FAILURE: Did not find expected pool {} in state changes!", expected_pool);
                error!("This means our call tracer is not capturing internal calls properly!");
            }
        }
        Err(e) => {
            warn!("Simulation failed (expected due to old nonce): {}", e);
            info!("\nThis is expected since we're simulating an old transaction.");
            info!("The important test is whether the call tracer would capture all addresses.");
            info!("\nTo properly test, we need to:");
            info!("1. Wait for a new liquidity removal in mempool");
            info!("2. Or modify the simulator to support historical block simulation");
        }
    }
    
    Ok(())
}