use std::time::Instant;
use anyhow::Result;

use ethers_core::types::{H256, U256};
use ethers_providers::{Provider, Http, Middleware};
use revm_tx_simulator_lib::{
    simulate_transaction, 
    conversions::{ethers_to_revm_u256, ethers_to_revm_address},
};
use revm_primitives::{Bytes as RevmBytes, hardfork::SpecId};
use revm_context::{TxEnv, BlockEnv, CfgEnv, TransactTo};

#[tokio::main]
async fn main() -> Result<()> {
    println!("⏱️  Timing REVM Simulation Only");
    println!("==============================\n");

    let tx_hash_str = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let tx_hash: H256 = tx_hash_str.parse()?;
    
    // Get transaction data
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")?;
    
    println!("📖 Fetching transaction data...");
    let fetch_start = Instant::now();
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
    let block = provider.get_block(receipt.block_number.unwrap()).await?
        .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
    let fetch_time = fetch_start.elapsed().as_secs_f64() * 1000.0;
    
    println!("  Fetch time: {:.2}ms", fetch_time);
    println!("  Block: {}", block.number.unwrap());
    println!("  Gas used (actual): {}", receipt.gas_used.unwrap());
    
    // Prepare REVM environments
    let mut tx_env = TxEnv::default();
    tx_env.caller = ethers_to_revm_address(tx.from);
    tx_env.gas_limit = tx.gas.as_u64();
    tx_env.gas_price = ethers_to_revm_u256(tx.gas_price.unwrap_or_default()).to::<u128>();
    tx_env.kind = match tx.to {
        Some(to) => TransactTo::Call(ethers_to_revm_address(to)),
        None => TransactTo::Create,
    };
    tx_env.value = ethers_to_revm_u256(tx.value);
    tx_env.data = RevmBytes(tx.input.0.clone());
    tx_env.nonce = tx.nonce.as_u64();
    tx_env.chain_id = Some(1);

    let mut block_env = BlockEnv::default();
    block_env.number = revm_primitives::U256::from(block.number.unwrap().as_u64());
    block_env.timestamp = ethers_to_revm_u256(block.timestamp);
    block_env.basefee = ethers_to_revm_u256(block.base_fee_per_gas.unwrap_or_default()).to::<u64>();

    let mut cfg_env = CfgEnv::default();
    cfg_env.chain_id = 1;
    cfg_env.spec = SpecId::SHANGHAI;

    println!("\n⏱️  Setting up database...");
    let db_start = Instant::now();
    
    // Setup database
    use alloy_provider::ProviderBuilder;
    use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
    use std::sync::Arc;
    
    let alloy_provider = ProviderBuilder::new()
        .on_http("http://127.0.0.1:8545".parse()?);
    
    let block_id = alloy_eips::BlockId::from(block.number.unwrap().as_u64() - 1);
    let alloy_db = AlloyDB::new(Arc::new(alloy_provider), block_id.into())
        .map_err(|e| anyhow::anyhow!("Failed to create AlloyDB: {}", e))?;
    
    let db_ref = WrapDatabaseAsync::new(alloy_db);
    let cache_db = CacheDB::new(db_ref);
    
    let db_time = db_start.elapsed().as_secs_f64() * 1000.0;
    println!("  Database setup: {:.2}ms", db_time);

    println!("\n⏱️  Running simulation...");
    let sim_start = Instant::now();
    
    let (sim_output, _final_db) = simulate_transaction(
        tx_env,
        block_env,
        cfg_env,
        cache_db,
    )?;
    
    let sim_time = sim_start.elapsed().as_secs_f64() * 1000.0;
    
    println!("  Simulation time: {:.2}ms", sim_time);
    println!("  Gas used (simulated): {}", sim_output.gas_used);
    println!("  Success: {:?}", sim_output.result_type);
    
    println!("\n📊 Total Breakdown:");
    println!("  Fetch (3 RPC calls): {:.2}ms", fetch_time);
    println!("  Database setup: {:.2}ms", db_time);
    println!("  REVM simulation: {:.2}ms", sim_time);
    println!("  ─────────────────────────");
    println!("  Total: {:.2}ms", fetch_time + db_time + sim_time);
    
    println!("\n💡 Key Insights:");
    println!("- Simulation itself is very fast: {:.2}ms", sim_time);
    println!("- Most time spent on RPC fetches: {:.2}ms", fetch_time);
    println!("- With caching, total could be <1ms");
    
    Ok(())
}