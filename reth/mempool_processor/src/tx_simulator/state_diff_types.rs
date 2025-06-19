/// Core State Change Types and Tracking
/// 
/// This module defines the fundamental data structures for tracking
/// state changes in Ethereum transactions. It provides:
///
/// Core Types:
/// - StateChange: Represents balance and storage changes for an address
/// - StateDiffTracker: Orchestrates state diff extraction from transactions
///
/// The types defined here are used by both simulation approaches:
/// - REVM-based simulator for comprehensive analysis
/// - debug_traceCall simulator for fast production use
///
/// These structures capture:
/// - ETH balance changes (including gas fees)
/// - ERC20 token balance changes via storage slots
/// - General storage modifications
/// - Transaction metadata for analysis

use crate::mempool_fetcher::types::*;
use ethers::prelude::*;
use eyre::Result;
use tracing::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use revm::primitives::Address;
use revm_primitives;
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use std::path::Path;

/// State diff for mempool transaction tracking
#[derive(Debug, Clone)]
pub struct MempoolStateDiff {
    pub before: Option<f64>,  // ETH balance before (None if unknown)
    pub after: Option<f64>,   // ETH balance after (None if unknown)
    pub change: f64,          // Net change in ETH
    // Extended fields for scam detection
    pub eth_changes: HashMap<ethers::types::Address, revm_primitives::I256>,  // Address -> ETH change
    pub token_changes: HashMap<(ethers::types::Address, ethers::types::Address), revm_primitives::I256>,  // (holder, token) -> change
}
use std::fs;
// Commented out - LMDB cache not actively used
// use lmdb::{Environment, Database as LmdbDatabase, DatabaseFlags, WriteFlags, Transaction as LmdbTransaction};

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

/// Represents balance information from state diff
#[derive(Debug, Clone)]
struct BalanceInfo {
    before: Option<f64>,
    after: Option<f64>,
    change: f64,
}

/// Represents an ETH transfer (including internal transfers)
#[derive(Debug, Clone)]
struct EthTransfer {
    from: String,
    to: String,
    amount_eth: f64,
}

/// Tracks state changes from transaction simulations
#[derive(Debug)]
pub struct StateDiffTracker {
    provider: Arc<Provider<Http>>,
    // state_cache: Option<StateCache>, // Commented out - LMDB cache not actively used
    recent_changes: HashMap<H256, Vec<StateChange>>, // tx_hash => state changes
}

// Commented out - LMDB cache not actively used
// /// Cache for state changes using LMDB
// #[derive(Debug)]
// pub struct StateCache {
//     env: Environment,
//     db: LmdbDatabase,
// }

// Commented out - LMDB cache not actively used
// impl StateCache {
//     /// Create a new state cache at the specified path
//     pub fn new(path: &str) -> Result<Self> {
//         // Create directory if it doesn't exist
//         let path = Path::new(path);
//         if !path.exists() {
//             fs::create_dir_all(path)?;
//         }
//         
//         // Initialize LMDB environment
//         let env = Environment::new()
//             .set_map_size(1024 * 1024 * 1024) // 1GB
//             .set_max_dbs(10)
//             .open(path)?;
//         
//         // Open database
//         let db = env.create_db(Some("state_changes"), DatabaseFlags::empty())?;
//         
//         Ok(Self { env, db })
//     }
//     
//     /// Store a state change in the cache
//     pub fn store_state_change(&self, tx_hash: H256, state_change: &StateChange) -> Result<()> {
//         let tx_hash_bytes = tx_hash.as_bytes();
//         let data = serde_json::to_vec(state_change)?;
//         
//         let mut txn = self.env.begin_rw_txn()?;
//         txn.put(self.db, &tx_hash_bytes, &data, WriteFlags::empty())?;
//         txn.commit()?;
//         
//         Ok(())
//     }
//     
//     /// Retrieve a state change from the cache
//     pub fn get_state_change(&self, tx_hash: H256) -> Result<Option<StateChange>> {
//         let tx_hash_bytes = tx_hash.as_bytes();
//         let txn = self.env.begin_ro_txn()?;
//         
//         match txn.get(self.db, &tx_hash_bytes) {
//             Ok(data) => {
//                 let state_change: StateChange = serde_json::from_slice(data)?;
//                 Ok(Some(state_change))
//             },
//             // Use specific LmdbError if possible, or a generic catch
//             Err(lmdb::Error::NotFound) => Ok(None),
//             Err(e) => Err(eyre::Report::from(e)), 
//         }
//     }
// }

impl StateDiffTracker {
    /// Create a new state diff tracker
    pub fn new(provider: Arc<Provider<Http>>, _cache_path: Option<&str>) -> Self {
        // LMDB cache removed - not actively used
        // let state_cache = cache_path
        //     .and_then(|path| match StateCache::new(path) {
        //         Ok(cache) => Some(cache),
        //         Err(e) => {
        //             warn!("Failed to initialize state cache at {}: {}", path, e);
        //             None
        //         }
        //     });
        
        // if state_cache.is_some() {
        //     info!("State cache initialized at {:?}", cache_path.unwrap_or("in-memory only (no path provided)"));
        // }
        
        Self {
            provider,
            // state_cache,
            recent_changes: HashMap::new(),
        }
    }
    
    /// Simulate a transaction and extract ETH balance changes (Python-compatible)
    pub async fn simulate_transaction(&mut self, tx: &TransactionView) -> Result<Option<HashMap<String, crate::tx_simulator::MempoolStateDiff>>> {
        // Validate transaction hash
        if tx.hash.len() != 32 {
            return Err(eyre::eyre!(
                "Invalid transaction hash length: expected 32 bytes, got {} bytes", 
                tx.hash.len()
            ));
        }
        
        let mut hash_array = [0u8; 32];
        hash_array.copy_from_slice(&tx.hash);
        let tx_hash = H256::from(hash_array);
        
        debug!("Simulating transaction with stateDiff: {}", hex::encode(tx_hash.as_bytes()));
        
        // Use trace_call with stateDiff to get net balance changes (like Python implementation)
        match self.trace_call_with_state_diff(tx).await {
            Ok(Some(state_diff)) => {
                let mut eth_balance_changes = HashMap::new();
                
                // Process each address in the state diff
                for (address, changes) in state_diff {
                    if let Some(balance_change) = changes.get("balance") {
                        if let Some(balance_info) = self.extract_balance_change(balance_change) {
                            // Only include significant changes
                            if balance_info.change.abs() >= 0.000001 {
                                let checksum_address = to_checksum_address(Address::from_slice(&hex::decode(address.trim_start_matches("0x"))?));
                                
                                debug!("ETH balance change: {} = {:.6} ETH", checksum_address, balance_info.change);
                                
                                eth_balance_changes.insert(checksum_address, crate::tx_simulator::MempoolStateDiff {
                                    before: balance_info.before,
                                    after: balance_info.after,
                                    change: balance_info.change,
                                    eth_changes: HashMap::new(),  // Will be populated separately
                                    token_changes: HashMap::new(),
                                });
                            }
                        }
                    }
                }
                
                if eth_balance_changes.is_empty() {
                    debug!("No significant ETH balance changes found");
                    return Ok(None);
                }
                
                Ok(Some(eth_balance_changes))
            }
            Ok(None) => {
                debug!("No state diff available for transaction");
                Ok(None)
            }
            Err(e) => {
                warn!("Failed to get state diff for transaction {}: {}", hex::encode(tx_hash.as_bytes()), e);
                Ok(None)
            }
        }
    }
    
    /// Use debug_traceTransaction to get all ETH transfers including internal ones
    async fn trace_call_with_state_diff(&self, tx: &TransactionView) -> Result<Option<HashMap<String, HashMap<String, Value>>>> {
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
        
        // First try to get internal transfers using debug_traceTransaction
        match self.trace_internal_eth_transfers(tx_hash).await {
            Ok(Some(transfers)) => {
                if transfers.is_empty() {
                    debug!("No internal ETH transfers found");
                    return self.fallback_to_transaction_value(tx).await;
                }
                
                // Aggregate transfers by address to get net changes
                let mut net_changes: HashMap<String, i128> = HashMap::new();
                
                for transfer in &transfers {
                    debug!("Internal ETH transfer: {} -> {} = {:.6} ETH", 
                           transfer.from, transfer.to, transfer.amount_eth);
                    
                    // Sender loses ETH
                    let sender_entry = net_changes.entry(transfer.from.clone()).or_insert(0);
                    *sender_entry -= (transfer.amount_eth * 1e18) as i128;
                    
                    // Recipient gains ETH
                    let recipient_entry = net_changes.entry(transfer.to.clone()).or_insert(0);
                    *recipient_entry += (transfer.amount_eth * 1e18) as i128;
                }
                
                // Convert net changes to state diff format
                let mut result = HashMap::new();
                for (address, net_change_wei) in net_changes {
                    if net_change_wei.abs() >= 1_000_000_000_000_000 { // 0.001 ETH threshold
                        let change_eth = net_change_wei as f64 / 1e18;
                        
                        let balance_change = if net_change_wei >= 0 {
                            json!({
                                "*": {
                                    "from": "0x0",
                                    "to": format!("0x{:x}", net_change_wei as u128)
                                }
                            })
                        } else {
                            json!({
                                "*": {
                                    "from": format!("0x{:x}", (-net_change_wei) as u128),
                                    "to": "0x0"
                                }
                            })
                        };
                        
                        let mut changes = HashMap::new();
                        changes.insert("balance".to_string(), balance_change);
                        debug!("Net ETH change: {} = {:.6} ETH", address, change_eth);
                        result.insert(address, changes);
                    }
                }
                
                if result.is_empty() {
                    debug!("All net changes below threshold");
                    return Ok(None);
                }
                
                Ok(Some(result))
            }
            Ok(None) => {
                debug!("No trace data available, falling back to transaction value");
                self.fallback_to_transaction_value(tx).await
            }
            Err(e) => {
                warn!("Trace failed: {}, falling back to transaction value", e);
                self.fallback_to_transaction_value(tx).await
            }
        }
    }
    
    /// Trace internal ETH transfers using debug_traceTransaction
    async fn trace_internal_eth_transfers(&self, tx_hash: H256) -> Result<Option<Vec<EthTransfer>>> {
        use ethers::types::GethDebugTracingOptions;
        
        let trace_options = GethDebugTracingOptions {
            disable_storage: Some(true),
            disable_stack: Some(true),
            enable_memory: Some(false),
            enable_return_data: Some(false),
            tracer: Some(ethers::types::GethDebugTracerType::JsTracer("callTracer".to_string())),
            ..Default::default()
        };
        
        match self.provider.debug_trace_transaction(tx_hash, trace_options).await {
            Ok(trace) => {
                let mut transfers = Vec::new();
                let trace_json = serde_json::to_value(&trace)?;
                self.extract_eth_transfers_from_trace(&trace_json, &mut transfers)?;
                Ok(Some(transfers))
            }
            Err(e) => {
                debug!("debug_traceTransaction failed: {}", e);
                Ok(None)
            }
        }
    }
    
    /// Extract ETH transfers from trace data recursively
    fn extract_eth_transfers_from_trace(&self, trace: &Value, transfers: &mut Vec<EthTransfer>) -> Result<()> {
        if let Some(trace_obj) = trace.as_object() {
            // Check if this call has value transfer
            if let (Some(from), Some(to), Some(value_str)) = (
                trace_obj.get("from").and_then(|v| v.as_str()),
                trace_obj.get("to").and_then(|v| v.as_str()),
                trace_obj.get("value").and_then(|v| v.as_str())
            ) {
                // Parse value (hex string)
                if let Ok(value_u256) = U256::from_str_radix(value_str.trim_start_matches("0x"), 16) {
                    if value_u256 > U256::zero() {
                        let amount_eth = wei_to_eth(value_u256);
                        let from_checksum = to_checksum_address(Address::from_slice(&hex::decode(from.trim_start_matches("0x"))?));
                        let to_checksum = to_checksum_address(Address::from_slice(&hex::decode(to.trim_start_matches("0x"))?));
                        
                        transfers.push(EthTransfer {
                            from: from_checksum,
                            to: to_checksum,
                            amount_eth,
                        });
                    }
                }
            }
            
            // Recursively process calls
            if let Some(calls) = trace_obj.get("calls").and_then(|v| v.as_array()) {
                for call in calls {
                    self.extract_eth_transfers_from_trace(call, transfers)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Fallback to transaction value field when tracing fails
    async fn fallback_to_transaction_value(&self, tx: &TransactionView) -> Result<Option<HashMap<String, HashMap<String, Value>>>> {
        if tx.value > U256::zero() {
            let mut result = HashMap::new();
            let transfer_amount_wei = tx.value;
            
            // Sender loses ETH
            let sender_addr = to_checksum_address(Address::from_slice(&tx.from));
            let sender_change = json!({
                "*": {
                    "from": format!("0x{:x}", transfer_amount_wei),
                    "to": "0x0"
                }
            });
            let mut sender_changes = HashMap::new();
            sender_changes.insert("balance".to_string(), sender_change);
            result.insert(sender_addr, sender_changes);
            
            // Recipient gains ETH
            if let Some(to_bytes) = &tx.to {
                let recipient_addr = to_checksum_address(Address::from_slice(to_bytes));
                let recipient_change = json!({
                    "*": {
                        "from": "0x0",
                        "to": format!("0x{:x}", transfer_amount_wei)
                    }
                });
                let mut recipient_changes = HashMap::new();
                recipient_changes.insert("balance".to_string(), recipient_change);
                result.insert(recipient_addr, recipient_changes);
            }
            
            debug!("Fallback: ETH transfer of {} wei", transfer_amount_wei);
            Ok(Some(result))
        } else {
            debug!("No ETH transfer in transaction value field");
            Ok(None)
        }
    }
    
    /// Extract ETH balance change from state diff structure (like Python implementation)
    fn extract_balance_change(&self, balance_data: &Value) -> Option<BalanceInfo> {
        // Handle different possible balance_data structures (matching Python logic)
        
        // Format 1: {'*': {'from': '0x...', 'to': '0x...'}}
        if let Some(star_obj) = balance_data.get("*").and_then(|v| v.as_object()) {
            if let (Some(from_str), Some(to_str)) = (
                star_obj.get("from").and_then(|v| v.as_str()),
                star_obj.get("to").and_then(|v| v.as_str())
            ) {
                if let (Ok(from_val), Ok(to_val)) = (
                    U256::from_str_radix(from_str.trim_start_matches("0x"), 16),
                    U256::from_str_radix(to_str.trim_start_matches("0x"), 16)
                ) {
                    let change = if to_val >= from_val {
                        wei_to_eth(to_val - from_val)
                    } else {
                        -wei_to_eth(from_val - to_val)
                    };
                    
                    return Some(BalanceInfo {
                        before: Some(wei_to_eth(from_val)),
                        after: Some(wei_to_eth(to_val)),
                        change,
                    });
                }
            }
        }
        
        // Format 2: {'+': '0x...'} (incremental change)
        if let Some(plus_str) = balance_data.get("+").and_then(|v| v.as_str()) {
            if let Ok(change_val) = U256::from_str_radix(plus_str.trim_start_matches("0x"), 16) {
                if change_val > U256::zero() {
                    return Some(BalanceInfo {
                        before: None,
                        after: None,
                        change: wei_to_eth(change_val),
                    });
                }
            }
        }
        
        // Format 3: {'-': '0x...'} (decremental change)
        if let Some(minus_str) = balance_data.get("-").and_then(|v| v.as_str()) {
            if let Ok(change_val) = U256::from_str_radix(minus_str.trim_start_matches("0x"), 16) {
                if change_val > U256::zero() {
                    return Some(BalanceInfo {
                        before: None,
                        after: None,
                        change: -wei_to_eth(change_val),
                    });
                }
            }
        }
        
        // Format 4: {'from': '0x...', 'to': '0x...'} (direct values)
        if let Some(balance_obj) = balance_data.as_object() {
            if let (Some(from_str), Some(to_str)) = (
                balance_obj.get("from").and_then(|v| v.as_str()),
                balance_obj.get("to").and_then(|v| v.as_str())
            ) {
                if let (Ok(from_val), Ok(to_val)) = (
                    U256::from_str_radix(from_str.trim_start_matches("0x"), 16),
                    U256::from_str_radix(to_str.trim_start_matches("0x"), 16)
                ) {
                    let change = if to_val >= from_val {
                        wei_to_eth(to_val - from_val)
                    } else {
                        -wei_to_eth(from_val - to_val)
                    };
                    
                    return Some(BalanceInfo {
                        before: Some(wei_to_eth(from_val)),
                        after: Some(wei_to_eth(to_val)),
                        change,
                    });
                }
            }
        }
        
        debug!("Unknown balance data format: {:?}", balance_data);
        None
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