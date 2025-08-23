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
    tax_calculator::{calculate_buy_tax, calculate_sell_tax, extract_eth_received, TaxCalculationResult},
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
        
        // Check if buy transaction failed and return early
        if buy_result.status != "1" {
            return Err(eyre::eyre!("Buy transaction failed - token may have trading disabled or other restrictions"));
        }
        
        // Step 3: Extract exact token amount received from buy transaction
        let tokens_bought_amount = self.extract_tokens_received_from_buy_transaction(
            &buy_result, // Now using cloned data
            token_contract_address,
            trader_wallet_address,
        )?;
        
        eprintln!("DEBUG: Extracted tokens_bought_amount = {}", tokens_bought_amount);
        eprintln!("DEBUG: Buy tx status = {}", buy_result.status);
        
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
        
        // Step 6: Calculate tax percentages from address balance changes
        let buy_tax_percentage = self.calculate_buy_tax_from_processed_transaction(
            &buy_result, // Use reference to cloned data
            &liquidity_pool_address,
            &trader_wallet_address,
            &token_contract_address,
        );
        
        let sell_tax_percentage = self.calculate_sell_tax_from_processed_transaction(
            &sell_result, // Use reference to cloned data
            &liquidity_pool_address,
            &trader_wallet_address,
        );
        
        // Step 7: Extract ETH received from selling tokens
        let eth_received_from_selling_tokens = self.extract_eth_received_from_sell_transaction(
            &sell_result, // Use reference to cloned data
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
    
    /// Calculate buy tax percentage from processed transaction
    fn calculate_buy_tax_from_processed_transaction(
        &self,
        buy_tx_result: &ProcessedTransaction,
        pool_address: &Address,
        trader_address: &Address,
        token_address: &Address,
    ) -> f64 {
        // Convert ProcessedTransaction address_balance_changes back to the format needed by tax calculator
        let address_balance_changes = self.convert_processed_tx_to_balance_changes(buy_tx_result);
        
        match calculate_buy_tax(&address_balance_changes, pool_address, trader_address, token_address) {
            TaxCalculationResult::Calculated(tax) => tax,
            TaxCalculationResult::InvalidSimulation { reason } => {
                tracing::warn!("Buy tax calculation failed: {}", reason);
                -1.0 // Indicates calculation failed
            }
        }
    }
    
    /// Calculate sell tax percentage from processed transaction
    fn calculate_sell_tax_from_processed_transaction(
        &self,
        sell_tx_result: &ProcessedTransaction,
        pool_address: &Address,
        trader_address: &Address,
    ) -> f64 {
        let address_balance_changes = self.convert_processed_tx_to_balance_changes(sell_tx_result);
        
        match calculate_sell_tax(&address_balance_changes, pool_address, trader_address) {
            TaxCalculationResult::Calculated(tax) => tax,
            TaxCalculationResult::InvalidSimulation { reason } => {
                tracing::warn!("Sell tax calculation failed: {}", reason);
                -1.0 // Indicates calculation failed
            }
        }
    }
    
    /// Extract ETH received from sell transaction
    fn extract_eth_received_from_sell_transaction(
        &self,
        sell_tx_result: &ProcessedTransaction,
        trader_address: &Address,
    ) -> U256 {
        let address_balance_changes = self.convert_processed_tx_to_balance_changes(sell_tx_result);
        
        extract_eth_received(&address_balance_changes, trader_address)
    }
    
    /// Convert ProcessedTransaction address_balance_changes to the format expected by tax calculator
    fn convert_processed_tx_to_balance_changes(
        &self,
        processed_tx: &ProcessedTransaction,
    ) -> std::collections::HashMap<Address, reth_tx_simulator::AddressBalanceChange> {
        use std::collections::HashMap;
        use alloy_primitives::I256;
        
        let mut result = HashMap::new();
        
        for (addr, balance_change_json) in &processed_tx.address_balance_changes {
            if let Ok(eth_net_str) = serde_json::from_value::<String>(
                balance_change_json.get("eth_net").unwrap_or(&serde_json::Value::String("0".to_string())).clone()
            ) {
                let eth_net = eth_net_str.parse::<i128>().unwrap_or(0);
                
                let mut token_net = HashMap::new();
                if let Some(token_net_json) = balance_change_json.get("token_net") {
                    if let Ok(token_net_map) = serde_json::from_value::<HashMap<String, String>>(token_net_json.clone()) {
                        for (token, amount_str) in token_net_map {
                            let amount = amount_str.parse::<i128>().unwrap_or(0);
                            token_net.insert(token, I256::try_from(amount).unwrap_or(I256::ZERO));
                        }
                    }
                }
                
                let balance_change = reth_tx_simulator::AddressBalanceChange {
                    eth_net: I256::try_from(eth_net).unwrap_or(I256::ZERO),
                    token_net,
                    movements: reth_tx_simulator::address_balance_change_calculator::AddressMovementsSummary {
                        denom: reth_tx_simulator::address_balance_change_calculator::AddressMovements {
                            incoming: std::collections::BTreeMap::new(),
                            outgoing: std::collections::BTreeMap::new(),
                        },
                        tokens: HashMap::new(),
                    },
                };
                
                result.insert(*addr, balance_change);
            }
        }
        
        result
    }
    
    /// Determine the reason why trading failed
    fn determine_failure_reason(
        &self,
        buy_result: &ProcessedTransaction,
        sell_result: &ProcessedTransaction,
    ) -> String {
        if buy_result.status != "1" {
            "Buy transaction failed - token may have trading disabled or other restrictions".to_string()
        } else if sell_result.status != "1" {
            "Sell transaction failed - token may prevent selling or have cooldown period".to_string()
        } else {
            "Unknown trading failure".to_string()
        }
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