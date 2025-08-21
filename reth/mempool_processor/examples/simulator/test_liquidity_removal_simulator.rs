/// Test the liquidity removal simulator with problematic transactions
/// 
/// This example tests transactions that fail normal simulation due to 0 balance
/// but should be detected as scams using state override.

use mempool_processor::simulator::{UnifiedSimulator, LiquidityRemovalSimulator};
use mempool_processor::common::convert::ipc_to_call_request;
use mempool_processor::token_tracking::TokenTrackingCache;
use eyre::Result;
use tracing::{info, error, Level};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🔍 Testing liquidity removal simulator");
    
    // Test transactions that had issues
    let test_txs = vec![
        // TX with 0 balance account (MEV bot)
        ("0x6e115e08f7832ba8291e507c3b44b529163ef1302cba8aa39a433ba700e2b214", "0 balance MEV bot"),
        // TX that removed 99.9% of pool but wasn't detected
        ("0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966", "99.9% drain undetected"),
        // Successfully detected scam (for comparison)
        ("0x88c91cac79fdabdef1fde0933ab8afc13c094e6fdf556375f81d3a306915dc4b", "Correctly detected scam"),
    ];
    
    // Initialize unified simulator
    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    info!("Initializing unified simulator with datadir: {}", datadir);
    let unified_simulator = Arc::new(UnifiedSimulator::new(&datadir)?);
    
    // Create liquidity removal simulator
    let mut liquidity_removal_simulator = LiquidityRemovalSimulator::new(
        unified_simulator.get_reth_simulator()
    );
    
    // Initialize token cache
    let token_cache = Arc::new(TokenTrackingCache::new(None));
    liquidity_removal_simulator.set_token_cache(token_cache);
    
    // Test each transaction
    for (tx_hash, description) in test_txs {
        info!("\n{'='*60}");
        info!("Testing: {}", description);
        info!("TX Hash: {}", tx_hash);
        info!("{'='*60}");
        
        // Fetch transaction from RPC
        match fetch_and_test_transaction(&liquidity_removal_simulator, tx_hash).await {
            Ok(_) => info!("✅ Test completed successfully"),
            Err(e) => error!("❌ Test failed: {}", e),
        }
    }
    
    Ok(())
}

async fn fetch_and_test_transaction(
    simulator: &LiquidityRemovalSimulator,
    tx_hash: &str,
) -> Result<()> {
    use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
    
    // Connect to local Ethereum node
    let rpc_url = std::env::var("ETH_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    
    let rpc_client = HttpClientBuilder::default()
        .build(&rpc_url)?;
    
    info!("Fetching transaction {} from RPC", tx_hash);
    
    // Get transaction data
    let tx_data: serde_json::Value = rpc_client.request(
        "eth_getTransactionByHash",
        rpc_params![tx_hash]
    ).await?;
    
    if tx_data.is_null() {
        return Err(eyre::eyre!("Transaction not found"));
    }
    
    // Get block number
    let block_number = if let Some(block_num_str) = tx_data["blockNumber"].as_str() {
        let block_num_hex = block_num_str.trim_start_matches("0x");
        Some(u64::from_str_radix(block_num_hex, 16)? - 1) // Simulate at parent block
    } else {
        None // Use latest block
    };
    
    info!("Transaction found, block: {:?}", block_number);
    
    // Convert to call request
    let call_request = ipc_to_call_request(&tx_data)?;
    
    info!("From: {:?}", call_request.from);
    info!("To: {:?}", call_request.to);
    info!("Value: {:?}", call_request.value);
    
    // Run simulation
    info!("\n🔬 Running liquidity removal simulation...");
    let result = simulator.simulate_removal(call_request, block_number).await?;
    
    // Display results
    info!("\n📊 Simulation Results:");
    info!("  Success: {}", result.success);
    if let Some(ref reason) = result.revert_reason {
        info!("  Revert reason: {}", reason);
    }
    info!("  State changes: {} addresses affected", result.state_changes.len());
    
    if let Some(pool_addr) = result.pool_address {
        info!("\n💧 Pool Analysis:");
        info!("  Pool address: {:?}", pool_addr);
        info!("  ETH removed: {:.4} ETH", result.eth_removed);
        info!("  Drain percentage: {:.1}%", result.drain_percentage);
        info!("  Remaining ETH: {:.4} ETH", result.remaining_eth);
        info!("  Is scam: {}", result.is_scam);
        
        if result.is_scam {
            info!("  🚨 SCAM DETECTED - Pool drained!");
        }
    } else {
        info!("  ⚠️ No pool identified in state changes");
    }
    
    if let Some(ref debug) = result.debug_info {
        info!("\n🐛 Debug info: {}", debug);
    }
    
    // Show some state changes
    info!("\n📝 Sample state changes:");
    for (addr, changes) in result.state_changes.iter().take(3) {
        let eth_change = changes.eth_net.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        if eth_change.abs() > 0.001 {
            info!("  Address {:?}: ETH change = {:.6} ETH", addr, eth_change);
        }
    }
    
    Ok(())
}