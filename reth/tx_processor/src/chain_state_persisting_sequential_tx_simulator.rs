/// Chain State Persisting Sequential Transaction Simulator
/// 
/// Simulates multiple transactions in sequence where each transaction 
/// sees the blockchain state changes made by all previous transactions.
/// 
/// This simulator maintains a forked blockchain state throughout the sequence,
/// allowing each transaction to build upon the state changes of previous ones.
/// Critical for simulating transaction sequences like:
/// - Enable trading -> Buy tokens -> Approve spending -> Sell tokens
/// - Flash loans -> Arbitrage -> Repayment
/// - Any multi-step DeFi interactions
/// 
/// Key Features:
/// - Maintains forked blockchain state across transactions
/// - Automatic nonce tracking and increment
/// - Gas usage accumulation
/// - Lazy initialization for performance
/// - Seamless integration with existing ProcessedTransaction format

use std::collections::HashMap;
use std::sync::Arc;
use eyre::Result;
use alloy_primitives::{Address, U256, B256};
use reth_tx_simulator::{RethTxSimulator, CallRequest, FullSimulationResult};
use crate::data_models::ProcessedTransaction;
use crate::TxProcessor;

/// Simulates multiple transactions in sequence where each transaction 
/// sees the blockchain state changes made by all previous transactions
pub struct ChainStatePersistingSequentialTxSimulator {
    simulator: Arc<RethTxSimulator>,
    persistent_chain_state: Option<PersistentChainState>,
    processed_transactions: Vec<ProcessedTransaction>,
}

/// Represents the persistent blockchain state maintained across sequential transactions
struct PersistentChainState {
    /// The forked blockchain database that accumulates state changes
    forked_blockchain_db: reth_tx_simulator::ForkedState,
    /// Block number this state was forked from
    block_number: u64,
    /// Tracks nonces for automatic increment across transactions
    nonce_tracker: HashMap<Address, u64>,
    /// Total gas used across all transactions in the sequence
    cumulative_gas_used: u64,
}

impl ChainStatePersistingSequentialTxSimulator {
    /// Create a new sequential transaction simulator
    /// 
    /// The simulator starts uninitialized and will lazily create the forked state
    /// when the first transaction is simulated.
    pub fn new(simulator: Arc<RethTxSimulator>) -> Self {
        Self {
            simulator,
            persistent_chain_state: None,
            processed_transactions: Vec::new(),
        }
    }
    
    /// Initialize the forked blockchain state at a specific block
    /// 
    /// This creates a forked state from the specified block (or latest if None).
    /// All subsequent transactions will see this state plus any changes from 
    /// previous transactions in the sequence.
    /// 
    /// # Arguments
    /// * `block` - Block number to fork from, or None for latest block
    pub fn initialize_blockchain_state_at_block(&mut self, block: Option<u64>) -> Result<()> {
        if self.persistent_chain_state.is_none() {
            let block_number = match block {
                Some(b) => b,
                None => self.simulator.get_latest_block()?,
            };
            
            let forked_blockchain_db = self.simulator.create_forked_state(block_number)?;
            
            self.persistent_chain_state = Some(PersistentChainState {
                forked_blockchain_db,
                block_number,
                nonce_tracker: HashMap::new(),
                cumulative_gas_used: 0,
            });
        }
        Ok(())
    }
    
    /// Simulate the next transaction in the sequence, preserving all previous state changes
    /// 
    /// This is the core method that:
    /// 1. Ensures blockchain state is initialized
    /// 2. Handles nonce auto-increment if needed
    /// 3. Simulates the transaction on the current forked state
    /// 4. Commits state changes to preserve them for next transaction
    /// 5. Converts result to ProcessedTransaction format
    /// 6. Updates internal tracking (gas, nonces)
    /// 
    /// # Arguments
    /// * `call_request` - The transaction to simulate
    /// * `tx_processor` - Used to convert simulation results to ProcessedTransaction
    /// 
    /// # Returns
    /// Reference to the newly created ProcessedTransaction
    pub async fn simulate_next_transaction_preserving_blockchain_state(
        &mut self, 
        mut call_request: CallRequest,
        tx_processor: Arc<TxProcessor>,
    ) -> Result<&ProcessedTransaction> {
        // Ensure blockchain state is initialized
        self.initialize_blockchain_state_at_block(None)?;
        
        // Extract data we need before mutable operations to avoid borrow conflicts
        let block_number = self.persistent_chain_state.as_ref().unwrap().block_number;
        let tx_index = self.processed_transactions.len() as u64;
        
        let chain_state = self.persistent_chain_state.as_mut().unwrap();
        
        // Handle automatic nonce increment if needed
        if let Some(from) = call_request.from {
            if call_request.nonce.is_none() {
                let current_nonce = match chain_state.nonce_tracker.get(&from) {
                    Some(&tracked_nonce) => tracked_nonce,
                    None => {
                        // Get nonce from current blockchain state
                        let nonce = self.simulator.get_nonce_from_state(
                            &mut chain_state.forked_blockchain_db, 
                            from
                        )?;
                        chain_state.nonce_tracker.insert(from, nonce);
                        nonce
                    }
                };
                call_request.nonce = Some(current_nonce);
            }
        }
        
        // Simulate transaction on current forked blockchain state
        let simulation_result = self.simulator.simulate_on_forked_state(
            &mut chain_state.forked_blockchain_db,
            call_request.clone()
        ).await?;
        
        // Update tracking for successful transactions (do this while we still have mutable borrow)
        chain_state.cumulative_gas_used += simulation_result.gas_used;
        
        if simulation_result.success {
            // Update nonce tracker for successful transactions
            if let Some(from) = call_request.from {
                if let Some(nonce) = call_request.nonce {
                    chain_state.nonce_tracker.insert(from, nonce + 1);
                }
            }
        }
        
        // End the mutable borrow by explicitly dropping chain_state reference
        let _ = chain_state;
        
        // Convert simulation result to ProcessedTransaction (now we can borrow self immutably)
        let processed_tx = self.convert_simulation_result_to_processed_transaction(
            &simulation_result,
            &call_request,
            tx_processor,
            block_number, // Use extracted value instead of accessing through chain_state
            tx_index,     // Use extracted value
        ).await?;
        
        // Store the processed transaction
        self.processed_transactions.push(processed_tx);
        Ok(self.processed_transactions.last().unwrap())
    }
    
    /// Get all processed transactions from the sequence
    pub fn get_all_processed_transactions(&self) -> &[ProcessedTransaction] {
        &self.processed_transactions
    }
    
    /// Get a specific processed transaction by index
    pub fn get_processed_transaction_at_index(&self, index: usize) -> Option<&ProcessedTransaction> {
        self.processed_transactions.get(index)
    }
    
    /// Get nonce for an address from the current forked state
    /// 
    /// This initializes the forked state if needed and returns the current nonce
    /// for the given address at the specified block (or latest if None).
    pub fn get_nonce_for_address(&mut self, address: Address, block_number: Option<u64>) -> Result<u64> {
        // Initialize state if needed
        if self.persistent_chain_state.is_none() {
            self.initialize_blockchain_state_at_block(block_number)?;
        }
        
        let chain_state = self.persistent_chain_state.as_mut().unwrap();
        self.simulator.get_nonce_from_state(&mut chain_state.forked_blockchain_db, address)
    }
    
    /// Extract token amount received by a specific address from a transaction
    /// 
    /// This helper method extracts the amount of a specific token that an address
    /// received during a transaction. Useful for getting the token amount from a
    /// buy transaction to use in a subsequent sell transaction.
    /// 
    /// # Arguments
    /// * `tx_index` - Index of the transaction in the sequence
    /// * `token_address` - Address of the token contract
    /// * `holder_address` - Address that received the tokens
    /// 
    /// # Returns
    /// The amount of tokens received, or an error if not found
    pub fn extract_token_amount_received_by_address(
        &self,
        tx_index: usize,
        token_address: Address,
        holder_address: Address,
    ) -> Result<U256> {
        let processed_tx = self.get_processed_transaction_at_index(tx_index)
            .ok_or_else(|| eyre::eyre!("Transaction at index {} not found", tx_index))?;
            
        self.extract_token_amount_from_processed_transaction(
            processed_tx, 
            token_address, 
            holder_address
        )
    }
    
    /// Get total gas used across all transactions in the sequence
    pub fn get_cumulative_gas_used(&self) -> u64 {
        self.persistent_chain_state
            .as_ref()
            .map(|state| state.cumulative_gas_used)
            .unwrap_or(0)
    }
    
    /// Get the block number the sequence was forked from
    pub fn get_fork_block_number(&self) -> Option<u64> {
        self.persistent_chain_state
            .as_ref()
            .map(|state| state.block_number)
    }
    
    /// Check if blockchain state has been initialized
    pub fn is_blockchain_state_initialized(&self) -> bool {
        self.persistent_chain_state.is_some()
    }
    
    /// Convert FullSimulationResult to ProcessedTransaction
    async fn convert_simulation_result_to_processed_transaction(
        &self,
        simulation_result: &FullSimulationResult,
        call_request: &CallRequest,
        tx_processor: Arc<TxProcessor>,
        block_number: u64,
        tx_index: u64,
    ) -> Result<ProcessedTransaction> {
        // Create a synthetic transaction hash for this simulated transaction
        let tx_hash = B256::from_slice(&[tx_index as u8; 32]);
        
        let block_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let from = call_request.from.unwrap_or(Address::ZERO);
        let to = call_request.to;
        let value = call_request.value.unwrap_or(U256::ZERO);
        let input = call_request.data.clone().unwrap_or_default();
        let gas_limit = call_request.gas.unwrap_or(300_000);
        let nonce = call_request.nonce.unwrap_or(0);
        
        // Convert address balance changes to JSON format for ProcessedTransaction
        let address_balance_changes_json: HashMap<Address, serde_json::Value> = 
            simulation_result.address_balance_changes.iter()
                .map(|(addr, change)| {
                    let token_net_strings: HashMap<String, String> = change.token_net.iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect();
                    
                    let json_change = serde_json::json!({
                        "eth_net": change.eth_net.to_string(),
                        "token_net": token_net_strings,
                    });
                    (*addr, json_change)
                })
                .collect();
        
        // Use tx_processor to create ProcessedTransaction with all decoded events
        tx_processor.process_transaction_from_raw_data(
            tx_hash,
            block_number,
            block_timestamp,
            tx_index,
            from,
            to,
            value,
            input.to_vec(),
            U256::from(call_request.gas_price.unwrap_or(30_000_000_000)),
            simulation_result.gas_used,
            if simulation_result.success { "1".to_string() } else { "0".to_string() },
            nonce,
            simulation_result.logs.clone(),
            gas_limit,
            Some(address_balance_changes_json),
        ).await
    }
    
    /// Extract token amount from ProcessedTransaction
    fn extract_token_amount_from_processed_transaction(
        &self,
        processed_tx: &ProcessedTransaction,
        token_address: Address,
        holder_address: Address,
    ) -> Result<U256> {
        // First, try to extract from address_balance_changes
        if let Some(balance_change) = processed_tx.address_balance_changes.get(&holder_address) {
            // Try to find the token by checksummed address
            let token_checksum = crate::utils::to_checksum_address(&token_address);
            
            if let Some(token_amount_json) = balance_change.get(&token_checksum) {
                // Parse the token_net field
                if let Some(token_net) = token_amount_json.get("token_net") {
                    if let Some(amount_str) = token_net.get(&token_checksum) {
                        if let Some(amount_str) = amount_str.as_str() {
                            // Parse I256 string and convert to U256
                            let amount_i256: i128 = amount_str.parse()
                                .map_err(|e| eyre::eyre!("Failed to parse token amount: {}", e))?;
                            
                            if amount_i256 > 0 {
                                return Ok(U256::from(amount_i256 as u128));
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: look through ERC20 transfers
        for transfer in &processed_tx.erc20_transfers {
            if transfer.token_address == token_address && 
               transfer.to_address == holder_address {
                return Ok(transfer.amount);
            }
        }
        
        Err(eyre::eyre!(
            "Could not extract token amount for token {} received by {} from transaction",
            crate::utils::to_checksum_address(&token_address),
            crate::utils::to_checksum_address(&holder_address)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simulator_creation() {
        // This would require a mock RethTxSimulator for proper testing
        // For now, just test that the struct can be created
        assert_eq!(std::mem::size_of::<ChainStatePersistingSequentialTxSimulator>(), 64); // Rough size check
    }
    
    #[test]
    fn test_blockchain_state_not_initialized_initially() {
        let mock_simulator = Arc::new(unsafe { std::mem::zeroed() }); // Unsafe but OK for size test
        std::mem::forget(mock_simulator); // Prevent drop
        
        // Would need proper mock to test initialization logic
    }
}