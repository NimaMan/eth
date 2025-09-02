/// Call simulation (unsigned transactions)
/// 
/// This module handles simulation of unsigned transactions (like debug_traceCall).
/// These are contract unsigned_txs that haven't been signed, used for read operations
/// and testing transaction effects without broadcasting.

use crate::{
    simulator::TxSimulator,
    types::{SimulationResult, FullSimulationResult},
    simulation_revert_decoder::decode_revert_data,
};
use eyre::Result;
use tokio::task;

// Reth imports
use reth_revm::database::StateProviderDatabase;
use reth_revm::db::CacheDB;
use reth_provider::HeaderProvider;
use reth_evm::{ConfigureEvm, Evm};
use revm::{Database, DatabaseCommit};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use alloy_rpc_types_trace::geth::CallConfig;
use alloy_primitives::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// Unsigned transaction for simulation (no signature required)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub gas: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub value: Option<U256>,
    pub data: Option<Bytes>,
    pub nonce: Option<u64>,
}

impl TxSimulator {
    /// Simulate an unsigned transaction at specific block
    pub async fn simulate_unsigned_transaction_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<SimulationResult> {
        let simulator = self.clone();
        
        task::spawn_blocking(move || {
            
            let provider = simulator.provider_factory.provider()?;
            let header = provider.header_by_number(block_number)?
                .ok_or_else(|| eyre::eyre!("Provider did not return header for block {}", block_number))?;
            
            let state = simulator.provider_factory.history_by_block_number(block_number)?;
            
            let mut db = CacheDB::new(StateProviderDatabase::new(state));
            
            let mut inspector = TracingInspector::new(TracingInspectorConfig::default_parity());
            
            let evm_env = simulator.evm_config.evm_env(&header);
            
            // Get base fee for gas price adjustment
            let base_fee = header.base_fee_per_gas.map(|v| v as u128);
            
            // Create transaction environment
            let tx_env = simulator.create_tx_env(&unsigned_tx, evm_env.block_env.gas_limit as u128, base_fee, &mut db)?;
            
            let mut evm = simulator.evm_config.evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);
            
            let res = evm.transact(tx_env)?;
            
            db.commit(res.state);
            
            Ok(SimulationResult {
                success: res.result.is_success(),
                gas_used: res.result.gas_used(),
                revert_reason: if res.result.is_success() { 
                    None 
                } else {
                    // Extract and decode the actual revert data
                    res.result.output()
                        .map(|bytes| decode_revert_data(&bytes))
                        .or_else(|| Some("Transaction reverted without data".to_string()))
                },
            })
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }
    
    /// Simulate an unsigned transaction with detailed unsigned_tx trace
    pub async fn simulate_unsigned_transaction_with_trace(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: Option<u64>,
    ) -> Result<FullSimulationResult> {
        let block = block_number.unwrap_or(self.get_latest_block()?);
        let simulator = self.clone();
        
        task::spawn_blocking(move || {
            
            let provider = simulator.provider_factory.provider()?;
            let header = provider.header_by_number(block)?
                .ok_or_else(|| eyre::eyre!("No header for block {}", block))?;
            
            let state = simulator.provider_factory.history_by_block_number(block)?;
            
            let mut db = CacheDB::new(StateProviderDatabase::new(state));
            
            // Create tracer with unsigned_tx config
            let unsigned_tx_config = TracingInspectorConfig::default_geth()
                .set_record_logs(true);
            let mut inspector = TracingInspector::new(unsigned_tx_config);
            
            let evm_env = simulator.evm_config.evm_env(&header);
            
            // Get base fee for gas price adjustment
            let base_fee = header.base_fee_per_gas.map(|v| v as u128);
            
            // Create transaction environment
            let tx_env = simulator.create_tx_env(&unsigned_tx, evm_env.block_env.gas_limit as u128, base_fee, &mut db)?;
            let gas_limit = tx_env.gas_limit;
            
            let mut evm = simulator.evm_config.evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);
            
            let res = evm.transact(tx_env)?;
            
            db.commit(res.state);
            
            let success = res.result.is_success();
            let gas_used = res.result.gas_used();
            let revert_reason = if success {
                None
            } else {
                // Extract and decode the actual revert data
                res.result.output()
                    .map(|bytes| decode_revert_data(&bytes))
                    .or_else(|| Some("Transaction reverted without data".to_string()))
            };
            
            // Extract unsigned_tx trace
            let unsigned_tx_frame = inspector
                .with_transaction_gas_limit(gas_limit)
                .into_geth_builder()
                .geth_call_traces(CallConfig::default().with_log(), gas_used);
            
            Ok(FullSimulationResult {
                success,
                gas_used,
                revert_reason,
                call_trace: unsigned_tx_frame,
            })
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }
    
    /// Simulate unsigned transaction with full trace at specific block
    /// 
    /// This method provides maximum detail including internal transactions,
    /// logs, and complete unsigned_tx traces. Use this for comprehensive analysis
    /// of transaction effects.
    pub async fn simulate_unsigned_transaction_with_full_trace_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<FullSimulationResult> {
        let simulator = self.clone();
        
        task::spawn_blocking(move || {
            
            // Get provider and state at specific block
            let provider = simulator.provider_factory.provider()?;
            let header = provider.header_by_number(block_number)?
                .ok_or_else(|| eyre::eyre!("Provider did not return header for block {}", block_number))?;
            
            // Get state at the block
            let state = simulator.provider_factory.history_by_block_number(block_number)?;
            
            // Create database
            let mut db = CacheDB::new(StateProviderDatabase::new(state));
            
            // Create TracingInspector with full config (logs enabled)
            let unsigned_tx_config = TracingInspectorConfig::default_geth()
                .set_record_logs(true)
                .set_steps(true);
            let mut inspector = TracingInspector::new(unsigned_tx_config);
            
            // Get EVM environment
            let evm_env = simulator.evm_config.evm_env(&header);
            
            // Create transaction environment from UnsignedTransaction
            let base_fee = header.base_fee_per_gas.map(|v| v as u128);
            let tx_env = simulator.create_tx_env(&unsigned_tx, evm_env.block_env.gas_limit as u128, base_fee, &mut db)?;
            let gas_limit = tx_env.gas_limit;
            
            // Create EVM with inspector
            let mut evm = simulator.evm_config.evm_with_env_and_inspector(&mut db, evm_env, &mut inspector);
            
            // Execute transaction
            let res = evm.transact(tx_env)?;
            
            // Commit changes to database
            db.commit(res.state);
            
            // Check if transaction was successful
            let success = res.result.is_success();
            let gas_used = res.result.gas_used();
            let revert_reason = if !success {
                res.result.output().map(|bytes| {
                    // Try to decode revert reason
                    if bytes.len() >= 4 {
                        format!("Reverted: 0x{}", hex::encode(bytes))
                    } else {
                        "Reverted".to_string()
                    }
                })
            } else {
                None
            };
            
            // Use geth builder to get unsigned_tx traces with full details
            let unsigned_tx_config = CallConfig::default()
                .with_log();
            let unsigned_tx_frame = inspector
                .with_transaction_gas_limit(gas_limit)
                .into_geth_builder()
                .geth_call_traces(unsigned_tx_config, gas_used);
            
            Ok(FullSimulationResult {
                success,
                gas_used,
                revert_reason,
                call_trace: unsigned_tx_frame,
            })
        })
        .await
        .map_err(|e| eyre::eyre!("Spawn blocking failed: {}", e))?
    }
    
    /// Helper to create transaction environment from UnsignedTransaction
    fn create_tx_env<DB: revm::Database>(
        &self,
        request: &UnsignedTransaction,
        block_gas_limit: u128,
        base_fee: Option<u128>,
        db: &mut DB,
    ) -> Result<revm::context::TxEnv> {
        use revm::context::TxEnv;
        use alloy_primitives::TxKind;
        
        // Determine transaction type
        let tx_type = if request.max_fee_per_gas.is_some() {
            2 // EIP-1559
        } else {
            0 // Legacy
        };
        
        // Get caller address
        let caller = request.from.unwrap_or_default();
        
        // Get nonce from state if not provided
        let nonce = if let Some(nonce) = request.nonce {
            nonce
        } else {
            // Query the database for the account's nonce
            match db.basic(caller.into()) {
                Ok(Some(acc)) => acc.nonce,
                _ => 0,
            }
        };
        
        // Calculate fees with base fee awareness
        let (gas_price, gas_priority_fee) = if tx_type == 2 {
            // EIP-1559
            let priority_fee = request.max_priority_fee_per_gas.unwrap_or(100_000_000_000); // 100 gwei default

            // If max_fee_per_gas is provided, use it; otherwise calculate from base fee
            let max_fee = if let Some(max_fee) = request.max_fee_per_gas {
                max_fee
            } else {
                // For EIP-1559, we need a base fee - it should always be available post-London
                let base = base_fee.expect("EIP-1559 transaction requires base fee (post-London)");
                // Set max fee to 10x base fee + priority fee as upper bound
                base.saturating_mul(10).saturating_add(priority_fee)
            };
            
            (max_fee, Some(priority_fee))
        } else {
            // Legacy
            let price = if let Some(price) = request.gas_price {
                price
            } else {
                // For legacy transactions on post-London blocks, use base fee
                // For pre-London blocks, base_fee will be None, use a reasonable gas price
                let base = base_fee.unwrap_or(20_000_000_000u128); // 20 gwei for pre-London
                // For legacy transactions, use 3x base fee to ensure simulation succeeds
                base.saturating_mul(3)
            };
            (price, None)
        };
        
        // Create TxEnv - no signature needed!
        Ok(TxEnv {
            tx_type,
            caller: caller.into(),
            gas_limit: request.gas.unwrap_or(block_gas_limit as u64),
            gas_price,
            gas_priority_fee,
            kind: if let Some(to) = request.to {
                TxKind::Call(to)
            } else {
                TxKind::Create
            },
            value: request.value.unwrap_or_default(),
            data: request.data.clone().unwrap_or_default(),
            nonce,
            chain_id: Some(1), // Mainnet
            access_list: Default::default(),
            blob_hashes: Default::default(),
            max_fee_per_blob_gas: 0,
            authorization_list: Default::default(),
        })
    }
    
    // ========== Backward Compatibility Aliases ==========
    
    /// Legacy alias for simulate_unsigned_transaction_at_block with latest block
    /// 
    /// This method exists for backward compatibility with code that used
    /// the old naming convention. New code should use simulate_unsigned_transaction_at_block.
    pub async fn simulate_call(&self, unsigned_tx: UnsignedTransaction) -> Result<SimulationResult> {
        let block = self.get_latest_block()?;
        self.simulate_unsigned_transaction_at_block(unsigned_tx, block).await
    }
    
    /// Legacy alias for simulate_unsigned_transaction_at_block
    /// 
    /// This method exists for backward compatibility with code that used
    /// the old naming convention. New code should use simulate_unsigned_transaction_at_block.
    pub async fn simulate_call_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<SimulationResult> {
        self.simulate_unsigned_transaction_at_block(unsigned_tx, block_number).await
    }
    
    /// Alias for simulate_unsigned_transaction_at_block with latest block (shorter name)
    pub async fn simulate_unsigned_tx(&self, unsigned_tx: UnsignedTransaction) -> Result<SimulationResult> {
        let block = self.get_latest_block()?;
        self.simulate_unsigned_transaction_at_block(unsigned_tx, block).await
    }
    
    /// Alias for simulate_unsigned_transaction_at_block (shorter name)
    pub async fn simulate_unsigned_tx_at_block(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<SimulationResult> {
        self.simulate_unsigned_transaction_at_block(unsigned_tx, block_number).await
    }
}