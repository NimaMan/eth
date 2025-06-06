/*
 * Test Specific Transaction for State Change Comparison
 * 
 * This test processes a specific transaction that we can validate against Python results.
 */

use std::sync::Arc;
use tokio;
use tracing::{info, error, warn};
use ethers::providers::{Http, Provider, Middleware};
use ethers::types::{BlockId, BlockNumber, H256};
use revm_context::BlockEnv as RevmBlockEnv;
use revm_primitives::hardfork::SpecId;
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};

use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::tx_simulator::TransactionSimulator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Transaction hash from command line argument or default
    let tx_hash = std::env::args().nth(1).unwrap_or_else(|| {
        "0xb1def6eafe5a48b715b5ed210ab4d8a24e40655934aff2180643bd31e42d3c4f".to_string()
    });

    info!("🔍 RUST REVM SIMULATION TEST");
    info!("Transaction: {}", tx_hash);
    info!("============================");
    info!("");

    // Initialize components
    let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545")?);
    
    let simulator = TransactionSimulator::new(
        "http://localhost:8545",
        1, // Ethereum mainnet
        SpecId::CANCUN, // Current spec
    ).await?;

    info!("✅ TransactionSimulator initialized successfully");

    // Parse transaction hash
    let tx_hash_bytes = hex::decode(tx_hash.trim_start_matches("0x"))?;
    let tx_hash_h256: H256 = H256::from_slice(&tx_hash_bytes);

    // Get transaction details
    info!("📦 Fetching transaction details...");
    let tx = provider.get_transaction(tx_hash_h256).await?
        .ok_or_else(|| eyre::eyre!("Transaction not found"))?;

    info!("Transaction found in block: {}", tx.block_number.unwrap_or_default());
    
    // Get block for simulation context
    let block_number = tx.block_number.unwrap_or_default();
    let block = provider.get_block(BlockId::Number(block_number.into())).await?
        .ok_or_else(|| eyre::eyre!("Block not found"))?;

    let mut block_env = RevmBlockEnv::default();
    block_env.number = ethers_to_revm_u256(block.number.unwrap_or_default().as_u64().into());
    block_env.beneficiary = block.author.map_or_else(|| revm_primitives::Address::ZERO, |h160| ethers_to_revm_address(h160));
    block_env.timestamp = ethers_to_revm_u256(block.timestamp);
    block_env.gas_limit = block.gas_limit.as_u64();
    block_env.basefee = block.base_fee_per_gas.map_or(0, |bf| bf.as_u64());
    block_env.difficulty = ethers_to_revm_u256(block.difficulty);
    block_env.prevrandao = block.mix_hash.map(|h| revm_primitives::B256::from(h.0));

    info!("✅ Block environment created for block #{}", block_number);

    // Convert to TransactionView (addresses need to be raw bytes, not string bytes)
    let tx_view = TransactionView {
        hash: tx.hash.as_bytes().to_vec(),
        from: tx.from.as_bytes().to_vec(),
        to: tx.to.map(|addr| addr.as_bytes().to_vec()),
        value: tx.value,
        gas_price: tx.gas_price,
        gas_limit: Some(tx.gas),
        nonce: Some(tx.nonce),
        input_data: Some(tx.input.to_vec()),
    };

    info!("🧪 Running REVM simulation...");
    info!("From: {:#x}", tx.from);
    info!("To: {}", tx.to.map(|addr| format!("{:#x}", addr)).unwrap_or("Contract Creation".to_string()));
    info!("Value: {} ETH", tx.value.as_u64() as f64 / 1e18);
    info!("");

    // Run simulation
    match simulator.process_transaction(&tx_view, &block_env).await {
        Ok(Some(account_changes)) => {
            info!("✅ REVM simulation successful!");
            info!("📈 Affected accounts: {}", account_changes.len());
            info!("");

            // Print detailed results for comparison with Python
            info!("📊 RUST REVM STATE CHANGES:");
            info!("============================");
            
            for (address, changes) in account_changes.iter() {
                info!("🏠 Address: {}", address);
                
                // ETH net change
                let eth_amount = changes.eth_net_change.absolute_value.to::<u64>() as f64 / 1e18;
                if eth_amount > 0.000001 { // Show even small changes
                    let sign = if changes.eth_net_change.is_negative { "-" } else { "+" };
                    info!("   💰 ETH Net Change: {}{:.6} ETH", sign, eth_amount);
                } else {
                    info!("   💰 ETH Net Change: 0.000000 ETH");
                }
                
                // Token changes
                if !changes.token_net_changes.is_empty() {
                    info!("   🔥 Token Net Changes: {}", changes.token_net_changes.len());
                    for (token_addr, token_change) in &changes.token_net_changes {
                        info!("      - {}: {}", token_addr, token_change.to_signed_string());
                    }
                } else {
                    info!("   🔥 Token Net Changes: 0");
                }
                
                // Movements
                let token_movements = changes.movements.token.len();
                let denom_in_movements = changes.movements.denom.in_list.len();
                let denom_out_movements = changes.movements.denom.out_list.len();
                
                if token_movements > 0 {
                    info!("   📊 Token Movements: {}", token_movements);
                }
                if denom_in_movements > 0 || denom_out_movements > 0 {
                    info!("   📊 Denom Movements: In={}, Out={}", denom_in_movements, denom_out_movements);
                }
                
                info!("");
            }

            // Create JSON-like output for comparison with Python
            info!("🔄 JSON-COMPATIBLE OUTPUT:");
            info!("==========================");
            for (address, changes) in account_changes.iter() {
                let eth_net = changes.eth_net_change.absolute_value.to::<u64>() as f64 / 1e18;
                let eth_net_signed = if changes.eth_net_change.is_negative { -eth_net } else { eth_net };
                
                info!("\"{}\":", address);
                info!("  token_net: 0.0,  // Rust doesn't calculate token net the same way");
                info!("  denom_net: {:.6},", eth_net_signed);
                info!("  movements: {{");
                info!("    token: {{ in: {}, out: {} }},", 
                      changes.movements.token.len(), 
                      changes.movements.token.len());
                info!("    denom: {{ in: {}, out: {} }}", 
                      changes.movements.denom.in_list.len(), 
                      changes.movements.denom.out_list.len());
                info!("  }}");
            }
        },
        Ok(None) => {
            info!("⚪ No state changes detected");
        },
        Err(e) => {
            error!("❌ Simulation failed: {}", e);
        }
    }

    info!("");
    info!("💡 Compare this output with Python results:");
    info!("python python/core/validate_state_changes.py {} --rust-format", tx_hash);

    Ok(())
}