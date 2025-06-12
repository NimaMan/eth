//! Simplified signed transaction simulator
//! Takes signed transaction bytes and simulates them

use anyhow::{Result, anyhow};
use alloy_primitives::B256;
use alloy_network::Ethereum;
use alloy_provider::{Provider, DynProvider};
use alloy_eips::BlockId;
use std::sync::Arc;

use revm_primitives::{Bytes, U256, Address};
use revm_context::{BlockEnv, CfgEnv, TxEnv, TransactTo};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};

use crate::simulate_signed_tx::{
    simulation_core::{simulate_transaction, SimCacheDB, SimulationOutput},
};
use crate::spec_id_from_block_number;
use crate::conversions::{ethers_to_revm_address, ethers_to_revm_u256};

// Re-use ethers for transaction decoding
use ethers_core::types::{Transaction as EthersTransaction, H256};
use ethers_providers::{Provider as EthersProvider, Http, Middleware};

/// Simulate a signed transaction given its hash
/// This is the main entry point that handles all conversions
pub async fn simulate_signed_tx_by_hash(
    tx_hash: H256,
    rpc_url: &str,
) -> Result<SimulationOutput> {
    // Setup providers
    let ethers_provider = EthersProvider::<Http>::try_from(rpc_url)?;
    let eth_client = Arc::new(ethers_provider);
    
    let alloy_provider = alloy_provider::ProviderBuilder::new()
        .network::<Ethereum>()
        .on_http(rpc_url.parse()?);
    let alloy_provider_dyn: Arc<DynProvider<Ethereum>> = Arc::new(alloy_provider.erased());
    
    // Fetch transaction
    let tx = eth_client.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow!("Transaction not found"))?;
    
    let block_number = tx.block_number
        .ok_or_else(|| anyhow!("Transaction not yet mined"))?.as_u64();
    
    // Fetch block
    let block = eth_client.get_block(block_number).await?
        .ok_or_else(|| anyhow!("Block not found"))?;
    
    // Convert to REVM types
    let chain_id = eth_client.get_chainid().await?.as_u64();
    
    // Setup REVM environments
    let tx_env = ethers_tx_to_revm_env(&tx, chain_id)?;
    let block_env = ethers_block_to_revm_env(&block, block_number);
    let cfg_env = create_cfg_env(chain_id, block_number);
    
    // Setup database at parent block
    let fork_block_id = BlockId::Number((block_number - 1).into());
    let alloy_db = AlloyDB::<Ethereum, Arc<DynProvider<Ethereum>>>::new(
        alloy_provider_dyn.clone(),
        fork_block_id
    );
    let cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db).unwrap());
    
    // Simulate
    let (output, _final_db) = simulate_transaction(
        tx_env,
        block_env,
        cfg_env,
        cache_db
    )?;
    
    Ok(output)
}

/// Convert Ethers transaction to REVM TxEnv
fn ethers_tx_to_revm_env(tx: &EthersTransaction, chain_id: u64) -> Result<TxEnv> {
    let mut tx_env = TxEnv::default();
    
    tx_env.caller = ethers_to_revm_address(tx.from);
    tx_env.gas_limit = tx.gas.as_u64();
    tx_env.value = ethers_to_revm_u256(tx.value);
    tx_env.data = Bytes::from(tx.input.0.clone());
    tx_env.nonce = Some(tx.nonce.as_u64());
    tx_env.chain_id = Some(chain_id);
    
    // Gas price
    if let Some(max_fee) = tx.max_fee_per_gas {
        tx_env.gas_price = U256::from(max_fee.as_u128());
        tx_env.gas_priority_fee = tx.max_priority_fee_per_gas.map(|p| U256::from(p.as_u128()));
    } else {
        tx_env.gas_price = U256::from(tx.gas_price.unwrap_or_default().as_u128());
    }
    
    // Transaction type
    tx_env.kind = if let Some(to) = tx.to {
        TransactTo::Call(ethers_to_revm_address(to))
    } else {
        TransactTo::Create
    };
    
    Ok(tx_env)
}

/// Convert Ethers block to REVM BlockEnv
fn ethers_block_to_revm_env(block: &ethers_core::types::Block<H256>, block_number: u64) -> BlockEnv {
    BlockEnv {
        number: U256::from(block_number),
        beneficiary: ethers_to_revm_address(block.author.unwrap_or_default()),
        timestamp: U256::from(block.timestamp.as_u64()),
        difficulty: U256::from_limbs(block.difficulty.0),
        prevrandao: Some(block.mix_hash.unwrap_or_default().0.into()),
        basefee: block.base_fee_per_gas.unwrap_or_default().as_u64(),
        gas_limit: block.gas_limit.as_u64(),
        blob_excess_gas_and_price: None,
    }
}

/// Create config environment with proper hardfork
fn create_cfg_env(chain_id: u64, block_number: u64) -> CfgEnv {
    let mut cfg_env = CfgEnv::default();
    cfg_env.chain_id = chain_id;
    cfg_env.spec = spec_id_from_block_number(block_number);
    cfg_env
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simulate_real_tx() {
        let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
            .parse::<H256>().unwrap();
        
        let result = simulate_signed_tx_by_hash(
            tx_hash,
            "http://127.0.0.1:8545"
        ).await;
        
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.gas_used, 315099);
    }
}