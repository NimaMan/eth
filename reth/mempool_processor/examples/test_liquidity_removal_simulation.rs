use mempool_processor::tx_simulator::DirectTxSimulator;
use mempool_processor::mempool_fetcher::FullTransactionIpcClient;
use tracing::info;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🔍 Testing liquidity removal simulation");
    
    // Initialize Direct simulator
    let simulator = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet")?;
    info!("✅ Direct simulator initialized");
    
    // Initialize IPC client to get transactions
    let ipc_client = FullTransactionIpcClient::new(Some("/tmp/reth.ipc"))?;
    info!("✅ IPC client connected");
    
    // Look for liquidity removal transactions
    info!("🔍 Searching for liquidity removal transactions...");
    
    loop {
        let txs = ipc_client.get_full_transactions(10).await?;
        
        for tx in txs {
            // Parse transaction data
            if let Some(input_str) = tx.tx_data["input"].as_str() {
                if input_str.len() >= 10 { // 0x + 8 hex chars for selector
                    let hex_str = input_str.strip_prefix("0x").unwrap_or(input_str);
                    if let Ok(data) = hex::decode(hex_str) {
                        if data.len() >= 4 {
                            let selector = hex::encode(&data[0..4]);
                            
                            // Check for liquidity removal function selectors
                            let is_liquidity_removal = match selector.as_str() {
                                "02751cec" => Some("removeLiquidityETH"),
                                "baa2abde" => Some("removeLiquidity"),
                                "af2979eb" => Some("removeLiquidityETHSupportingFeeOnTransferTokens"),
                                "5b0d5984" => Some("removeLiquidityETHWithPermit"),
                                "ded9382a" => Some("removeLiquidityETHWithPermitSupportingFeeOnTransferTokens"),
                                _ => None,
                            };
                            
                            if let Some(removal_type) = is_liquidity_removal {
                                info!("🎯 Found {} transaction: {}", removal_type, tx.hash);
                                
                                // Update to latest block
                                let _ = simulator.update_latest_block().await?;
                                
                                // Simulate with call trace
                                match simulator.simulate_with_call_trace(&tx).await {
                                    Ok(result) => {
                                        info!("✅ Simulation successful");
                                        info!("📊 State changes: {} addresses affected", result.detailed_changes.len());
                                        
                                        // Log all affected addresses
                                        for (address, changes) in &result.detailed_changes {
                                            let address_str = mempool_processor::common::address::alloy_address_to_checksum(*address);
                                            
                                            if changes.eth_net.abs() > 0.0001 || !changes.token_net.is_empty() {
                                                info!("  Address: {}", address_str);
                                                info!("    ETH change: {:.6} ETH", changes.eth_net);
                                                
                                                for (token_addr, amount) in &changes.token_net {
                                                    let token_str = format!("{:?}", token_addr);
                                                    info!("    Token {} change: {:.6}", token_str, amount);
                                                }
                                            }
                                        }
                                        
                                        // Check which addresses are monitored pools
                                        info!("\n🏊 Checking monitored pools...");
                                        // Note: We can't access pool cache here, but in the main detector it would check
                                        
                                        return Ok(());
                                    }
                                    Err(e) => {
                                        info!("❌ Simulation failed: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}