//! Main entry point for the simulate_signed_tx module
//! This handles all conversions from signed transactions to REVM types

use anyhow::{Result, anyhow};
use std::sync::Arc;

// Re-use existing types
use crate::simulate_signed_tx::simulation_core::{simulate_transaction as simulate_core, SimCacheDB, SimulationOutput};
use crate::conversions::{ethers_to_revm_address, ethers_to_revm_u256};
use super::spec_utils::spec_id_from_block_number;

use revm_primitives::{Bytes, U256};
use revm_context::{BlockEnv, CfgEnv, TxEnv, TransactTo};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use revm::context_interface::block::BlobExcessGasAndPrice;

use ethers_core::types::{Transaction as EthersTransaction, Block, H256};
use ethers_providers::{Provider as EthersProvider, Http, Middleware};
use alloy_eips::BlockId;
use alloy_network::Ethereum;
use alloy_provider::ProviderBuilder;

/// Main entry point: Simulate a signed transaction by its hash
/// This is what users of this module should call
pub async fn simulate_signed_tx(
    tx_hash: H256,
    rpc_url: &str,
) -> Result<SimulationOutput> {
    // Setup providers
    let ethers_provider = EthersProvider::<Http>::try_from(rpc_url)?;
    let eth_client = Arc::new(ethers_provider);
    
    // Fetch transaction
    let tx = eth_client.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow!("Transaction not found: {:?}", tx_hash))?;
    
    let block_number = tx.block_number
        .ok_or_else(|| anyhow!("Transaction not yet mined"))?.as_u64();
    
    // Fetch block
    let block = eth_client.get_block(block_number).await?
        .ok_or_else(|| anyhow!("Block {} not found", block_number))?;
    
    // Get chain ID
    let chain_id = eth_client.get_chainid().await?.as_u64();
    
    // Convert to REVM types (this is the key part that was missing!)
    let (tx_env, block_env, cfg_env) = convert_to_revm_types(&tx, &block, chain_id, block_number)?;
    
    // Setup Alloy provider for database
    use alloy_provider::DynProvider;
    let alloy_provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let alloy_provider_dyn: Arc<DynProvider<Ethereum>> = Arc::new(DynProvider::new(alloy_provider));
    
    // Setup database at parent block
    let fork_block_id = BlockId::from(block_number - 1);
    let alloy_db = AlloyDB::<Ethereum, Arc<DynProvider<Ethereum>>>::new(
        alloy_provider_dyn.clone(),
        fork_block_id
    );
    let cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db).unwrap());
    
    // Call the core simulation function
    let (output, _final_db) = simulate_core(tx_env, block_env, cfg_env, cache_db)?;
    
    Ok(output)
}

/// Alternative entry point: Simulate from signed transaction bytes
/// This function simulates directly from raw bytes without fetching anything via RPC
pub async fn simulate_signed_tx_bytes(
    signed_tx_bytes: &[u8],
    block_number: u64,
    rpc_url: &str,
) -> Result<SimulationOutput> {
    // Decode transaction bytes using ethers RLP
    use ethers_core::utils::rlp;
    
    let tx: EthersTransaction = rlp::decode(signed_tx_bytes)
        .map_err(|e| anyhow!("Failed to decode transaction: {}", e))?;
    
    // We need to fetch the block info for the simulation environment
    // This is the ONLY RPC call we make - just to get block environment
    let ethers_provider = EthersProvider::<Http>::try_from(rpc_url)?;
    let eth_client = Arc::new(ethers_provider);
    
    // Fetch block info for simulation environment
    let block = eth_client.get_block(block_number).await?
        .ok_or_else(|| anyhow!("Block {} not found", block_number))?;
    
    // Get chain ID
    let chain_id = eth_client.get_chainid().await?.as_u64();
    
    // Convert transaction and block to REVM types
    let (tx_env, block_env, cfg_env) = convert_to_revm_types(&tx, &block, chain_id, block_number)?;
    
    // Setup Alloy provider for database (state at parent block)
    use alloy_provider::DynProvider;
    let alloy_provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let alloy_provider_dyn: Arc<DynProvider<Ethereum>> = Arc::new(DynProvider::new(alloy_provider));
    
    // Setup database at parent block
    let fork_block_id = BlockId::from(block_number - 1);
    let alloy_db = AlloyDB::<Ethereum, Arc<DynProvider<Ethereum>>>::new(
        alloy_provider_dyn.clone(),
        fork_block_id
    );
    let cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db).unwrap());
    
    // Call the core simulation function
    let (output, _final_db) = simulate_core(tx_env, block_env, cfg_env, cache_db)?;
    
    Ok(output)
}

/// Convert Ethers types to REVM types
/// This is the crucial conversion that makes the module self-contained
fn convert_to_revm_types(
    tx: &EthersTransaction,
    block: &Block<H256>,
    chain_id: u64,
    block_number: u64,
) -> Result<(TxEnv, BlockEnv, CfgEnv)> {
    // Transaction environment
    let mut tx_env = TxEnv::default();
    tx_env.caller = ethers_to_revm_address(tx.from);
    tx_env.gas_limit = tx.gas.as_u64();
    tx_env.value = ethers_to_revm_u256(tx.value);
    tx_env.data = Bytes::from(tx.input.0.clone());
    tx_env.nonce = tx.nonce.as_u64();
    tx_env.chain_id = Some(chain_id);
    
    // Gas price (handle both legacy and EIP-1559)
    if let Some(max_fee) = tx.max_fee_per_gas {
        tx_env.gas_price = max_fee.as_u128();
        tx_env.gas_priority_fee = tx.max_priority_fee_per_gas.map(|p| p.as_u128());
    } else {
        tx_env.gas_price = tx.gas_price.unwrap_or_default().as_u128();
    }
    
    // Transaction kind
    tx_env.kind = if let Some(to) = tx.to {
        TransactTo::Call(ethers_to_revm_address(to))
    } else {
        TransactTo::Create
    };
    
    // Block environment
    let mut block_env = BlockEnv {
        number: U256::from(block_number),
        beneficiary: ethers_to_revm_address(block.author.unwrap_or_default()),
        timestamp: U256::from(block.timestamp.as_u64()),
        difficulty: U256::from_limbs(block.difficulty.0),
        prevrandao: Some(block.mix_hash.unwrap_or_default().0.into()),
        basefee: block.base_fee_per_gas.unwrap_or_default().as_u64(),
        gas_limit: block.gas_limit.as_u64(),
        blob_excess_gas_and_price: None,
    };
    
    // For Cancun blocks, set blob gas fields  
    if block_number >= 19_426_587 {
        // Set default blob gas values for Cancun
        block_env.blob_excess_gas_and_price = Some(BlobExcessGasAndPrice {
            excess_blob_gas: 0,
            blob_gasprice: 1, // Minimum blob gas price
        });
    }
    
    // Config environment with automatic hardfork detection
    let mut cfg_env = CfgEnv::default();
    cfg_env.chain_id = chain_id;
    cfg_env.spec = spec_id_from_block_number(block_number);
    
    Ok((tx_env, block_env, cfg_env))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simulate_signed_tx() {
        let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
            .parse::<H256>().unwrap();
        
        let result = simulate_signed_tx(tx_hash, "http://127.0.0.1:8545").await;
        
        assert!(result.is_ok(), "Simulation should succeed");
        let output = result.unwrap();
        assert_eq!(output.gas_used, 315099, "Gas usage should match");
        assert!(matches!(output.result_type, crate::simulate_signed_tx::simulation_core::ExecutionResultType::Success(_)), "Transaction should succeed");
    }
}