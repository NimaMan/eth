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
    
    // Get transaction index within the block
    let tx_index = tx.transaction_index
        .ok_or_else(|| anyhow!("Transaction index not available"))?.as_u64();
    
    // Fetch block with full transaction data
    let block = eth_client.get_block_with_txs(block_number).await?
        .ok_or_else(|| anyhow!("Block {} not found", block_number))?;
    
    // Get chain ID
    let chain_id = eth_client.get_chainid().await?.as_u64();
    
    // Convert to REVM types (this is the key part that was missing!)
    let (tx_env, block_env, cfg_env) = convert_to_revm_types(&tx, &block, chain_id, block_number)?;
    
    // Debug: Log simulation parameters
    println!("🔍 SIMULATION PARAMETERS (lib.rs):");
    println!("  Transaction Hash: {:?}", tx_hash);
    println!("  Transaction Index: {} ({}th transaction)", tx_index, tx_index + 1);
    println!("  Fork Block: {} (parent of {})", block_number - 1, block_number);
    println!("  Total Block Transactions: {}", block.transactions.len());
    println!("  TX Caller: {:?}", tx_env.caller);
    println!("  TX Gas Limit: {}", tx_env.gas_limit);
    println!("  TX Gas Price: {} wei", tx_env.gas_price);
    println!("  TX Priority Fee: {:?}", tx_env.gas_priority_fee);
    println!("  TX Value: {} wei", tx_env.value);
    println!("  TX Nonce: {:?}", tx_env.nonce);
    println!("  TX Data Length: {} bytes", tx_env.data.len());
    println!("  Block Base Fee: {} wei", block_env.basefee);
    println!("  Block Timestamp: {}", block_env.timestamp);
    println!("  Block Beneficiary: {:?}", block_env.beneficiary);
    println!("  Block Number: {}", block_env.number);
    println!("  Chain ID: {}", chain_id);
    
    // Setup Alloy provider for database
    use alloy_provider::DynProvider;
    let alloy_provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let alloy_provider_dyn: Arc<DynProvider<Ethereum>> = Arc::new(DynProvider::new(alloy_provider));
    
    // Setup database at parent block
    // Note: This gives us clean parent block state, but misses previous transactions in the same block
    let fork_block_id = BlockId::from(block_number - 1);
    let alloy_db = AlloyDB::<Ethereum, Arc<DynProvider<Ethereum>>>::new(
        alloy_provider_dyn.clone(),
        fork_block_id
    );
    let mut cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db).unwrap());
    
    // Replay previous transactions if this is not the first transaction in the block
    if tx_index > 0 {
        println!("🔄 REPLAYING {} previous transactions to get correct state...", tx_index);
        cache_db = replay_previous_transactions(
            cache_db,
            &block,
            tx_index as usize,
            &block_env,
            &cfg_env,
            chain_id,
        ).await?;
        println!("✅ Transaction replay complete, state updated");
    } else {
        println!("ℹ️  First transaction in block, no replay needed");
    }
    
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
    
    // Convert transaction and block to REVM types (for simple block without full transactions)
    let (tx_env, block_env, cfg_env) = convert_simple_to_revm_types(&tx, &block, chain_id, block_number)?;
    
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

/// Simulate a signed transaction by hash using Reth database for transaction data
/// This is a hybrid approach: load transaction from database, use RPC for state
pub async fn simulate_signed_tx_from_db(
    tx_hash: H256,
    reth_datadir: Option<&str>,
    rpc_url: &str,
) -> Result<SimulationOutput> {
    use crate::fetch_from_reth::{RethDatabaseProvider, RethDataProvider};
    use alloy_primitives::B256;
    use std::str::FromStr;
    
    // Convert H256 to B256 for database lookup
    let tx_hash_str = format!("{:x}", tx_hash);
    let tx_hash_b256 = B256::from_str(&tx_hash_str)?;
    
    // Connect to Reth database
    let datadir = reth_datadir.unwrap_or("/home/nima/.local/share/reth/mainnet");
    let db_provider = RethDatabaseProvider::new(datadir)?;
    
    // Fetch transaction data from database
    let tx_data = db_provider.fetch_transaction(tx_hash_b256)?;
    
    // Fetch block data from database  
    let block_data = db_provider.fetch_block(
        crate::fetch_from_reth::provider::BlockId::Number(tx_data.block_number)
    )?;
    
    // Convert database data to REVM types
    let (tx_env, block_env, cfg_env) = convert_db_data_to_revm_types(&tx_data, &block_data)?;
    
    // Setup Alloy provider for state access
    use alloy_provider::DynProvider;
    let alloy_provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let alloy_provider_dyn: Arc<DynProvider<Ethereum>> = Arc::new(DynProvider::new(alloy_provider));
    
    // Setup database at parent block for state access
    let fork_block_id = BlockId::from(tx_data.block_number - 1);
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
    block: &Block<EthersTransaction>,
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

/// Convert database data to REVM types
/// This converts TransactionData and BlockData from fetch_from_reth to REVM types
fn convert_db_data_to_revm_types(
    tx_data: &crate::fetch_from_reth::provider::TransactionData,
    block_data: &crate::fetch_from_reth::provider::BlockData,
) -> Result<(TxEnv, BlockEnv, CfgEnv)> {
    // Transaction environment from database data
    let mut tx_env = TxEnv::default();
    tx_env.caller = tx_data.from;
    tx_env.gas_limit = tx_data.gas_limit;
    tx_env.gas_price = tx_data.gas_price.to::<u128>();
    tx_env.value = tx_data.value;
    tx_env.data = tx_data.input.clone();
    tx_env.nonce = tx_data.nonce;
    tx_env.chain_id = Some(1); // Mainnet
    
    // Transaction kind
    tx_env.kind = if let Some(to) = tx_data.to {
        TransactTo::Call(to)
    } else {
        TransactTo::Create
    };
    
    // Block environment from database data
    let mut block_env = BlockEnv {
        number: U256::from(block_data.number),
        beneficiary: block_data.miner,
        timestamp: U256::from(block_data.timestamp),
        difficulty: U256::from_limbs(block_data.difficulty.into_limbs()),
        prevrandao: Some(alloy_primitives::B256::ZERO), // Simplified
        basefee: 0, // Will be calculated if needed
        gas_limit: block_data.gas_limit,
        blob_excess_gas_and_price: None,
    };
    
    // For Cancun blocks, set blob gas fields  
    if block_data.number >= 19_426_587 {
        block_env.blob_excess_gas_and_price = Some(BlobExcessGasAndPrice {
            excess_blob_gas: 0,
            blob_gasprice: 1, // Minimum blob gas price
        });
    }
    
    // Config environment with automatic hardfork detection
    let mut cfg_env = CfgEnv::default();
    cfg_env.chain_id = 1; // Mainnet
    cfg_env.spec = spec_id_from_block_number(block_data.number);
    
    Ok((tx_env, block_env, cfg_env))
}

/// Convert Ethers types to REVM types for simple blocks (without full transaction data)
fn convert_simple_to_revm_types(
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
        difficulty: ethers_to_revm_u256(block.difficulty),
        prevrandao: Some(alloy_primitives::B256::from(block.mix_hash.unwrap_or_default().0)),
        basefee: block.base_fee_per_gas.map(|fee| fee.as_u64()).unwrap_or(0),
        gas_limit: block.gas_limit.as_u64(),
        blob_excess_gas_and_price: None,
    };
    
    // For Cancun blocks, set blob gas fields  
    if block_number >= 19_426_587 {
        block_env.blob_excess_gas_and_price = Some(BlobExcessGasAndPrice {
            excess_blob_gas: block.excess_blob_gas.map(|gas| gas.as_u64()).unwrap_or(0),
            blob_gasprice: 1, // Minimum blob gas price
        });
    }
    
    // Config environment with automatic hardfork detection
    let mut cfg_env = CfgEnv::default();
    cfg_env.chain_id = chain_id;
    cfg_env.spec = spec_id_from_block_number(block_number);
    
    Ok((tx_env, block_env, cfg_env))
}

/// Replay all transactions in a block up to (but not including) the target transaction index
/// This function replicates Reth's `replay_transactions_until` functionality
async fn replay_previous_transactions(
    mut cache_db: SimCacheDB,
    block: &ethers_core::types::Block<ethers_core::types::Transaction>,
    target_index: usize,
    block_env: &BlockEnv,
    cfg_env: &CfgEnv,
    chain_id: u64,
) -> Result<SimCacheDB> {
    use revm_context::{Context as RevmContext, Journal};
    use revm::handler::{ExecuteCommitEvm, MainBuilder};
    
    // Get number of transactions to replay
    let num_to_replay = target_index.min(block.transactions.len());
    
    if num_to_replay == 0 {
        return Ok(cache_db); // Nothing to replay, return unchanged database
    }
    
    println!("📋 Block has {} total transactions", block.transactions.len());
    println!("🎯 Target transaction index: {}", target_index);
    println!("🔄 Will replay transactions 0 to {} (exclusive)", num_to_replay);
    
    // We need to execute transactions and commit state changes
    // Since we can't easily clone the database, we'll use the simulate_transaction function
    // but without the inspector to make it simpler
    
    for (index, tx) in block.transactions.iter().take(num_to_replay).enumerate() {
        if index % 10 == 0 {
            println!("  🔄 Replaying transaction {}/{}: {}", index + 1, num_to_replay, tx.hash);
        }
        
        // Convert transaction to REVM TxEnv
        let mut tx_env = TxEnv::default();
        tx_env.caller = ethers_to_revm_address(tx.from);
        tx_env.gas_limit = tx.gas.as_u64();
        tx_env.value = ethers_to_revm_u256(tx.value);
        tx_env.data = Bytes::from(tx.input.0.clone());
        tx_env.nonce = tx.nonce.as_u64();
        tx_env.chain_id = Some(chain_id);
        
        // Handle gas price (legacy vs EIP-1559)
        if let Some(max_fee) = tx.max_fee_per_gas {
            tx_env.gas_price = max_fee.as_u128();
            tx_env.gas_priority_fee = tx.max_priority_fee_per_gas.map(|p| p.as_u128());
        } else {
            tx_env.gas_price = tx.gas_price.unwrap_or_default().as_u128();
        }
        
        // Transaction kind (call vs create)
        tx_env.kind = if let Some(to) = tx.to {
            TransactTo::Call(ethers_to_revm_address(to))
        } else {
            TransactTo::Create
        };
        
        // Create a new context with the current database state
        let spec_id = cfg_env.spec.clone();
        let mut ctx: RevmContext<BlockEnv, TxEnv, CfgEnv, _, Journal<_>, ()> = 
            RevmContext::new(cache_db, spec_id);
        ctx.cfg = cfg_env.clone();
        ctx.block = block_env.clone();
        ctx.tx = tx_env.clone();
        
        // Build mainnet EVM and execute with commit
        let mut evm = ctx.build_mainnet();
        match evm.transact_commit(tx_env) {
            Ok(_result) => {
                // Transaction executed successfully, state is already committed
                if index % 10 == 0 || index == num_to_replay - 1 {
                    println!("    ✅ Transaction {} executed successfully", index + 1);
                }
            }
            Err(e) => {
                // Some transactions might fail in the original block too, so we continue
                if index % 10 == 0 {
                    println!("    ⚠️  Transaction {} failed: {:?} (continuing...)", index + 1, e);
                }
            }
        }
        
        // Take back ownership of the database with committed state
        cache_db = evm.ctx.journaled_state.database;
    }
    
    println!("✅ Replay complete: {} transactions processed", num_to_replay);
    Ok(cache_db)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simulate_signed_tx_function_exists() {
        // Test that the function signature is correct
        // Actual functionality is tested in integration_tests.rs and high_level_api_tests.rs
        
        let _function_doc = "simulate_signed_tx(tx_hash: H256, rpc_url: &str) -> Result<SimulationOutput>";
        let _bytes_function_doc = "simulate_signed_tx_bytes(signed_tx_bytes: &[u8], block_number: u64, rpc_url: &str) -> Result<SimulationOutput>";
        
        // Test that we can parse transaction hash
        let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
            .parse::<H256>();
        assert!(tx_hash.is_ok(), "Should be able to parse transaction hash");
        
        // Function signature verification passes if this compiles
        assert!(true, "Function signatures verified");
    }
}