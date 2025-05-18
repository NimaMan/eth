/*
* Transaction Simulator
*
* This module implements transaction simulation using REVM to verify
* transactions before execution and estimate their effects.
*
* Algorithm:
* 1. Create an EVM environment from the current blockchain state
* 2. Set up transaction parameters (from, to, value, data, gas)
* 3. Execute the transaction in the EVM
* 4. Track state changes (balances, storage)
* 5. Return simulation results including gas used and execution outcome
*/

use crate::tx_execution::types::{Transaction, SimulationResult, SimulationError, StateChange, StateChangeType};
use crate::tx_execution::error::{TxExecutionError, TxResult};
use crate::tx_execution::config::TxExecutionConfig;

use ethers::providers::{Provider, Http};
use ethers::types::{Address, U256, Bytes, H256, BlockId, BlockNumber};
use revm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{
        AccountInfo, Bytecode, Env, ExecutionResult, Output, 
        TransactTo, TxEnv, U256 as rU256,
    },
    Database, EVM,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Transaction simulator for checking transactions before execution
pub struct TxSimulator {
    /// Ethereum provider
    provider: Arc<Provider<Http>>,
    
    /// Configuration
    config: TxExecutionConfig,
    
    /// Block number to simulate at (None for latest)
    block_number: Option<u64>,
}

impl TxSimulator {
    /// Create a new transaction simulator
    pub fn new(provider: Arc<Provider<Http>>, config: TxExecutionConfig) -> Self {
        Self {
            provider,
            config,
            block_number: None,
        }
    }
    
    /// Set the block number to simulate at
    pub fn with_block_number(mut self, block_number: u64) -> Self {
        self.block_number = Some(block_number);
        self
    }
    
    /// Simulate a transaction to check if it will succeed and estimate effects
    pub async fn simulate_transaction(&self, transaction: &Transaction) -> TxResult<SimulationResult> {
        // Create database from provider
        let block_id = match self.block_number {
            Some(num) => BlockId::Number(BlockNumber::Number(num.into())),
            None => BlockId::Number(BlockNumber::Latest),
        };
        
        let db = EthersDB::new(self.provider.clone(), block_id).unwrap();
        let mut cache_db = CacheDB::new(db);
        
        // Set up EVM environment
        let mut evm = EVM::new();
        evm.database(cache_db);
        
        // Get block information for environment
        let block = self.provider.get_block(block_id).await?
            .ok_or_else(|| TxExecutionError::Internal("Block not found".to_string()))?;
        
        // Set up environment
        let mut env = Env::default();
        env.block.number = rU256::from(block.number.unwrap_or_default().as_u64());
        env.block.timestamp = rU256::from(block.timestamp.as_u64());
        env.block.basefee = rU256::from_be_bytes(block.base_fee_per_gas.unwrap_or_default().0);
        env.cfg.chain_id = self.config.chain_id;
        
        // Set up transaction environment
        let params = &transaction.params;
        env.tx.caller = params.from.0.into();
        env.tx.gas_limit = params.gas_limit.as_u64();
        env.tx.gas_price = match params.gas_price {
            Some(price) => rU256::from_be_bytes(price.0),
            None => rU256::from_be_bytes(block.base_fee_per_gas.unwrap_or_default().0),
        };
        env.tx.value = rU256::from_be_bytes(params.value.0);
        env.tx.data = params.data.0.clone();
        
        env.tx.transact_to = match params.to {
            Some(to) => TransactTo::Call(to.0.into()),
            None => TransactTo::Create,
        };
        
        // Execute transaction
        evm.env = env;
        let result = evm.transact_commit();
        
        // Process result
        match result {
            Ok(exec_result) => {
                match exec_result {
                    ExecutionResult::Success { gas_used, gas_refunded, output, logs, .. } => {
                        // Extract state changes
                        let state_changes = self.extract_state_changes(&evm);
                        
                        // Extract logs
                        let formatted_logs = logs.iter()
                            .map(|log| Bytes::from(log.data.clone()))
                            .collect();
                        
                        // Prepare result bytes
                        let result_bytes = match output {
                            Output::Call(data) => Some(Bytes::from(data)),
                            Output::Create(data, _) => Some(Bytes::from(data)),
                        };
                        
                        // Extract created contracts
                        let created_contracts = match output {
                            Output::Create(_, addr) => vec![Address::from_slice(&addr.0)],
                            _ => vec![],
                        };
                        
                        let gas_limit = params.gas_limit;
                        let effective_gas_price = params.gas_price.unwrap_or_default();
                        
                        Ok(SimulationResult {
                            success: true,
                            gas_used: gas_used.into(),
                            gas_limit,
                            result: result_bytes,
                            error: None,
                            state_changes,
                            created_contracts,
                            logs: formatted_logs,
                            trace: None,
                            effective_gas_price,
                            transaction_cost: gas_used.into() * effective_gas_price,
                        })
                    },
                    ExecutionResult::Revert { gas_used, output } => {
                        let error = if output.is_empty() {
                            SimulationError::Revert("Unknown revert reason".to_string())
                        } else {
                            // Parse revert reason if available
                            SimulationError::Revert(format!("{:?}", output))
                        };
                        
                        let gas_limit = params.gas_limit;
                        let effective_gas_price = params.gas_price.unwrap_or_default();
                        
                        Ok(SimulationResult::failure(
                            error,
                            gas_used.into(),
                            gas_limit,
                            effective_gas_price,
                        ))
                    },
                    ExecutionResult::Halt { gas_used, error } => {
                        let error = SimulationError::Other(format!("Execution halted: {:?}", error));
                        
                        let gas_limit = params.gas_limit;
                        let effective_gas_price = params.gas_price.unwrap_or_default();
                        
                        Ok(SimulationResult::failure(
                            error,
                            gas_used.into(),
                            gas_limit,
                            effective_gas_price,
                        ))
                    },
                }
            },
            Err(e) => {
                Err(TxExecutionError::SimulationFailed(format!("Simulation error: {:?}", e)))
            }
        }
    }
    
    // Extract state changes from EVM after execution
    fn extract_state_changes(&self, evm: &EVM<CacheDB<EthersDB>>) -> Vec<StateChange> {
        let mut changes = Vec::new();
        
        // Access the database
        let db = evm.db();
        if let Some(db) = db {
            // Extract accounts that were modified
            for (address, account_info) in db.cache.iter() {
                let eth_address = Address::from_slice(&address.0);
                
                // Add balance changes
                changes.push(StateChange {
                    change_type: StateChangeType::Balance,
                    address: eth_address,
                    slot: None,
                    old_value: None, // We don't track old values in this simple version
                    new_value: Some(Bytes::from(account_info.info.balance.to_be_bytes().to_vec())),
                });
                
                // Add code changes if any
                if let Some(code) = &account_info.info.code {
                    changes.push(StateChange {
                        change_type: StateChangeType::Code,
                        address: eth_address,
                        slot: None,
                        old_value: None,
                        new_value: Some(Bytes::from(code.bytecode.clone())),
                    });
                }
                
                // Add storage changes
                for (slot, value) in &account_info.storage {
                    changes.push(StateChange {
                        change_type: StateChangeType::Storage,
                        address: eth_address,
                        slot: Some(H256::from_slice(&slot.0)),
                        old_value: None,
                        new_value: Some(Bytes::from(value.present_value.to_be_bytes().to_vec())),
                    });
                }
            }
        }
        
        changes
    }
    
    /// Estimate gas required for a transaction
    pub async fn estimate_gas(&self, transaction: &Transaction) -> TxResult<U256> {
        // Simple gas estimation just uses provider's estimate_gas
        let tx = ethers::types::TransactionRequest::new()
            .from(transaction.params.from)
            .to(transaction.params.to.unwrap_or_default())
            .value(transaction.params.value)
            .data(transaction.params.data.clone());
            
        let gas = self.provider.estimate_gas(&tx, None).await?;
        
        // Apply gas limit multiplier for safety
        let multiplier = self.config.gas_price_config.gas_limit_multiplier;
        let gas_with_buffer = (gas.as_u64() as f64 * multiplier) as u64;
        
        Ok(U256::from(gas_with_buffer))
    }
    
    /// Get the current base fee from the latest block
    pub async fn get_current_base_fee(&self) -> TxResult<U256> {
        let block = self.provider.get_block(BlockNumber::Latest).await?
            .ok_or_else(|| TxExecutionError::Internal("Latest block not found".to_string()))?;
            
        Ok(block.base_fee_per_gas.unwrap_or_default())
    }
} 