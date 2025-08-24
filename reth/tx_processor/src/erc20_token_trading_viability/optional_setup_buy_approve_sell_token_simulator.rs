/// Optional Setup + Buy + Approve + Sell Token Simulator
/// 
/// Simulates the complete token trading lifecycle sequentially:
/// [optional_setup_tx] -> buy_tokens_with_eth -> approve_token_spending -> sell_all_tokens_for_eth
/// 
/// This simulator handles the common pattern where:
/// 1. Optional setup transaction (e.g., enable trading, remove limits, etc.)
/// 2. Buy tokens with ETH from a liquidity pool  
/// 3. Approve the router to spend the received tokens
/// 4. Sell all received tokens back to the pool for ETH
/// 
/// Key Innovation: 
/// - Simulates each transaction sequentially while preserving blockchain state between them
/// - Dynamically calculates exact token amount from buy transaction
/// - Uses that exact amount for the sell transaction
/// - Maintains blockchain state throughout the sequence
/// 
/// Tax Calculation:
/// - Buy tax = percentage of ETH lost during token purchase
/// - Sell tax = percentage of ETH lost during token sale
/// - Based on actual address balance changes during simulation

use std::sync::Arc;
use eyre::Result;
use alloy_primitives::{Address, U256};
use reth_tx_simulator::CallRequest;
use crate::data_models::ProcessedTransaction;
use crate::TxProcessor;
use crate::chain_state_persisting_sequential_tx_simulator::ChainStatePersistingSequentialTxSimulator;
use crate::erc20_token_trading_viability::{
    pool_adapters::PoolAdapter,
    tax_calculator::{
        calculate_buy_tax_from_processed_transaction, 
        calculate_sell_tax_from_processed_transaction, 
        extract_eth_received_from_processed_transaction, 
        TaxCalculationResult
    },
};

/// Simulates the exact sequence: [optional_setup_tx] -> buy_tokens_with_eth -> approve_token_spending -> sell_all_tokens_for_eth
/// Calculates exact token amount from buy transaction and uses it for sell transaction
pub struct OptionalSetupBuyApproveSellTokenSimulator {
    /// The underlying chain state persisting simulator that maintains blockchain state
    chain_state_simulator: ChainStatePersistingSequentialTxSimulator,
    /// Transaction processor for converting simulation results
    tx_processor: Arc<TxProcessor>,
}

/// Complete result of the optional setup + buy + approve + sell token trading sequence
#[derive(Debug, Clone)]
pub struct OptionalSetupBuyApproveSellResult {
    /// Result of the optional setup transaction (e.g., enable trading)
    pub setup_tx_result: Option<ProcessedTransaction>,
    /// Result of buying tokens with ETH
    pub token_buy_result: ProcessedTransaction,
    /// Result of approving token spending by router
    pub token_approve_result: ProcessedTransaction,
    /// Result of selling all tokens back for ETH
    pub token_sell_result: ProcessedTransaction,
    /// Exact amount of tokens bought (extracted from buy transaction)
    pub tokens_bought_amount: U256,
    /// Amount of ETH originally spent on tokens
    pub eth_spent_on_tokens: U256,
    /// Amount of ETH received from selling all tokens
    pub eth_received_from_selling_tokens: U256,
    /// Buy tax percentage (ETH lost during token purchase)
    pub buy_tax_percentage: f64,
    /// Sell tax percentage (ETH lost during token sale)
    pub sell_tax_percentage: f64,
    /// Whether all transactions in the sequence succeeded
    pub all_transactions_succeeded: bool,
    /// Whether the token can be traded (bought and sold successfully)
    pub token_is_tradeable: bool,
    /// Total gas used across all transactions
    pub total_gas_used: u64,
    /// Block number the sequence was simulated at
    pub simulation_block_number: u64,
    /// Detailed failure reason if trading failed
    pub failure_reason: Option<String>,
}

impl OptionalSetupBuyApproveSellTokenSimulator {
    /// Create a new optional setup + buy + approve + sell token simulator
    pub fn new(
        chain_state_simulator: ChainStatePersistingSequentialTxSimulator,
        tx_processor: Arc<TxProcessor>,
    ) -> Self {
        Self {
            chain_state_simulator,
            tx_processor,
        }
    }
    
    /// Simulate the complete token trading lifecycle with optional setup transaction
    /// 
    /// This is the main method that orchestrates the entire sequence:
    /// 1. [Optional] Setup transaction (enable trading, remove limits, etc.)
    /// 2. Buy tokens with ETH from liquidity pool
    /// 3. Extract exact token amount received from buy transaction
    /// 4. Approve router to spend the exact amount of received tokens  
    /// 5. Sell all received tokens back to pool for ETH
    /// 6. Calculate buy and sell tax percentages
    /// 
    /// # Arguments
    /// * `optional_setup_tx` - Optional setup transaction (e.g., enable trading)
    /// * `eth_amount_to_spend_on_tokens` - Amount of ETH to spend buying tokens
    /// * `token_contract_address` - Address of the token contract
    /// * `liquidity_pool_address` - Address of the liquidity pool
    /// * `trader_wallet_address` - Address of the trader's wallet
    /// * `pool_adapter` - Adapter for building pool-specific transactions
    /// * `slippage_tolerance` - Maximum acceptable slippage (e.g., 0.02 for 2%)
    /// * `simulation_block` - Block to simulate at (None for latest)
    /// 
    /// # Returns
    /// Complete result including all transaction details and tax calculations
    pub async fn simulate_optional_setup_then_buy_approve_sell_token_sequence(
        &mut self,
        optional_setup_tx: Option<CallRequest>,
        eth_amount_to_spend_on_tokens: U256,
        token_contract_address: Address,
        liquidity_pool_address: Address,
        trader_wallet_address: Address,
        pool_adapter: Box<dyn PoolAdapter>,
        slippage_tolerance: f64,
        simulation_block: Option<u64>,
    ) -> Result<OptionalSetupBuyApproveSellResult> {
        // Initialize blockchain state at the specified block
        self.chain_state_simulator.initialize_blockchain_state_at_block(simulation_block)?;
        
        let simulation_block_number = self.chain_state_simulator
            .get_fork_block_number()
            .unwrap_or(0);
        
        // Step 1: Simulate optional setup transaction if provided
        let setup_tx_result = if let Some(setup_tx) = optional_setup_tx {
            let result = self.chain_state_simulator
                .simulate_next_transaction_preserving_blockchain_state(
                    setup_tx,
                    self.tx_processor.clone()
                ).await?;
            Some(result.clone())
        } else {
            None
        };
        
        // Get buyer's nonce after optional setup (buyer nonce won't change from setup tx)
        let buyer_nonce = self.chain_state_simulator.get_nonce_for_address(
            trader_wallet_address,
            Some(simulation_block_number)
        )?;
        
        // Step 2: Build and simulate buy transaction with explicit nonce
        let mut buy_tx = pool_adapter.build_buy_transaction(
            token_contract_address,
            eth_amount_to_spend_on_tokens,
            trader_wallet_address,
            slippage_tolerance,
        )?;
        buy_tx.nonce = Some(buyer_nonce);
        
        let buy_result_ref = self.chain_state_simulator
            .simulate_next_transaction_preserving_blockchain_state(
                buy_tx,
                self.tx_processor.clone()
            ).await?;
        
        // Clone immediately to end the mutable borrow
        let buy_result = buy_result_ref.clone();
        
        // Check if buy transaction failed and return early with failure details
        if buy_result.status != "1" {
            // Buy failed - return result indicating trading is not enabled
            // Extract the failure reason from the transaction
            let failure_reason = self.extract_failure_reason_from_transaction(&buy_result);
            
            // Create placeholder transactions for approve and sell since they weren't executed
            let empty_approve = ProcessedTransaction::empty_failed(
                trader_wallet_address,
                Some(token_contract_address),
                buyer_nonce + 1,
                "Skipped: buy transaction failed"
            );
            
            let empty_sell = ProcessedTransaction::empty_failed(
                trader_wallet_address,
                Some(pool_adapter.router_address()),
                buyer_nonce + 2,
                "Skipped: buy transaction failed"
            );
            
            return Ok(OptionalSetupBuyApproveSellResult {
                setup_tx_result,
                token_buy_result: buy_result,
                token_approve_result: empty_approve,
                token_sell_result: empty_sell,
                tokens_bought_amount: U256::ZERO,
                eth_spent_on_tokens: eth_amount_to_spend_on_tokens,
                eth_received_from_selling_tokens: U256::ZERO,
                buy_tax_percentage: -1.0,
                sell_tax_percentage: -1.0,
                all_transactions_succeeded: false,
                token_is_tradeable: false,
                total_gas_used: self.chain_state_simulator.get_cumulative_gas_used(),
                simulation_block_number,
                failure_reason: Some(failure_reason),
            });
        }
        
        // Step 3: Extract exact token amount received from buy transaction
        let tokens_bought_amount = self.extract_tokens_received_from_buy_transaction(
            &buy_result, // Now using cloned data
            token_contract_address,
            trader_wallet_address,
        )?;
        
        
        // Step 4: Build and simulate approve transaction for exact token amount with explicit nonce
        let mut approve_tx = pool_adapter.build_approve_transaction(
            token_contract_address,
            tokens_bought_amount, // Approve exactly what we received
            trader_wallet_address,
        )?;
        approve_tx.nonce = Some(buyer_nonce + 1);
        
        let approve_result_ref = self.chain_state_simulator
            .simulate_next_transaction_preserving_blockchain_state(
                approve_tx,
                self.tx_processor.clone()
            ).await?;
        
        // Clone immediately to end the mutable borrow
        let approve_result = approve_result_ref.clone();
        
        // Step 5: Build and simulate sell transaction with exact token amount and explicit nonce
        let mut sell_tx = pool_adapter.build_sell_transaction(
            token_contract_address,
            tokens_bought_amount, // Sell exactly what we bought
            trader_wallet_address,
            slippage_tolerance,
        )?;
        sell_tx.nonce = Some(buyer_nonce + 2);
        
        let sell_result_ref = self.chain_state_simulator
            .simulate_next_transaction_preserving_blockchain_state(
                sell_tx,
                self.tx_processor.clone()
            ).await?;
        
        // Clone immediately to end the mutable borrow
        let sell_result = sell_result_ref.clone();
        
        // Step 6: Calculate tax percentages using centralized tax calculator
        let buy_tax_percentage = match calculate_buy_tax_from_processed_transaction(
            &buy_result,
            &liquidity_pool_address,
            &trader_wallet_address,
            &token_contract_address,
        ) {
            TaxCalculationResult::Calculated(tax) => tax,
            TaxCalculationResult::InvalidSimulation { reason } => {
                tracing::warn!("Buy tax calculation failed: {}", reason);
                -1.0 // Indicates calculation failed
            }
        };
        
        let sell_tax_percentage = match calculate_sell_tax_from_processed_transaction(
            &sell_result,
            &liquidity_pool_address,
            &trader_wallet_address,
        ) {
            TaxCalculationResult::Calculated(tax) => tax,
            TaxCalculationResult::InvalidSimulation { reason } => {
                tracing::warn!("Sell tax calculation failed: {}", reason);
                -1.0 // Indicates calculation failed
            }
        };
        
        // Step 7: Extract ETH received from selling tokens using centralized function
        let eth_received_from_selling_tokens = extract_eth_received_from_processed_transaction(
            &sell_result,
            &trader_wallet_address,
        );
        
        // Step 8: Determine success and failure reasons
        let all_transactions_succeeded = 
            setup_tx_result.as_ref().map_or(true, |tx| tx.status == "1") &&
            buy_result.status == "1" &&
            approve_result.status == "1" &&
            sell_result.status == "1";
        
        let token_is_tradeable = buy_result.status == "1" && sell_result.status == "1";
        
        let failure_reason = if !token_is_tradeable {
            Some(self.determine_failure_reason(&buy_result, &sell_result)) // Use references to cloned data
        } else {
            None
        };
        
        // Extract gas used (this is safe since it doesn't require any borrows)
        let total_gas_used = self.chain_state_simulator.get_cumulative_gas_used();
        
        Ok(OptionalSetupBuyApproveSellResult {
            setup_tx_result,
            token_buy_result: buy_result, // Move the cloned data
            token_approve_result: approve_result, // Move the cloned data
            token_sell_result: sell_result, // Move the cloned data
            tokens_bought_amount,
            eth_spent_on_tokens: eth_amount_to_spend_on_tokens,
            eth_received_from_selling_tokens,
            buy_tax_percentage,
            sell_tax_percentage,
            all_transactions_succeeded,
            token_is_tradeable,
            total_gas_used,
            simulation_block_number,
            failure_reason,
        })
    }
    
    /// Just simulate the buy + approve + sell sequence (no setup transaction)
    /// 
    /// Convenience method for when no setup transaction is needed.
    pub async fn simulate_buy_approve_sell_token_sequence_without_setup(
        &mut self,
        eth_amount_to_spend_on_tokens: U256,
        token_contract_address: Address,
        liquidity_pool_address: Address,
        trader_wallet_address: Address,
        pool_adapter: Box<dyn PoolAdapter>,
        slippage_tolerance: f64,
        simulation_block: Option<u64>,
    ) -> Result<OptionalSetupBuyApproveSellResult> {
        self.simulate_optional_setup_then_buy_approve_sell_token_sequence(
            None, // No setup transaction
            eth_amount_to_spend_on_tokens,
            token_contract_address,
            liquidity_pool_address,
            trader_wallet_address,
            pool_adapter,
            slippage_tolerance,
            simulation_block,
        ).await
    }
    
    /// Extract the exact amount of tokens received from a buy transaction
    fn extract_tokens_received_from_buy_transaction(
        &self,
        buy_tx_result: &ProcessedTransaction,
        token_contract_address: Address,
        trader_wallet_address: Address,
    ) -> Result<U256> {
        // First, try to extract from address_balance_changes
        if let Some(balance_change) = buy_tx_result.address_balance_changes.get(&trader_wallet_address) {
            // Try to find the token by checksummed address
            let token_checksum = crate::utils::to_checksum_address(&token_contract_address);
            
            if let Some(token_amount_json) = balance_change.get(&token_checksum) {
                if let Some(token_net) = token_amount_json.get("token_net") {
                    if let Some(amount_str) = token_net.get(&token_checksum) {
                        if let Some(amount_str) = amount_str.as_str() {
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
        
        // Fallback: look through ERC20 transfers for tokens received by trader
        for transfer in &buy_tx_result.erc20_transfers {
            if transfer.token_address == token_contract_address && 
               transfer.to_address == trader_wallet_address &&
               transfer.amount > U256::ZERO {
                return Ok(transfer.amount);
            }
        }
        
        // If we can't extract the token amount after checking both sources, log the error
        eprintln!("ERROR: Could not extract token amount from buy transaction!");
        eprintln!("  Token address: {:?}", token_contract_address);
        eprintln!("  Trader address: {:?}", trader_wallet_address);
        eprintln!("  Buy tx hash: {:?}", buy_tx_result.hash);
        eprintln!("  ERC20 transfers count: {}", buy_tx_result.erc20_transfers.len());
        
        // Return zero - the sell will fail anyway
        Ok(U256::ZERO)
    }
    
    /// Determine the reason why trading failed
    fn determine_failure_reason(
        &self,
        buy_result: &ProcessedTransaction,
        sell_result: &ProcessedTransaction,
    ) -> String {
        if buy_result.status != "1" {
            "Buy transaction failed".to_string()
        } else if sell_result.status != "1" {
            "Sell transaction failed".to_string()
        } else {
            "Unknown trading failure".to_string()
        }
    }
    
    /// Extract the actual failure reason from a failed transaction
    /// This attempts to get the EVM error or revert reason if available
    fn extract_failure_reason_from_transaction(&self, tx: &ProcessedTransaction) -> String {
        // Since ProcessedTransaction doesn't have direct access to revert reason,
        // we can at least provide more context about the failed transaction
        // In the future, we could enhance ProcessedTransaction to include error details
        
        // Check if there are any events that might indicate the failure
        // For now, we'll provide a descriptive message based on transaction details
        if tx.status != "1" {
            // Check if value transfer failed (insufficient balance)
            if tx.value > U256::ZERO {
                return format!("Transaction failed: Possible insufficient balance for {} ETH transfer", 
                    format!("{:.6}", tx.value.to_string().parse::<f64>().unwrap_or(0.0) / 1e18));
            }
            
            // Check if no gas was used (transaction reverted immediately)
            if tx.fees.gas_used == 0 {
                return "Transaction failed: Reverted immediately (no gas used)".to_string();
            }
            
            // Generic failure with transaction details
            return format!("Transaction failed at block {} (gas used: {}, status: {})",
                tx.block_number, tx.fees.gas_used, tx.status);
        }
        
        "Transaction failed with unknown reason".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_result_struct_initialization() {
        // Test that the result struct can be properly initialized
        let result = OptionalSetupBuyApproveSellResult {
            setup_tx_result: None,
            token_buy_result: ProcessedTransaction::new(
                alloy_primitives::B256::ZERO,
                0, 0, 0,
                Address::ZERO, None,
                U256::ZERO, "1".to_string(), 0,
                vec![],
            ),
            token_approve_result: ProcessedTransaction::new(
                alloy_primitives::B256::ZERO,
                0, 0, 0,
                Address::ZERO, None,
                U256::ZERO, "1".to_string(), 0,
                vec![],
            ),
            token_sell_result: ProcessedTransaction::new(
                alloy_primitives::B256::ZERO,
                0, 0, 0,
                Address::ZERO, None,
                U256::ZERO, "1".to_string(), 0,
                vec![],
            ),
            tokens_bought_amount: U256::from(1000),
            eth_spent_on_tokens: U256::from(1_000_000_000_000_000_000u128),
            eth_received_from_selling_tokens: U256::from(900_000_000_000_000_000u128),
            buy_tax_percentage: 2.5,
            sell_tax_percentage: 3.0,
            all_transactions_succeeded: true,
            token_is_tradeable: true,
            total_gas_used: 500_000,
            simulation_block_number: 19_000_000,
            failure_reason: None,
        };
        
        assert_eq!(result.tokens_bought_amount, U256::from(1000));
        assert!(result.token_is_tradeable);
        assert!(result.failure_reason.is_none());
    }
}