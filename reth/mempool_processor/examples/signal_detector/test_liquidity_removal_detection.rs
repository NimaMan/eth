/// Test the complete liquidity removal detection pipeline
/// 
/// This example demonstrates the full flow from transaction receipt
/// through simulation to signal generation for liquidity removal transactions.
/// 
/// Tests the integration of:
/// - Function detection (identifying removeLiquidity calls)
/// - Liquidity removal simulation (with state override for 0-balance accounts)
/// - Signal generation (detecting scams based on drain percentage)

use mempool_processor::function_detector::FunctionDetector;
use mempool_processor::tx_router::{CreatorTransactionRouter, TransactionCategory};
use mempool_processor::simulator::{SimulationManager, UnifiedSimulator, SimulationRequest, SimulationType};
use mempool_processor::signal_detector::{SignalManager, SignalManagerConfig};
use mempool_processor::token_tracking::TokenTrackingCache;
use mempool_processor::mempool_fetcher::MempoolTransaction;
use eyre::Result;
use tracing::{info, error, Level};
use std::sync::Arc;
use tokio::sync::Mutex;
use ethers::types::H256;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🔍 Testing Liquidity Removal Detection Pipeline");
    
    // Test transactions
    let test_cases = vec![
        ("0x6e115e08f7832ba8291e507c3b44b529163ef1302cba8aa39a433ba700e2b214", "MEV bot with 0 balance"),
        ("0xb20e91c60b35647725b1878b60e2ccf6543fc17983983227656cf98bebb22966", "99.9% drain (should be detected)"),
        ("0x88c91cac79fdabdef1fde0933ab8afc13c094e6fdf556375f81d3a306915dc4b", "Known scam transaction"),
    ];
    
    // Initialize components
    let datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    info!("Initializing components...");
    
    // 1. Function detector
    let function_detector = Arc::new(FunctionDetector::new());
    
    // 2. Creator transaction router
    let creator_router = Arc::new(CreatorTransactionRouter::new());
    
    // 3. Token tracking cache
    let token_cache = Arc::new(TokenTrackingCache::new(None));
    
    // 4. Unified simulator
    let unified_simulator = Arc::new(UnifiedSimulator::new(&datadir)?);
    
    // 5. Simulation manager
    let mut simulation_manager = SimulationManager::new(
        unified_simulator,
        token_cache.clone(),
        SignalManagerConfig::default(),
    );
    
    // 6. Signal manager (accessed through simulation manager)
    // The signal manager is internally created and managed
    
    info!("✅ All components initialized");
    
    // Test each transaction
    for (tx_hash, description) in test_cases {
        info!("\n{'='*60}");
        info!("Testing: {}", description);
        info!("TX Hash: {}", tx_hash);
        info!("{'='*60}");
        
        // Fetch and process the transaction
        match process_transaction(
            tx_hash,
            &function_detector,
            &creator_router,
            &mut simulation_manager,
        ).await {
            Ok(signals) => {
                info!("✅ Processing complete. Signals generated: {}", signals.len());
                for signal in signals {
                    info!("  Signal: {:?}", signal);
                }
            }
            Err(e) => {
                error!("❌ Processing failed: {}", e);
            }
        }
    }
    
    Ok(())
}

async fn process_transaction(
    tx_hash: &str,
    function_detector: &Arc<FunctionDetector>,
    creator_router: &Arc<CreatorTransactionRouter>,
    simulation_manager: &mut SimulationManager,
) -> Result<Vec<String>> {
    use jsonrpsee::{core::client::ClientT, http_client::HttpClientBuilder, rpc_params};
    
    // 1. Fetch transaction from RPC
    let rpc_url = std::env::var("ETH_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    
    let rpc_client = HttpClientBuilder::default()
        .build(&rpc_url)?;
    
    info!("📡 Fetching transaction from RPC...");
    let tx_data: serde_json::Value = rpc_client.request(
        "eth_getTransactionByHash",
        rpc_params![tx_hash]
    ).await?;
    
    if tx_data.is_null() {
        return Err(eyre::eyre!("Transaction not found"));
    }
    
    // 2. Detect function signature
    let input = tx_data["input"].as_str().unwrap_or("0x");
    let input_bytes = hex::decode(input.trim_start_matches("0x"))?;
    
    info!("🔎 Detecting function signature...");
    let function_type = function_detector.detect_function(&input_bytes);
    info!("  Function type: {:?}", function_type);
    
    // 3. Check if it's a creator transaction
    let from_address = tx_data["from"].as_str().unwrap_or("0x0");
    let from_bytes = hex::decode(from_address.trim_start_matches("0x"))?;
    
    let to_address = tx_data["to"].as_str().unwrap_or("0x0");
    
    // Create a mock mempool transaction for routing
    let mempool_tx = MempoolTransaction {
        hash: tx_hash.to_string(),
        from: from_bytes.clone(),
        to: to_address.to_string(),
        data: tx_data.clone(),
        detection_ns: 0,
    };
    
    info!("🔀 Routing transaction...");
    let category = creator_router.categorize_transaction(&mempool_tx, function_type).await;
    info!("  Category: {:?}", category);
    
    // 4. Create simulation request
    let simulation_request = SimulationRequest {
        tx: mempool_tx,
        category,
        priority: mempool_processor::tx_router::SimulationPriority::High,
        simulation_type: SimulationType::TransactionWithBuySell,
        tx_hash: H256::from_slice(&hex::decode(tx_hash.trim_start_matches("0x"))?),
    };
    
    // 5. Run simulation (this will use the liquidity removal simulator if appropriate)
    info!("🧪 Running simulation...");
    let signals = simulation_manager.process_request(simulation_request).await;
    
    // Convert signals to strings for display
    let signal_strings: Vec<String> = signals.iter()
        .map(|s| format!("{:?}", s))
        .collect();
    
    Ok(signal_strings)
}