/*
 * State Diff Tracker Module
 * 
 * This module simulates Ethereum transactions using REVM to:
 * 1. Extract per-address balance changes from transactions
 * 2. Track storage changes to detect liquidity removal and token transfers
 * 3. Cache state changes for quick access and analysis
 * 4. Identify suspicious transactions by monitoring pool states
 */

use crate::mempool_processor::types::*;
use ethers::prelude::*;
use eyre::Result;
use tracing::{debug, info, warn, error};
use std::collections::HashMap;
use std::sync::Arc;
use revm::{
    primitives::{AccountInfo as RevmAccountInfo, Address, U256 as RevmU256, TransactTo, ExecutionResult, Bytes as RevmBytes, ResultAndState},
    db::{EmptyDB, CacheDB, DatabaseCommit},
    Evm,
};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use lmdb::{Environment, Database as LmdbDatabase, DatabaseFlags, WriteFlags, Transaction as LmdbTransaction};

/// Represents a state change for an Ethereum address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    pub address: H160,
    pub balance_before: U256,
    pub balance_after: U256,
    pub balance_change: i128,
    pub eth_value: f64,
    pub storage_changes: HashMap<H256, (H256, H256)>, // slot => (before, after)
    pub timestamp: u64,
}

/// Tracks state changes from transaction simulations
#[derive(Debug)]
pub struct StateDiffTracker {
    provider: Arc<Provider<Http>>,
    state_cache: Option<StateCache>,
    recent_changes: HashMap<H256, Vec<StateChange>>, // tx_hash => state changes
}

/// Cache for state changes using LMDB
#[derive(Debug)]
pub struct StateCache {
    env: Environment,
    db: LmdbDatabase,
}

impl StateCache {
    /// Create a new state cache at the specified path
    pub fn new(path: &str) -> Result<Self> {
        // Create directory if it doesn't exist
        let path = Path::new(path);
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        
        // Initialize LMDB environment
        let env = Environment::new()
            .set_map_size(1024 * 1024 * 1024) // 1GB
            .set_max_dbs(10)
            .open(path)?;
        
        // Open database
        let db = env.create_db(Some("state_changes"), DatabaseFlags::empty())?;
        
        Ok(Self { env, db })
    }
    
    /// Store a state change in the cache
    pub fn store_state_change(&self, tx_hash: H256, state_change: &StateChange) -> Result<()> {
        let tx_hash_bytes = tx_hash.as_bytes();
        let data = serde_json::to_vec(state_change)?;
        
        let mut txn = self.env.begin_rw_txn()?;
        txn.put(self.db, &tx_hash_bytes, &data, WriteFlags::empty())?;
        txn.commit()?;
        
        Ok(())
    }
    
    /// Retrieve a state change from the cache
    pub fn get_state_change(&self, tx_hash: H256) -> Result<Option<StateChange>> {
        let tx_hash_bytes = tx_hash.as_bytes();
        let txn = self.env.begin_ro_txn()?;
        
        match txn.get(self.db, &tx_hash_bytes) {
            Ok(data) => {
                let state_change: StateChange = serde_json::from_slice(data)?;
                Ok(Some(state_change))
            },
            // Use specific LmdbError if possible, or a generic catch
            Err(lmdb::Error::NotFound) => Ok(None),
            Err(e) => Err(eyre::Report::from(e)), 
        }
    }
}

impl StateDiffTracker {
    /// Create a new state diff tracker
    pub fn new(provider: Arc<Provider<Http>>, cache_path: Option<&str>) -> Self {
        let state_cache = cache_path
            .and_then(|path| match StateCache::new(path) {
                Ok(cache) => Some(cache),
                Err(e) => {
                    warn!("Failed to initialize state cache at {}: {}", path, e);
                    None
                }
            });
        
        if state_cache.is_some() {
            info!("State cache initialized at {:?}", cache_path.unwrap_or("in-memory only (no path provided)"));
        }
        
        Self {
            provider,
            state_cache,
            recent_changes: HashMap::new(),
        }
    }
    
    /// Simulate a transaction and extract ETH balance changes (Python-compatible)
    pub async fn simulate_transaction(&mut self, tx: &TransactionView) -> Result<Option<HashMap<String, crate::tx_simulator::MempoolStateDiff>>> {
        // Skip if no destination
        if tx.to.is_none() {
            return Ok(None);
        }
        
        // Convert from ethers types to REVM types
        let from_addr = Address::from_slice(&tx.from);
        let to_addr_opt = tx.to.as_ref().map(|to_bytes| Address::from_slice(to_bytes));

        let to_revm_addr = match to_addr_opt {
            Some(addr) => TransactTo::Call(addr),
            None => return Ok(None), // Skip contract creations for now or handle as TransactTo::Create
        };
        
        let tx_hash = {
            let mut hash_array = [0u8; 32];
            if tx.hash.len() == 32 {
                hash_array.copy_from_slice(&tx.hash);
            } else {
                warn!("Invalid tx hash length: {} for tx data: {:?}", tx.hash.len(), tx);
                return Ok(None);
            }
            H256::from(hash_array)
        };
        
        // Skip cache for now - we need to return Python-compatible format
        
        debug!("Simulating transaction: {}", hex::encode(tx_hash.as_bytes()));
        
        let ethers_from_addr = H160::from_slice(&tx.from);
        let nonce = match self.provider.get_transaction_count(ethers_from_addr, None).await {
            Ok(n) => n.as_u64(),
            Err(e) => {
                error!("Failed to get nonce for {}: {}", hex::encode(&tx.from), e);
                return Ok(None);
            }
        };
        
        let mut cache_db = CacheDB::new(EmptyDB::default());
        let mut state_before = HashMap::new();

        let accounts_to_load = vec![ethers_from_addr, tx.to.as_ref().map_or(H160::zero(), |t| H160::from_slice(t))];

        for acc_h160 in accounts_to_load.iter().filter(|&&a| a != H160::zero()) {
            let acc_balance = self.provider.get_balance(*acc_h160, None).await?;
            let acc_nonce = self.provider.get_transaction_count(*acc_h160, None).await?.as_u64();
            let revm_acc_info = RevmAccountInfo {
                balance: RevmU256::from_limbs(acc_balance.0),
                nonce: acc_nonce,
                code_hash: Default::default(), 
                code: None, 
            };
            cache_db.insert_account_info(Address::from_slice(acc_h160.as_bytes()), revm_acc_info);
            state_before.insert(*acc_h160, acc_balance);
        }

        let mut evm = Evm::builder()
            .with_db(cache_db)
            .build();
        
        let from_addr = Address::from_slice(&tx.from);
        let to_addr_opt = tx.to.as_ref().map(|to_bytes| Address::from_slice(to_bytes));
        let to_revm_addr = match to_addr_opt {
            Some(addr) => TransactTo::Call(addr),
            None => TransactTo::Create,
        };

        let gas_limit = tx.gas_limit.map_or(3_000_000u64, |gl| gl.as_u64()); 
        let gas_price_u256 = tx.gas_price.unwrap_or_else(|| U256::from(20_000_000_000u64));
        let gas_price_revm = RevmU256::from_limbs(gas_price_u256.0);
        
        let tx_env = evm.tx_mut();
        tx_env.caller = from_addr;
        tx_env.transact_to = to_revm_addr;
        tx_env.data = RevmBytes::copy_from_slice(&tx.input_data.as_ref().map_or(&[][..], |d| d.as_ref()));
        tx_env.value = RevmU256::from_limbs(tx.value.0);
        tx_env.gas_limit = gas_limit;
        tx_env.gas_price = gas_price_revm;
        tx_env.nonce = Some(nonce);
        tx_env.access_list = Vec::new();
        
        let ResultAndState { result, state } = match evm.transact() {
            Ok(res) => res,
            Err(e) => {
                // These errors are common in mempool simulation since transactions
                // may depend on state changes from other pending transactions
                debug!("EVM simulation failed for tx {} (expected for mempool): {:?}", hex::encode(tx_hash.as_bytes()), e);
                return Ok(None); 
            }
        };
        
        // Extract ETH balance changes in Python-compatible format
        let mut eth_balance_changes = HashMap::new();
        
        match result {
            ExecutionResult::Success { gas_used, logs, .. } => {
                debug!("Tx {} success. GasUsed: {}. Logs: {}", hex::encode(tx_hash.as_bytes()), gas_used, logs.len());
                
                for (addr_revm, account_state) in state.iter() {
                    let eth_addr = H160::from_slice(addr_revm.as_slice());
                    let balance_after = U256(account_state.info.balance.into_limbs());
                    
                    let balance_before = state_before.get(&eth_addr).cloned().unwrap_or_default();
                    
                    // Only include addresses with balance changes
                    if balance_after != balance_before {
                        let balance_before_eth = wei_to_eth(balance_before);
                        let balance_after_eth = wei_to_eth(balance_after);
                        let change_eth = balance_after_eth - balance_before_eth;
                        
                        // Convert to checksum address like Python
                        let checksum_address = to_checksum_address(Address::from_slice(eth_addr.as_bytes()));
                        
                        let mempool_diff = crate::tx_simulator::MempoolStateDiff {
                            before: Some(balance_before_eth),
                            after: Some(balance_after_eth),
                            change: change_eth,
                        };
                        
                        debug!("ETH balance change for {}: {:.6} -> {:.6} (change: {:.6})", 
                               &checksum_address, balance_before_eth, balance_after_eth, change_eth);
                        
                        eth_balance_changes.insert(checksum_address, mempool_diff);
                    }
                }
            },
            ExecutionResult::Revert { gas_used, output } => {
                debug!("Tx {} reverted. GasUsed: {}. Output: {:?}", hex::encode(tx_hash.as_bytes()), gas_used, output);
                return Ok(None); // No state changes for reverted transactions
            },
            ExecutionResult::Halt { reason, gas_used } => {
                debug!("Tx {} halted: {:?}. GasUsed: {}", hex::encode(tx_hash.as_bytes()), reason, gas_used);
                return Ok(None); // No state changes for halted transactions
            },
        }
        
        // Commit the state changes to the database
        evm.db_mut().commit(state);
        
        Ok(if eth_balance_changes.is_empty() { None } else { Some(eth_balance_changes) })
    }
    
    /// Get recent state changes
    pub fn get_recent_changes(&self) -> &HashMap<H256, Vec<StateChange>> {
        &self.recent_changes
    }
    
    /// Get state changes for a specific transaction
    pub fn get_transaction_changes(&self, tx_hash: H256) -> Option<&Vec<StateChange>> {
        self.recent_changes.get(&tx_hash)
    }
    
    /// Clear recent state changes to free memory
    pub fn clear_recent_changes(&mut self) {
        self.recent_changes.clear();
    }
}

/// Helper to convert U256 to i128
pub fn u256_to_i128(value: U256) -> i128 {
    if value > U256::from(i128::MAX as u128) {
        i128::MAX
    } else {
        value.as_u128() as i128
    }
}

/// Helper to convert wei to ETH
pub fn wei_to_eth(wei: U256) -> f64 {
    wei.as_u128() as f64 / 1_000_000_000_000_000_000f64
}

/// Convert address to EIP-55 checksum format (matches Python's Web3.to_checksum_address)
pub fn to_checksum_address(address: Address) -> String {
    use revm_primitives::alloy_primitives::keccak256;
    
    let addr_hex = hex::encode(address.as_slice());
    let hash = keccak256(addr_hex.as_bytes());
    
    let mut result = String::with_capacity(42);
    result.push_str("0x");
    
    for (i, ch) in addr_hex.chars().enumerate() {
        if ch.is_ascii_digit() {
            result.push(ch);
        } else {
            // Check if the corresponding hash byte's high nibble is >= 8
            let hash_byte = hash[i / 2];
            let nibble = if i % 2 == 0 { hash_byte >> 4 } else { hash_byte & 0xf };
            
            if nibble >= 8 {
                result.push(ch.to_ascii_uppercase());
            } else {
                result.push(ch.to_ascii_lowercase());
            }
        }
    }
    
    result
} 