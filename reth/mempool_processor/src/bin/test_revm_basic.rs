/*
 * Basic REVM Simulation Test
 * 
 * This standalone test verifies that our REVM simulation works correctly
 * and can detect state changes in transactions.
 */

use std::sync::Arc;
use tokio;
use tracing::{info, error, warn, debug};
use ethers::providers::{Http, Provider, Middleware};
use ethers::types::{BlockId, BlockNumber};
use revm_context::BlockEnv as RevmBlockEnv;
use revm_primitives::hardfork::SpecId;
use revm_tx_simulator_lib::conversions::{ethers_to_revm_u256, ethers_to_revm_address};

use mempool_processor::mempool_processor::fetcher::{MempoolFetcher, TransactionSource};
use mempool_processor::mempool_processor::types::TransactionView;
use mempool_processor::tx_simulator::TransactionSimulator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🧪 REVM SIMULATION BASIC TEST");
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
    info!("");

    // Get current block for simulation context
    info!("📊 Getting current block information...");
    let latest_block = provider.get_block(BlockNumber::Latest).await?
        .expect("Latest block not found");
    
    let mut block_env = RevmBlockEnv::default();
    block_env.number = ethers_to_revm_u256(latest_block.number.unwrap_or_default().as_u64().into());
    block_env.beneficiary = latest_block.author.map_or_else(|| revm_primitives::Address::ZERO, |h160| ethers_to_revm_address(h160));
    block_env.timestamp = ethers_to_revm_u256(latest_block.timestamp);
    block_env.gas_limit = latest_block.gas_limit.as_u64();
    block_env.basefee = latest_block.base_fee_per_gas.map_or(0, |bf| bf.as_u64());
    block_env.difficulty = ethers_to_revm_u256(latest_block.difficulty);
    block_env.prevrandao = latest_block.mix_hash.map(|h| revm_primitives::B256::from(h.0));

    info!("✅ Block environment created: Block #{}", latest_block.number.unwrap_or_default());
    info!("");

    // Get some recent transactions to test
    info!("📦 Fetching recent mempool transactions...");
    let fetcher = MempoolFetcher::new("http://localhost:8545")?;
    let transactions = fetcher.get_transactions().await?;
    
    info!("✅ Fetched {} transactions from mempool", transactions.len());
    info!("");

    if transactions.is_empty() {
        warn!("❌ No transactions found in mempool");
        warn!("This test requires active mempool transactions");
        return Ok(());
    }

    // Test simulation on first few transactions
    info!("🧪 Testing REVM simulation on first 5 transactions...");
    info!("");

    let mut successful_simulations = 0;
    let mut failed_simulations = 0;
    let mut total_accounts_affected = 0;

    for (i, tx) in transactions.iter().take(5).enumerate() {
        let tx_hash = hex::encode(&tx.hash);
        info!("📊 Transaction {}: 0x{}", i + 1, &tx_hash[..8]);

        match simulator.process_transaction(tx, &block_env).await {
            Ok(Some(account_changes)) => {
                successful_simulations += 1;
                total_accounts_affected += account_changes.len();
                
                info!("   ✅ Simulation successful");
                info!("   📈 Affected accounts: {}", account_changes.len());
                
                // Show some details about changes
                for (address, changes) in account_changes.iter().take(3) {
                    info!("   📍 Address: {}", address);
                    
                    // Convert SignedAmount to float for display
                    let eth_amount = changes.eth_net_change.absolute_value.to::<u64>() as f64 / 1e18;
                    if eth_amount > 0.001 {
                        let sign = if changes.eth_net_change.is_negative { "-" } else { "+" };
                        info!("      💰 ETH change: {}{:.6} ETH", sign, eth_amount);
                    }
                    
                    if !changes.token_net_changes.is_empty() {
                        info!("      🔥 Token changes: {}", changes.token_net_changes.len());
                    }
                    
                    // Check movements - it's a struct with token and denom fields
                    let movement_count = changes.movements.token.len() + changes.movements.denom.in_list.len();
                    if movement_count > 0 {
                        info!("      📊 Movements: {}", movement_count);
                    }
                }
                
                if account_changes.len() > 3 {
                    info!("   ... and {} more accounts", account_changes.len() - 3);
                }
            },
            Ok(None) => {
                info!("   ⚪ No state changes detected");
            },
            Err(e) => {
                failed_simulations += 1;
                warn!("   ❌ Simulation failed: {}", e);
            }
        }
        
        info!("");
    }

    // Summary
    info!("📊 SIMULATION RESULTS SUMMARY");
    info!("===========================");
    info!("✅ Successful simulations: {}", successful_simulations);
    info!("❌ Failed simulations: {}", failed_simulations);
    info!("📈 Total accounts affected: {}", total_accounts_affected);
    info!("📊 Average accounts per transaction: {:.1}", 
          if successful_simulations > 0 { total_accounts_affected as f64 / successful_simulations as f64 } else { 0.0 });
    info!("");

    if successful_simulations > 0 {
        info!("🎉 REVM simulation is working correctly!");
        info!("The system can detect state changes in transactions.");
    } else {
        error!("❌ REVM simulation failed for all transactions");
        error!("This indicates a problem with the simulation setup.");
    }

    Ok(())
}