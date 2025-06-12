//! Debug why transaction shows as failed

use anyhow::Result;
use ethers_providers::{Provider as EthersProvider, Http as EthersHttp, Middleware};
use std::sync::Arc;

use revm_tx_simulator_lib::{
    simulate_transaction, SimCacheDB,
    conversions::{ethers_to_revm_address, ethers_to_revm_u256},
};

use revm_primitives::{
    Bytes as RevmBytes, hardfork::SpecId,
};
use revm_context::{
    TxEnv as RevmTxEnv, BlockEnv as RevmBlockEnv, CfgEnv as RevmCfgEnv,
    TransactTo as RevmTransactTo,
};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use alloy_provider::ProviderBuilder;
use alloy_eips::BlockId;
use alloy_network::Ethereum;

const TARGET_TX: &str = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = "http://127.0.0.1:8545";
    let provider = Arc::new(EthersProvider::<EthersHttp>::try_from(rpc_url)?);
    
    // Fetch transaction and receipt
    let tx_hash: ethers_core::types::H256 = TARGET_TX.parse()?;
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
    
    println!("Transaction: {}", TARGET_TX);
    println!("Receipt Status: {:?} (0x1 = success)", receipt.status);
    println!("Gas Used: {:?}", receipt.gas_used);
    
    // Get block
    let block = provider.get_block(tx.block_number.unwrap()).await?.unwrap();
    
    // Setup REVM
    let mut cfg_env = RevmCfgEnv::default();
    cfg_env.spec = SpecId::LONDON;
    cfg_env.chain_id = 1;
    
    let mut block_env = RevmBlockEnv::default();
    block_env.number = block.number.unwrap().as_u64();
    block_env.timestamp = ethers_to_revm_u256(block.timestamp);
    block_env.beneficiary = ethers_to_revm_address(block.author.unwrap_or_default());
    block_env.basefee = block.base_fee_per_gas.unwrap_or_default().as_u64();
    
    let mut tx_env = RevmTxEnv::default();
    tx_env.caller = ethers_to_revm_address(tx.from);
    tx_env.gas_limit = tx.gas.as_u64();
    tx_env.gas_price = tx.gas_price.unwrap_or_default().as_u128();
    tx_env.value = ethers_to_revm_u256(tx.value);
    tx_env.data = RevmBytes::from(tx.input.0.clone());
    tx_env.nonce = tx.nonce.as_u64();
    
    if let Some(to) = tx.to {
        tx_env.kind = RevmTransactTo::Call(ethers_to_revm_address(to));
    }
    
    // Create database
    let alloy_provider = ProviderBuilder::new()
        .on_http(rpc_url.parse()?)
        .await?;
    
    let fork_block = BlockId::from(block.number.unwrap().as_u64() - 1);
    let alloy_provider_dyn = Arc::new(alloy_provider.into_dyn());
    let alloy_db = AlloyDB::<Ethereum, Arc<dyn alloy_provider::Provider<_>>>::new(
        alloy_provider_dyn,
        fork_block
    );
    let cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db)?);
    
    // Simulate
    println!("\nRunning simulation...");
    let (sim_output, _) = simulate_transaction(
        tx_env,
        block_env,
        cfg_env,
        cache_db
    )?;
    
    println!("\nSimulation Result:");
    println!("  Result Type: {:?}", sim_output.result_type);
    println!("  Gas Used: {}", sim_output.gas_used);
    println!("  Output Data Length: {}", sim_output.output_data.len());
    println!("  Logs: {}", sim_output.logs.len());
    
    if sim_output.output_data.len() > 0 {
        println!("  Output: 0x{}", hex::encode(&sim_output.output_data));
    }
    
    Ok(())
}