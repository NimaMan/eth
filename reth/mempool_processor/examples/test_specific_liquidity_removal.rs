use mempool_processor::tx_simulator::DirectTxSimulator;
use mempool_processor::common::address::alloy_address_to_checksum;
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::H256;
use eyre::Result;
use std::str::FromStr;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    // Use a recent liquidity removal transaction we know about
    let test_tx = "0x7d8c989e2acf664a3750a0f626660e9bdb4bb579d660cc72076a77365720981a";
    
    info!("🔍 Testing simulation of specific liquidity removal: {}", test_tx);
    
    // Connect to Ethereum node
    let provider = Provider::<Http>::try_from("http://localhost:8545")?;
    
    // Fetch the transaction
    let tx_hash = H256::from_str(test_tx)?;
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| eyre::eyre!("Transaction not found"))?;
    
    info!("✅ Transaction found");
    info!("   From: {:?}", tx.from);
    info!("   To: {:?}", tx.to);
    info!("   Value: {:?}", tx.value);
    
    // Initialize Direct simulator
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct simulator initialized");
    
    // Update to latest block
    let latest_block = simulator.update_latest_block().await?;
    info!("📦 Latest block: {}", latest_block);
    
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
    });
    
    let full_tx = mempool_processor::mempool_fetcher::FullTransaction {
        tx_data: tx_json,
        hash: format!("0x{}", hex::encode(tx.hash.as_bytes())),
        detection_time: std::time::Instant::now(),
        latency_ns: 0,
    };
    
    // Simulate with call trace
    info!("\n🔬 Simulating transaction with call trace...");
    match simulator.simulate_with_call_trace(&full_tx).await {
        Ok(result) => {
            info!("✅ Simulation successful!");
            info!("📊 Total addresses affected: {}", result.detailed_changes.len());
            info!("\n===== DETAILED STATE CHANGES =====");
            
            // Sort addresses by ETH change magnitude for better visibility
            let mut sorted_changes: Vec<_> = result.detailed_changes.iter().collect();
            sorted_changes.sort_by(|a, b| {
                b.1.eth_net.abs().partial_cmp(&a.1.eth_net.abs()).unwrap()
            });
            
            for (idx, (address, changes)) in sorted_changes.iter().enumerate() {
                let address_str = alloy_address_to_checksum(**address);
                
                // Show all addresses with any changes
                if changes.eth_net.abs() > 0.0 || !changes.token_net.is_empty() {
                    info!("\n{}. Address: {}", idx + 1, address_str);
                    
                    // Check if this is the router
                    if address_str.to_lowercase() == "0x7a250d5630b4cf539739df2c5dacb4c659f2488d" {
                        info!("   ℹ️  This is the Uniswap V2 Router");
                    }
                    
                    if changes.eth_net.abs() > 0.0 {
                        info!("   ETH change: {:.8} ETH", changes.eth_net);
                    }
                    
                    if !changes.token_net.is_empty() {
                        info!("   Token changes:");
                        for (token_addr, amount) in &changes.token_net {
                            info!("     Token {} change: {:.8}", token_addr, amount);
                        }
                    }
                    
                    // Check if this looks like a pool (has both ETH and token changes)
                    if changes.eth_net.abs() > 0.0 && !changes.token_net.is_empty() {
                        info!("   🎯 POTENTIAL POOL - has both ETH and token changes!");
                    }
                }
            }
            
            info!("\n===== END DETAILED STATE CHANGES =====");
            
            // Summary statistics
            let addresses_with_eth_changes = result.detailed_changes.iter()
                .filter(|(_, c)| c.eth_net.abs() > 0.0)
                .count();
            let addresses_with_token_changes = result.detailed_changes.iter()
                .filter(|(_, c)| !c.token_net.is_empty())
                .count();
                
            info!("\n📊 Summary:");
            info!("   Total addresses affected: {}", result.detailed_changes.len());
            info!("   Addresses with ETH changes: {}", addresses_with_eth_changes);
            info!("   Addresses with token changes: {}", addresses_with_token_changes);
        }
        Err(e) => {
            info!("❌ Simulation failed: {}", e);
        }
    }
    
    Ok(())
}