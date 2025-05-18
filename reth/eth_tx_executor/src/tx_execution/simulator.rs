/*
* Transaction Simulator
*
* Responsible for:
* - Simulating transaction execution
* - Estimating gas usage with buffers
* - Validating transaction parameters
* - Providing execution previews
*
* Algorithm:
* 1. Create transaction request from parameters
* 2. Call node's eth_call method to simulate
* 3. Estimate gas with safety buffer
* 4. Validate parameters (gas limit, etc.)
* 5. Report simulation result with state changes
*/

use crate::tx_execution::{
    types::{Transaction, SimulationResult, StateChange, StateChangeType},
    config::TxExecutionConfig,
    error::{TxError, TxResult},
};
use ethers::{
    providers::{Middleware},
    types::{Address, U256, Bytes, H256, TransactionRequest},
    core::types::transaction::eip2718::TypedTransaction,
};
use std::sync::Arc;

/// Transaction Simulator for pre-execution validation and gas estimation
pub struct TxSimulator<M> 
where
    M: Middleware
{
    /// Provider for blockchain interaction
    provider: Arc<M>,
    /// Configuration for transaction execution
    config: TxExecutionConfig,
    /// Gas safety buffer percentage (e.g., 20 = add 20% to estimated gas)
    gas_buffer: u64,
}

// Standard implementation for any Middleware
impl<M> TxSimulator<M> 
where
    M: Middleware
{
    /// Create a new transaction simulator
    pub fn new(provider: Arc<M>, config: TxExecutionConfig) -> Self {
        Self {
            provider,
            config,
            gas_buffer: 20, // Default 20% gas buffer
        }
    }
    
    /// Set gas buffer percentage
    pub fn with_gas_buffer(mut self, buffer_percentage: u64) -> Self {
        self.gas_buffer = buffer_percentage;
        self
    }
    
    /// Simulate transaction execution
    pub async fn simulate(&self, tx: &Transaction) -> TxResult<SimulationResult> {
        // Create a transaction request from our transaction
        let request = self.create_tx_request(tx);
        let typed_tx: TypedTransaction = request.into();
        
        // Call the node's eth_call method to simulate the transaction
        let result = self.provider
            .call(&typed_tx, None)
            .await
            .map_err(|e| TxError::SimulationFailed(format!("Simulation failed: {}", e)))?;
        
        // Estimate gas
        let gas_used = self.estimate_gas(tx).await?;
        
        // For a real implementation, we would track state changes
        // This would require more complex interaction with the node or a local EVM
        let state_changes = self.simulate_state_changes(tx).await?;
        
        // Create simulation result
        Ok(SimulationResult {
            success: true, // Assume success if we got here (no revert)
            gas_used,
            gas_limit: tx.params.gas_limit,
            result: Some(result),
            error: None,
            state_changes,
        })
    }
    
    /// Estimate gas for transaction with safety buffer
    pub async fn estimate_gas(&self, tx: &Transaction) -> TxResult<U256> {
        // Create transaction request
        let request = self.create_tx_request(tx);
        let typed_tx: TypedTransaction = request.into();
        
        // Call the node's eth_estimateGas method
        let estimate = self.provider
            .estimate_gas(&typed_tx, None)
            .await
            .map_err(|e| TxError::GasEstimationFailed(format!("Gas estimation failed: {}", e)))?;
        
        // Add safety buffer
        let buffer = estimate * U256::from(self.gas_buffer) / U256::from(100);
        let total_estimate = estimate + buffer;
        
        // Cap at gas limit if necessary
        if total_estimate > tx.params.gas_limit {
            return Ok(tx.params.gas_limit);
        }
        
        Ok(total_estimate)
    }
    
    /// Simulate state changes (simplified version for now)
    async fn simulate_state_changes(&self, tx: &Transaction) -> TxResult<Vec<StateChange>> {
        let mut changes = Vec::new();
        
        // For ETH transfers, track balance changes
        if tx.params.value > U256::zero() && tx.params.to.is_some() {
            // Get sender's balance before
            let sender_balance_before = self.provider
                .get_balance(tx.params.from, None)
                .await
                .map_err(|e| TxError::SimulationFailed(format!("Failed to get sender balance: {}", e)))?;
            
            // Recipient balance before
            let recipient = tx.params.to.unwrap();
            let recipient_balance_before = self.provider
                .get_balance(recipient, None)
                .await
                .map_err(|e| TxError::SimulationFailed(format!("Failed to get recipient balance: {}", e)))?;
            
            // Simulate ETH transfer
            let transfer_amount = tx.params.value;
            
            // Calculate balances after (simplified - doesn't account for gas)
            let sender_balance_after = sender_balance_before - transfer_amount;
            let recipient_balance_after = recipient_balance_before + transfer_amount;
            
            // Convert U256 to bytes for storage in state change
            let sender_before_bytes = ethers::utils::format_units(sender_balance_before, "wei")
                .unwrap_or_default()
                .as_bytes()
                .to_vec();
            let sender_after_bytes = ethers::utils::format_units(sender_balance_after, "wei")
                .unwrap_or_default()
                .as_bytes()
                .to_vec();
            let recipient_before_bytes = ethers::utils::format_units(recipient_balance_before, "wei")
                .unwrap_or_default()
                .as_bytes()
                .to_vec();
            let recipient_after_bytes = ethers::utils::format_units(recipient_balance_after, "wei")
                .unwrap_or_default()
                .as_bytes()
                .to_vec();
            
            // Add sender balance change
            changes.push(StateChange {
                change_type: StateChangeType::Balance,
                address: tx.params.from,
                slot: None,
                old_value: Some(Bytes::from(sender_before_bytes)),
                new_value: Some(Bytes::from(sender_after_bytes)),
            });
            
            // Add recipient balance change
            changes.push(StateChange {
                change_type: StateChangeType::Balance,
                address: recipient,
                slot: None,
                old_value: Some(Bytes::from(recipient_before_bytes)),
                new_value: Some(Bytes::from(recipient_after_bytes)),
            });
        }
        
        // In a real implementation, we would track more complex state changes
        // like storage modifications and contract deployments
        
        Ok(changes)
    }
    
    /// Helper to create a transaction request from our transaction type
    fn create_tx_request(&self, tx: &Transaction) -> TransactionRequest {
        let mut request = TransactionRequest::new()
            .from(tx.params.from)
            .data(tx.params.data.clone())
            .value(tx.params.value);
        
        // Add to address if not contract creation
        if let Some(to) = tx.params.to {
            request = request.to(to);
        }
        
        // Add gas limit
        request = request.gas(tx.params.gas_limit);
        
        // Add nonce if specified
        if let Some(nonce) = tx.params.nonce {
            request = request.nonce(nonce);
        }
        
        // Set gas pricing based on transaction type
        match tx.params.tx_type {
            crate::tx_execution::types::TransactionType::Legacy => {
                if let Some(gas_price) = tx.params.gas_price {
                    request = request.gas_price(gas_price);
                }
            },
            crate::tx_execution::types::TransactionType::Eip1559 => {
                // For EIP-1559, ethers-rs uses different transaction types
                // We'll keep it simple for now and just set gas price for tests
                if let Some(max_fee) = tx.params.max_fee_per_gas {
                    request = request.gas_price(max_fee);
                }
            }
        }
        
        request
    }
    
    /// Run additional checks for transaction safety
    pub async fn validate_transaction(&self, tx: &Transaction) -> TxResult<bool> {
        // Verify sender has sufficient balance for value + gas
        let balance = self.provider
            .get_balance(tx.params.from, None)
            .await
            .map_err(|e| TxError::ValidationFailed(format!("Failed to get balance: {}", e)))?;
        
        // Estimate max gas cost
        let gas_price = match tx.params.tx_type {
            crate::tx_execution::types::TransactionType::Legacy => {
                tx.params.gas_price.unwrap_or_else(|| U256::from(50_000_000_000u64)) // 50 Gwei default
            },
            crate::tx_execution::types::TransactionType::Eip1559 => {
                tx.params.max_fee_per_gas.unwrap_or_else(|| U256::from(50_000_000_000u64)) // 50 Gwei default
            }
        };
        
        let max_gas_cost = tx.params.gas_limit * gas_price;
        let total_required = tx.params.value + max_gas_cost;
        
        if balance < total_required {
            return Err(TxError::InsufficientFunds(format!(
                "Insufficient balance for transaction: have {} wei, need {} wei",
                balance, total_required
            )));
        }
        
        // Additional validations could be added here
        
        Ok(true)
    }
}
