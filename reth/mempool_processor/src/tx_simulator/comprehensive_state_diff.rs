/*
 * Comprehensive State Diff Calculator
 * 
 * ALGORITHMIC DESCRIPTION:
 * This module calculates complete state changes for Ethereum transactions by:
 * 1. Extracting ETH transfers from debug_traceTransaction (internal transfers)
 * 2. Extracting ERC20 token transfers from transaction logs
 * 3. Combining both to calculate net state changes per address
 * 4. Handling special cases like WETH conversions and fee payments
 * 5. Filtering insignificant changes based on thresholds
 * 
 * The algorithm mirrors the Python ProcessedTxStateDiffCalculator to ensure
 * compatibility and validation against the existing Python implementation.
 * 
 * Key Features:
 * - Tracks both token and denomination (ETH/WETH) movements
 * - Maintains chronological order of transfers
 * - Handles WETH conversions as denomination-neutral
 * - Excludes bribes to known fee recipients
 * - Provides net changes and detailed movement tracking
 */

use crate::mempool_processor::types::*;
use ethers::prelude::*;
use ethers::utils::to_checksum;
use eyre::Result;
use tracing::debug;
use std::collections::HashMap;
use std::sync::Arc;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use serde_json::Value;

/// Individual movement record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movement {
    pub transfer_id: String, // (block_number, txn_index, log_index_or_depth) as string
    pub amount: f64,
}

/// Complete state change for an address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComprehensiveStateChange {
    pub token_net: f64,
    pub denom_net: f64, // ETH/WETH net change
    pub movements: StateChangeMovements,
}

/// Detailed movement tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeMovements {
    pub token: AddressMovements,
    pub denom: AddressMovements,
}

/// Tracks movements for an address using HashMap for compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressMovements {
    pub incoming: HashMap<String, f64>, // transfer_id -> amount
    pub outgoing: HashMap<String, f64>, // transfer_id -> amount
}

impl Default for AddressMovements {
    fn default() -> Self {
        Self {
            incoming: HashMap::new(),
            outgoing: HashMap::new(),
        }
    }
}

/// ERC20 Transfer extracted from logs
#[derive(Debug, Clone)]
pub struct Erc20Transfer {
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    pub amount: U256,
    pub log_index: u64,
    pub contract_address: Vec<u8>, // Add contract address from log
}

/// ETH Transfer extracted from traces
#[derive(Debug, Clone)]
pub struct EthTransfer {
    pub from: Vec<u8>,
    pub to: Vec<u8>,
    pub value: U256,
    pub trace_index: u64,
}

/// Comprehensive state diff calculator
pub struct ComprehensiveStateDiffCalculator {
    provider: Arc<Provider<Http>>,
    weth_address: String,
    denom_threshold: f64,
    token_threshold: f64,
    
    // Movement tracking
    token_movements: HashMap<String, AddressMovements>,
    denom_movements: HashMap<String, AddressMovements>,
}

impl ComprehensiveStateDiffCalculator {
    /// Create a new comprehensive state diff calculator
    pub fn new(provider: Arc<Provider<Http>>) -> Self {
        Self {
            provider,
            weth_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            denom_threshold: 0.0005, // Matches Python denom_state_change_threshold
            token_threshold: 0.1,    // Matches Python token_state_change_threshold
            token_movements: HashMap::new(),
            denom_movements: HashMap::new(),
        }
    }
    
    /// Convert byte address to checksummed string (matches Python format)
    fn to_checksummed_address(&self, addr_bytes: &[u8]) -> String {
        let address = Address::from_slice(addr_bytes);
        to_checksum(&address, None)
    }
    
    /// Calculate comprehensive state changes for a transaction
    pub async fn calculate_state_changes(
        &mut self,
        tx_view: &TransactionView,
        block_number: u64,
        txn_index: u64,
    ) -> Result<HashMap<String, ComprehensiveStateChange>> {
        let tx_hash = format!("0x{}", hex::encode(&tx_view.hash));
        let from_address = self.to_checksummed_address(&tx_view.from);
        debug!("Calculating comprehensive state changes for tx: {}", tx_hash);
        
        // Get ETH transfers from trace
        let eth_transfers = self.get_eth_transfers_from_trace(tx_view, block_number).await?;
        debug!("Found {} ETH transfers", eth_transfers.len());
        
        // Get ERC20 transfers from logs
        let erc20_transfers = self.get_erc20_transfers_from_logs(tx_view, block_number).await?;
        debug!("Found {} ERC20 transfers", erc20_transfers.len());
        
        // Calculate state changes
        let mut state_changes = HashMap::new();
        let mut processed_transfers = std::collections::HashSet::new(); // Track processed transfers to avoid duplicates
        
        // Process ETH transfers (denomination changes)
        for transfer in &eth_transfers {
            let from_addr = self.to_checksummed_address(&transfer.from);
            let to_addr = self.to_checksummed_address(&transfer.to);
            let amount_eth = transfer.value.as_u128() as f64 / 1e18;
            
            // Skip very small amounts (gas, dust, etc.)
            if amount_eth < 1e-15 {
                continue;
            }
            
            // CRITICAL: Match Python logic - skip ANY movement involving WETH contract
            // Python: if to_addr == self.WETH_ADDRESS or from_addr == self.WETH_ADDRESS: return
            if from_addr.to_lowercase() == self.weth_address.to_lowercase() || 
               to_addr.to_lowercase() == self.weth_address.to_lowercase() {
                debug!("Skipping ETH transfer involving WETH contract: {} -> {} (matches Python logic)", from_addr, to_addr);
                continue;
            }
            
            // Create unique transfer ID for ETH transfers - match Python format exactly  
            // Python uses: (bn, txi, f"internal_{internal_index}")
            let transfer_id = format!("internal_{}", transfer.trace_index);
            
            // Skip if already processed (shouldn't happen for ETH transfers, but safety check)
            if processed_transfers.contains(&transfer_id) {
                debug!("Skipping duplicate ETH transfer: {}", transfer_id);
                continue;
            }
            processed_transfers.insert(transfer_id.clone());
            
            // Update from address (outgoing)
            let from_change = state_changes.entry(from_addr.clone()).or_insert_with(|| ComprehensiveStateChange {
                token_net: 0.0,
                denom_net: 0.0,
                movements: StateChangeMovements {
                    token: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                    denom: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                },
            });
            from_change.denom_net -= amount_eth;
            from_change.movements.denom.outgoing.insert(transfer_id.clone(), amount_eth);
            
            // Update to address (incoming) - only if different from from_addr
            if from_addr != to_addr {
                let to_change = state_changes.entry(to_addr).or_insert_with(|| ComprehensiveStateChange {
                    token_net: 0.0,
                    denom_net: 0.0,
                    movements: StateChangeMovements {
                        token: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                        denom: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                    },
                });
                to_change.denom_net += amount_eth;
                to_change.movements.denom.incoming.insert(transfer_id, amount_eth);
            }
        }
        
        // Process ERC20 transfers (token changes)
        for transfer in &erc20_transfers {
            let from_addr = self.to_checksummed_address(&transfer.from);
            let to_addr = self.to_checksummed_address(&transfer.to);
            let contract_address = self.to_checksummed_address(&transfer.contract_address);
            
            // Check if this is a WETH transfer (treat as denomination, not token)
            let is_weth_transfer = contract_address.to_lowercase() == self.weth_address.to_lowercase();
            
            if is_weth_transfer {
                // CRITICAL: Python treats WETH transfers as DENOMINATION movements (converted to ETH units)
                // Python: mtype = "denom" if is_weth else "token"
                // Python: amount = tr["amount"] / 1e18 if is_weth else tr["amount"]
                let amount_eth = transfer.amount.as_u128() as f64 / 1e18; // Convert to ETH units
                
                debug!("WETH Transfer (as denom): {} -> {}, amount: {} ETH (raw: {})", 
                       from_addr, to_addr, amount_eth, transfer.amount);
                
                // Skip zero amounts
                if amount_eth == 0.0 {
                    continue;
                }
                
                // Create unique transfer ID for WETH transfers - match Python: (bn, txi, tr.log_index)
                let transfer_id = format!("{}", transfer.log_index);
                
                // Skip if already processed
                if processed_transfers.contains(&transfer_id) {
                    debug!("Skipping duplicate WETH transfer: {}", transfer_id);
                    continue;
                }
                processed_transfers.insert(transfer_id.clone());
                
                // CRITICAL: Apply Python's _track_movement logic for WETH
                // Python: if to_addr == self.WETH_ADDRESS or from_addr == self.WETH_ADDRESS: return
                // This means WETH contract itself doesn't get state changes
                if from_addr.to_lowercase() == self.weth_address.to_lowercase() || 
                   to_addr.to_lowercase() == self.weth_address.to_lowercase() {
                    debug!("Skipping WETH transfer involving WETH contract itself: {} -> {} (Python _track_movement logic)", from_addr, to_addr);
                    continue;
                }
                
                // Update from address (outgoing denom) - only if different from to_addr
                if from_addr != to_addr {
                    let from_change = state_changes.entry(from_addr.clone()).or_insert_with(|| ComprehensiveStateChange {
                        token_net: 0.0,
                        denom_net: 0.0,
                        movements: StateChangeMovements {
                            token: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                            denom: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                        },
                    });
                    from_change.denom_net -= amount_eth;
                    from_change.movements.denom.outgoing.insert(transfer_id.clone(), amount_eth);
                }
                
                // Update to address (incoming denom) - only if different from from_addr
                if from_addr != to_addr {
                    let to_change = state_changes.entry(to_addr).or_insert_with(|| ComprehensiveStateChange {
                        token_net: 0.0,
                        denom_net: 0.0,
                        movements: StateChangeMovements {
                            token: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                            denom: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                        },
                    });
                    to_change.denom_net += amount_eth;
                    to_change.movements.denom.incoming.insert(transfer_id, amount_eth);
                }
            } else {
                // Regular ERC20 token transfer (non-WETH)
                // Keep raw amount without decimal conversion (matches Python approach)
                let amount = transfer.amount.as_u128() as f64;
                
                debug!("ERC20 Transfer: {} -> {}, amount: {} (raw: {})", 
                       from_addr, to_addr, amount, transfer.amount);
                
                // Skip zero amounts
                if amount == 0.0 {
                    continue;
                }
                
                // CRITICAL: Skip if either address is the WETH contract (Python _track_movement logic)
                if from_addr.to_lowercase() == self.weth_address.to_lowercase() || 
                   to_addr.to_lowercase() == self.weth_address.to_lowercase() {
                    debug!("Skipping ERC20 transfer involving WETH contract: {} -> {} (Python _track_movement logic)", from_addr, to_addr);
                    continue;
                }
                
                // Create unique transfer ID for ERC20 transfers - match Python: (bn, txi, tr.log_index)  
                let transfer_id = format!("{}", transfer.log_index);
                
                // Skip if already processed
                if processed_transfers.contains(&transfer_id) {
                    debug!("Skipping duplicate ERC20 transfer: {}", transfer_id);
                    continue;
                }
                processed_transfers.insert(transfer_id.clone());
                
                // Update from address (outgoing token) - only if different from to_addr
                if from_addr != to_addr {
                    let from_change = state_changes.entry(from_addr.clone()).or_insert_with(|| ComprehensiveStateChange {
                        token_net: 0.0,
                        denom_net: 0.0,
                        movements: StateChangeMovements {
                            token: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                            denom: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                        },
                    });
                    from_change.token_net -= amount;
                    from_change.movements.token.outgoing.insert(transfer_id.clone(), amount);
                }
                
                // Update to address (incoming token) - only if different from from_addr
                if from_addr != to_addr {
                    let to_change = state_changes.entry(to_addr).or_insert_with(|| ComprehensiveStateChange {
                        token_net: 0.0,
                        denom_net: 0.0,
                        movements: StateChangeMovements {
                            token: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                            denom: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                        },
                    });
                    to_change.token_net += amount;
                    to_change.movements.token.incoming.insert(transfer_id, amount);
                }
            }
        }
        
        // Post-process to remove any remaining internal transfers (same address incoming/outgoing)
        for (_address, change) in state_changes.iter_mut() {
            self.remove_internal_transfers(change);
        }
        
        // CRITICAL: Ensure transaction sender is always included (matches Python logic)
        // Python always includes from_address even with zero net changes
        if !state_changes.contains_key(&from_address) {
            debug!("Adding transaction sender {} with zero state changes (matches Python logic)", from_address);
            state_changes.insert(from_address.clone(), ComprehensiveStateChange {
                token_net: 0.0,
                denom_net: 0.0,
                movements: StateChangeMovements {
                    token: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                    denom: AddressMovements { incoming: HashMap::new(), outgoing: HashMap::new() },
                },
            });
        }
        
        // Apply threshold filtering (matches Python implementation exactly)
        let original_count = state_changes.len();
        let filtered_changes: HashMap<String, ComprehensiveStateChange> = state_changes
            .into_iter()
            .filter(|(address, change)| {
                // Keep changes where:
                // 1. token_net exceeds token threshold (0.1)
                // 2. OR denom_net exceeds denom threshold (0.0005)  
                // 3. OR address is the transaction sender (always included)
                change.token_net.abs() > self.token_threshold || 
                change.denom_net.abs() > self.denom_threshold ||
                address == &from_address
            })
            .collect();
        
        debug!("Applied Python thresholds: token > {}, denom > {}", 
               self.token_threshold, self.denom_threshold);
        debug!("Filtered {} addresses below threshold, {} addresses remain", 
               original_count - filtered_changes.len(), 
               filtered_changes.len());
        
        Ok(filtered_changes)
    }
    
    /// Check if this WETH transfer corresponds to an ETH conversion to avoid double-counting
    fn find_matching_eth_conversion(
        &self,
        eth_transfers: &[EthTransfer],
        weth_from: &str,
        weth_to: &str,
        weth_amount_eth: f64,
        block_number: u64,
        txn_index: u64,
    ) -> bool {
        // Look for ETH transfers with similar amounts that might be the same conversion
        for eth_transfer in eth_transfers {
            let eth_from = self.to_checksummed_address(&eth_transfer.from);
            let eth_to = self.to_checksummed_address(&eth_transfer.to);
            let eth_amount = eth_transfer.value.as_u128() as f64 / 1e18;
            
            // Check if amounts are very close (within 0.1% tolerance for rounding)
            let amount_diff = (eth_amount - weth_amount_eth).abs();
            let tolerance = weth_amount_eth * 0.001; // 0.1% tolerance
            
            if amount_diff <= tolerance {
                // Check if this looks like a WETH wrap/unwrap operation
                if (eth_to.to_lowercase() == self.weth_address.to_lowercase() && weth_from.to_lowercase() == self.weth_address.to_lowercase()) ||
                   (eth_from.to_lowercase() == self.weth_address.to_lowercase() && weth_to.to_lowercase() == self.weth_address.to_lowercase()) {
                    debug!("Found matching ETH/WETH conversion: ETH {} -> {} ({:.9}) vs WETH {} -> {} ({:.9})",
                           eth_from, eth_to, eth_amount, weth_from, weth_to, weth_amount_eth);
                    return true;
                }
            }
        }
        false
    }
    
    /// Remove internal transfers where the same address has both incoming and outgoing for the same transfer ID
    fn remove_internal_transfers(&self, change: &mut ComprehensiveStateChange) {
        // Check token movements
        let mut to_remove_token = Vec::new();
        for (transfer_id, outgoing_amount) in &change.movements.token.outgoing {
            if let Some(incoming_amount) = change.movements.token.incoming.get(transfer_id) {
                if (outgoing_amount - incoming_amount).abs() < 1e-10 {
                    // Same transfer ID with same amount - this is an internal transfer
                    to_remove_token.push(transfer_id.clone());
                    debug!("Removing internal token transfer: {}", transfer_id);
                }
            }
        }
        
        for transfer_id in to_remove_token {
            if let Some(amount) = change.movements.token.outgoing.remove(&transfer_id) {
                change.movements.token.incoming.remove(&transfer_id);
                // Adjust net amounts
                change.token_net += amount; // Remove the outgoing (add back)
                change.token_net -= amount; // Remove the incoming (subtract back)
            }
        }
        
        // Check denom movements
        let mut to_remove_denom = Vec::new();
        for (transfer_id, outgoing_amount) in &change.movements.denom.outgoing {
            if let Some(incoming_amount) = change.movements.denom.incoming.get(transfer_id) {
                if (outgoing_amount - incoming_amount).abs() < 1e-15 {
                    // Same transfer ID with same amount - this is an internal transfer
                    to_remove_denom.push(transfer_id.clone());
                    debug!("Removing internal denom transfer: {}", transfer_id);
                }
            }
        }
        
        for transfer_id in to_remove_denom {
            if let Some(amount) = change.movements.denom.outgoing.remove(&transfer_id) {
                change.movements.denom.incoming.remove(&transfer_id);
                // Adjust net amounts
                change.denom_net += amount; // Remove the outgoing (add back)
                change.denom_net -= amount; // Remove the incoming (subtract back)
            }
        }
    }
    
    /// Track a movement between addresses
    fn track_movement(
        &mut self,
        movement_type: &str,
        from_addr: &str,
        to_addr: &str,
        amount: f64,
        transfer_id: String,
    ) {
        // Skip WETH conversions (denomination-neutral)
        if to_addr.to_lowercase() == self.weth_address.to_lowercase() || 
           from_addr.to_lowercase() == self.weth_address.to_lowercase() {
            return;
        }
        
        // Skip deployer transactions (depth/log_index == 0)
        // Parse transfer_id format: "block-txn-index"
        if let Some(last_part) = transfer_id.split('-').last() {
            if let Ok(index) = last_part.parse::<u64>() {
                if index == 0 {
                    return;
                }
            }
        }
        
        // Check if recipient is fee recipient before borrowing
        let is_fee_recipient = movement_type == "denom" && self.is_fee_recipient(to_addr);
        
        let movements = if movement_type == "token" {
            &mut self.token_movements
        } else {
            &mut self.denom_movements
        };
        
        // Track outgoing from sender
        let from_movements = movements.entry(from_addr.to_string()).or_default();
        from_movements.outgoing.insert(transfer_id.clone(), amount);
        
        // Track incoming to recipient (skip bribes to fee recipients for denom)
        if !is_fee_recipient {
            let to_movements = movements.entry(to_addr.to_string()).or_default();
            to_movements.incoming.insert(transfer_id.clone(), amount);
        }
    }
    
    /// Check if address is a known fee recipient
    fn is_fee_recipient(&self, _address: &str) -> bool {
        // Add known fee recipient addresses here
        // For now, we'll use a simple check
        false // TODO: Implement fee recipient detection
    }
    
    /// Calculate net changes for all addresses
    fn get_net_changes(&self, from_address: &str) -> Result<HashMap<String, ComprehensiveStateChange>> {
        let mut net_changes = HashMap::new();
        
        // Get all addresses that have movements
        let mut all_addresses = std::collections::HashSet::new();
        all_addresses.extend(self.token_movements.keys());
        all_addresses.extend(self.denom_movements.keys());
        
        for address in all_addresses {
            let token_movements = self.token_movements.get(address).cloned().unwrap_or_default();
            let denom_movements = self.denom_movements.get(address).cloned().unwrap_or_default();
            
            // Calculate net changes
            let token_in: f64 = token_movements.incoming.values().sum();
            let token_out: f64 = token_movements.outgoing.values().sum();
            let token_net = token_in - token_out;
            
            let denom_in: f64 = denom_movements.incoming.values().sum();
            let denom_out: f64 = denom_movements.outgoing.values().sum();
            let denom_net = denom_in - denom_out;
            
            // Include if significant change or is the transaction sender
            if token_net.abs() > self.token_threshold || 
               denom_net.abs() > self.denom_threshold || 
               address == from_address {
                
                let state_change = ComprehensiveStateChange {
                    token_net,
                    denom_net,
                    movements: StateChangeMovements {
                        token: token_movements,
                        denom: denom_movements,
                    },
                };
                
                net_changes.insert(address.clone(), state_change);
            }
        }
        
        Ok(net_changes)
    }
    
    /// Extract ETH transfers from debug_traceTransaction
    async fn extract_eth_transfers(&self, tx: &TransactionView) -> Result<Vec<EthTransfer>> {
        let tx_hash = H256::from_slice(&tx.hash);
        
        // Call debug_traceTransaction
        let trace_result: Value = self.provider
            .request("debug_traceTransaction", [
                serde_json::to_value(format!("0x{}", hex::encode(tx_hash.as_bytes())))?,
                serde_json::json!({
                    "tracer": "callTracer",
                    "tracerConfig": {
                        "withLog": false
                    }
                })
            ])
            .await?;
        
        let mut transfers = Vec::new();
        let mut current_index = 0; // Initialize sequential counter
        self.process_trace_for_eth_transfers(&trace_result, &mut current_index, &mut transfers)?;
        
        Ok(transfers)
    }
    
    /// Recursively process trace to extract ETH transfers
    fn process_trace_for_eth_transfers(
        &self,
        trace: &Value,
        current_index: &mut u64, // Use mutable reference for sequential indexing
        transfers: &mut Vec<EthTransfer>,
    ) -> Result<()> {
        if let Some(trace_obj) = trace.as_object() {
            // Extract value if present and non-zero
            if let Some(value_str) = trace_obj.get("value").and_then(|v| v.as_str()) {
                if let Ok(value) = U256::from_str_radix(value_str.trim_start_matches("0x"), 16) {
                    if value > U256::zero() {
                        if let (Some(from), Some(to)) = (
                            trace_obj.get("from").and_then(|v| v.as_str()),
                            trace_obj.get("to").and_then(|v| v.as_str())
                        ) {
                            // Convert hex addresses to byte vectors
                            let from_bytes = hex::decode(from.trim_start_matches("0x"))?;
                            let to_bytes = hex::decode(to.trim_start_matches("0x"))?;
                            
                            transfers.push(EthTransfer {
                                from: from_bytes,
                                to: to_bytes,
                                value: value,
                                trace_index: *current_index, // Use sequential index for uniqueness
                            });
                            
                            *current_index += 1; // Increment for next transfer
                        }
                    }
                }
            }
            
            // Process subcalls
            if let Some(calls) = trace_obj.get("calls").and_then(|v| v.as_array()) {
                for call in calls {
                    self.process_trace_for_eth_transfers(call, current_index, transfers)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract ERC20 transfers from transaction logs
    async fn extract_erc20_transfers(&self, tx: &TransactionView, block_number: u64) -> Result<Vec<Erc20Transfer>> {
        let tx_hash = H256::from_slice(&tx.hash);
        
        // Get transaction receipt to access logs
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?;
        
        let mut transfers = Vec::new();
        
        if let Some(receipt) = receipt {
            for (log_index, log) in receipt.logs.iter().enumerate() {
                // Check if this is an ERC20 Transfer event
                if log.topics.len() >= 3 && 
                   log.topics[0] == H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")? {
                    
                    // Extract addresses from topics (skip first 12 bytes of padding)
                    let from_bytes = log.topics[1].as_bytes()[12..].to_vec();
                    let to_bytes = log.topics[2].as_bytes()[12..].to_vec();
                    
                    // Decode amount from log data
                    if log.data.len() >= 32 {
                        let amount = U256::from_big_endian(&log.data[..32]);
                        
                        transfers.push(Erc20Transfer {
                            from: from_bytes,
                            to: to_bytes,
                            amount: amount,
                            log_index: log_index as u64,
                            contract_address: log.address.as_bytes().to_vec(),
                        });
                    }
                }
            }
        }
        
        Ok(transfers)
    }
    
    /// Get ETH transfers from debug trace
    async fn get_eth_transfers_from_trace(&self, tx_view: &TransactionView, _block_number: u64) -> Result<Vec<EthTransfer>> {
        self.extract_eth_transfers(tx_view).await
    }
    
    /// Get ERC20 transfers from transaction logs
    async fn get_erc20_transfers_from_logs(&self, tx_view: &TransactionView, block_number: u64) -> Result<Vec<Erc20Transfer>> {
        self.extract_erc20_transfers(tx_view, block_number).await
    }
} 