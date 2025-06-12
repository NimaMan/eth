//! Main entry point for simulating signed transactions
//! This module handles all conversions from signed transactions to REVM types

use anyhow::{Result, anyhow};
use alloy_primitives::{B256, Address as AlloyAddress};
use alloy_rpc_types::{Transaction as RpcTransaction, TransactionSigned};
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

/// Main function to simulate a signed transaction from its hash
/// 
/// # Arguments
/// * `tx_hash` - The transaction hash
/// * `provider` - Provider for fetching blockchain state
/// 
/// # Returns
/// * Simulation output with gas usage, logs, and execution result
pub async fn simulate_signed_transaction(
    tx_hash: B256,
    provider: Arc<DynProvider<Ethereum>>,
) -> Result<SimulationOutput> {
    // Fetch the transaction
    let tx = provider.get_transaction_by_hash(tx_hash).await?
        .ok_or_else(|| anyhow!("Transaction not found"))?;
    
    // Get block number from transaction
    let block_number = tx.block_number
        .ok_or_else(|| anyhow!("Transaction not yet mined"))?;
    
    // Fetch block for environment setup
    let block = provider.get_block_by_number(block_number.into())
        .await?
        .ok_or_else(|| anyhow!("Block not found"))?;
    
    // Setup REVM environments
    let (tx_env, block_env, cfg_env) = convert_to_revm_env(
        &tx,
        &block,
        block_number
    )?;
    
    // Setup database at the parent block
    let fork_block_id = BlockId::Number((block_number - 1).into());
    let alloy_db = AlloyDB::<Ethereum, Arc<DynProvider<Ethereum>>>::new(
        provider.clone(),
        fork_block_id
    );
    let cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db).unwrap());
    
    // Simulate transaction
    let (output, _final_db) = simulate_transaction(
        tx_env,
        block_env,
        cfg_env,
        cache_db
    )?;
    
    Ok(output)
}

/// Simulate a signed transaction bytes directly
/// 
/// # Arguments
/// * `signed_tx_bytes` - The RLP encoded signed transaction
/// * `provider` - Provider for fetching blockchain state
/// * `block_number` - Block number to simulate at (None for latest)
/// 
/// # Returns
/// * Simulation output with gas usage, logs, and execution result
pub async fn simulate_signed_transaction_bytes(
    signed_tx_bytes: &[u8],
    provider: Arc<DynProvider<Ethereum>>,
    block_number: Option<u64>,
) -> Result<SimulationOutput> {
    // Decode the signed transaction
    use alloy_rlp::Decodable;
    let signed_tx = TransactionSigned::decode(&mut &signed_tx_bytes[..])?;
    
    // Get block number
    let block_number = if let Some(bn) = block_number {
        bn
    } else {
        provider.get_block_number().await?
    };
    
    // Fetch block for environment setup
    let block = provider.get_block_by_number(block_number.into())
        .await?
        .ok_or_else(|| anyhow!("Block not found"))?;
    
    // Convert to RPC transaction for compatibility
    let tx = signed_tx_to_rpc_transaction(&signed_tx, block_number);
    
    // Setup REVM environments
    let (tx_env, block_env, cfg_env) = convert_to_revm_env(
        &tx,
        &block,
        block_number
    )?;
    
    // Setup database at the parent block
    let fork_block_id = BlockId::Number((block_number - 1).into());
    let alloy_db = AlloyDB::<Ethereum, Arc<DynProvider<Ethereum>>>::new(
        provider.clone(),
        fork_block_id
    );
    let cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db).unwrap());
    
    // Simulate transaction
    let (output, _final_db) = simulate_transaction(
        tx_env,
        block_env,
        cfg_env,
        cache_db
    )?;
    
    Ok(output)
}

/// Simulate with CallTracer to capture internal transfers
pub async fn simulate_signed_transaction_with_tracer(
    tx_hash: B256,
    provider: Arc<DynProvider<Ethereum>>,
) -> Result<(SimulationOutput, Vec<crate::simulate_signed_tx::InternalTransfer>)> {
    // This would integrate CallTracer with the simulation
    // For now, just simulate and return empty transfers
    let output = simulate_signed_transaction(tx_hash, provider).await?;
    
    // TODO: Integrate CallTracer properly
    let transfers = vec![];
    
    Ok((output, transfers))
}

/// Convert RPC transaction and block to REVM environments
fn convert_to_revm_env(
    tx: &RpcTransaction,
    block: &alloy_rpc_types::Block,
    block_number: u64,
) -> Result<(TxEnv, BlockEnv, CfgEnv)> {
    // Transaction environment
    let mut tx_env = TxEnv::default();
    
    // Sender
    tx_env.caller = Address::from(tx.from.0);
    
    // Basic transaction fields
    tx_env.gas_limit = tx.gas.try_into().unwrap_or(u64::MAX);
    tx_env.value = U256::from_limbs(tx.value.into_limbs());
    tx_env.data = Bytes::from(tx.input.to_vec());
    tx_env.nonce = Some(tx.nonce);
    tx_env.chain_id = tx.chain_id;
    
    // Gas price handling (EIP-1559 vs legacy)
    if let Some(max_fee) = tx.max_fee_per_gas {
        tx_env.gas_price = U256::from(max_fee);
        tx_env.gas_priority_fee = tx.max_priority_fee_per_gas.map(U256::from);
    } else {
        // Legacy transaction
        tx_env.gas_price = U256::from(tx.gas_price.unwrap_or_default());
    }
    
    // Transaction type (Call or Create)
    tx_env.transact_to = if let Some(to) = tx.to {
        TransactTo::Call(Address::from(to.0))
    } else {
        TransactTo::Create
    };
    
    // Block environment
    let block_env = BlockEnv {
        number: U256::from(block_number),
        beneficiary: Address::from(block.header.miner.0),
        timestamp: U256::from(block.header.timestamp),
        difficulty: U256::from_limbs(block.header.difficulty.into_limbs()),
        prevrandao: block.header.mix_hash.map(|h| h.0.into()),
        basefee: block.header.base_fee_per_gas.unwrap_or(0),
        gas_limit: block.header.gas_limit,
        blob_excess_gas_and_price: None,
    };
    
    // Configuration environment
    let mut cfg_env = CfgEnv::default();
    cfg_env.chain_id = tx.chain_id.unwrap_or(1);
    cfg_env.spec = spec_id_from_block_number(block_number);
    
    Ok((tx_env, block_env, cfg_env))
}

/// Convert signed transaction to RPC transaction format
fn signed_tx_to_rpc_transaction(signed_tx: &TransactionSigned, block_number: u64) -> RpcTransaction {
    // This is a simplified conversion - in production you'd handle all fields properly
    RpcTransaction {
        hash: signed_tx.hash(),
        nonce: signed_tx.nonce(),
        block_hash: None,
        block_number: Some(block_number),
        transaction_index: None,
        from: signed_tx.recover_signer().unwrap_or_default(),
        to: signed_tx.to(),
        value: signed_tx.value(),
        gas_price: signed_tx.gas_price(),
        gas: signed_tx.gas_limit() as u128,
        max_fee_per_gas: signed_tx.max_fee_per_gas(),
        max_priority_fee_per_gas: signed_tx.max_priority_fee_per_gas(),
        input: signed_tx.input().clone(),
        v: None,
        r: None,
        s: None,
        chain_id: signed_tx.chain_id(),
        access_list: None,
        transaction_type: Some(signed_tx.tx_type() as u64),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_signed_tx_simulation() {
        // This would test with an actual signed transaction
        // For now, just verify the module compiles
    }
}