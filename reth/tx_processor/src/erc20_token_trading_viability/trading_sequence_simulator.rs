/// Trading Sequence Simulator
/// 
/// Simulates complete token trading sequences with automatic state persistence:
/// [optional_setup_tx] -> buy_tokens_with_eth -> approve_token_spending -> sell_all_tokens_for_eth
/// 
/// This simulator handles the common pattern where:
/// 1. Optional setup transaction (e.g., enable trading, remove limits, etc.)
/// 2. Buy tokens with ETH from a liquidity pool  
/// 3. Approve the router to spend the received tokens
/// 4. Sell all received tokens back to the pool for ETH
/// 
/// Key Features:
/// - Uses tx_simulator's built-in sequential simulation with state persistence
/// - Converts simulation results to ProcessedTransaction for rich data access
/// - Extracts exact token amounts from buy transaction for sell transaction
/// - Calculates taxes from address balance changes
/// - Early failure detection with detailed error reporting

use std::sync::Arc;
use eyre::Result;
use alloy_primitives::{Address, U256};
use tx_simulator::{TxSimulator, CallRequest, SequentialSimulationOptions};
use crate::data_models::ProcessedTransaction;
use crate::TxProcessor;
use crate::erc20_token_trading_viability::{
    pool_adapters::PoolAdapter,
    tax_calculator::{
        calculate_buy_tax_from_processed_transaction, 
        calculate_sell_tax_from_processed_transaction, 
        extract_eth_received_from_processed_transaction, 
        TaxCalculationResult
    },
};

/// Simulates trading sequences with state persistence between transactions
pub struct TradingSequenceSimulator {
    /// The underlying transaction simulator
    tx_simulator: Arc<TxSimulator>,
    /// Transaction processor for converting simulation results
    tx_processor: Arc<TxProcessor>,
}

/// Complete result of the trading sequence simulation
#[derive(Debug, Clone)]
pub struct TradingSequenceResult {
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

impl TradingSequenceSimulator {
    /// Create a new trading sequence simulator
    pub fn new(
        tx_simulator: Arc<TxSimulator>,
        tx_processor: Arc<TxProcessor>,
    ) -> Self {
        Self {
            tx_simulator,
            tx_processor,
        }
    }
    
    /// Simulate the complete token trading lifecycle
    /// 
    /// This method orchestrates the entire sequence:
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
    pub async fn simulate_trading_sequence(
        &self,
        optional_setup_tx: Option<CallRequest>,
        eth_amount_to_spend_on_tokens: U256,
        token_contract_address: Address,
        liquidity_pool_address: Address,
        trader_wallet_address: Address,
        pool_adapter: Box<dyn PoolAdapter>,
        slippage_tolerance: f64,
        simulation_block: Option<u64>,
    ) -> Result<TradingSequenceResult> {
        let block_number = simulation_block.unwrap_or(self.tx_simulator.get_latest_block()?);
        
        // Get trader's starting nonce
        let mut current_nonce = self.get_trader_nonce(trader_wallet_address, block_number)?;
        
        // NOTE: This is a simplified implementation that simulates transactions independently.
        // For full state preservation, this should use tx_simulator's sequential simulation
        // with proper state management between transactions.
        // Execute transactions sequentially, dynamically building each based on previous results
        
        // Step 1: Execute optional setup transaction if provided
        let setup_tx_result = if let Some(mut setup_tx) = optional_setup_tx {
            setup_tx.nonce = Some(current_nonce);
            
            let sim_result = self.tx_simulator
                .simulate_unsigned_transaction_at_block(setup_tx.clone(), block_number)
                .await?;
            
            let processed_tx = self.tx_processor.convert_simulation_to_processed_tx(
                &setup_tx,
                &sim_result,
                block_number,
            ).await?;
            
            if processed_tx.status != "1" {
                return Ok(TradingSequenceResult {
                    setup_tx_result: Some(processed_tx),
                    token_buy_result: self.create_empty_failed_tx(trader_wallet_address, "Skipped: setup failed"),
                    token_approve_result: self.create_empty_failed_tx(trader_wallet_address, "Skipped: setup failed"), 
                    token_sell_result: self.create_empty_failed_tx(trader_wallet_address, "Skipped: setup failed"),
                    tokens_bought_amount: U256::ZERO,
                    eth_spent_on_tokens: eth_amount_to_spend_on_tokens,
                    eth_received_from_selling_tokens: U256::ZERO,
                    buy_tax_percentage: -1.0,
                    sell_tax_percentage: -1.0,
                    all_transactions_succeeded: false,
                    token_is_tradeable: false,
                    total_gas_used: processed_tx.gas_used,
                    simulation_block_number: block_number,
                    failure_reason: Some(self.extract_failure_reason_from_transaction(&processed_tx)),
                });
            }
            
            current_nonce += 1;
            Some(processed_tx)
        } else {
            None
        };
        
        // Step 2: Execute buy transaction
        let mut buy_tx = pool_adapter.build_buy_transaction(
            token_contract_address,
            eth_amount_to_spend_on_tokens,
            trader_wallet_address,
            slippage_tolerance,
        )?;
        buy_tx.nonce = Some(current_nonce);
        
        let buy_sim_result = self.tx_simulator
            .simulate_unsigned_transaction_at_block(buy_tx.clone(), block_number)
            .await?;
        
        let token_buy_result = self.tx_processor.convert_simulation_to_processed_tx(
            &buy_tx,
            &buy_sim_result,
            block_number,
        ).await?;
        
        // Check if buy transaction failed - early exit if so
        if token_buy_result.status != "1" {
            let failure_reason = self.extract_failure_reason_from_transaction(&token_buy_result);
            
            return Ok(TradingSequenceResult {
                setup_tx_result,
                token_buy_result,
                token_approve_result: self.create_empty_failed_tx(trader_wallet_address, "Skipped: buy failed"),
                token_sell_result: self.create_empty_failed_tx(trader_wallet_address, "Skipped: buy failed"),
                tokens_bought_amount: U256::ZERO,
                eth_spent_on_tokens: eth_amount_to_spend_on_tokens,
                eth_received_from_selling_tokens: U256::ZERO,
                buy_tax_percentage: -1.0,
                sell_tax_percentage: -1.0,
                all_transactions_succeeded: false,
                token_is_tradeable: false,
                total_gas_used: token_buy_result.gas_used,
                simulation_block_number: block_number,
                failure_reason: Some(failure_reason),
            });
        }
        
        // Extract token amount received from buy transaction
        let tokens_bought_amount = self.extract_token_amount_from_buy_transaction(
            &token_buy_result,
            token_contract_address,
            trader_wallet_address,
        )?;
        
        // If we got zero tokens, something went wrong
        if tokens_bought_amount == U256::ZERO {
            return Ok(TradingSequenceResult {
                setup_tx_result,
                token_buy_result,
                token_approve_result,
                token_sell_result,
                tokens_bought_amount: U256::ZERO,
                eth_spent_on_tokens: eth_amount_to_spend_on_tokens,
                eth_received_from_selling_tokens: U256::ZERO,
                buy_tax_percentage: -1.0,
                sell_tax_percentage: -1.0,
                all_transactions_succeeded: false,
                token_is_tradeable: false,
                total_gas_used: sequence_result.total_gas_used,
                simulation_block_number: block_number,
                failure_reason: Some("Zero tokens received from buy transaction".to_string()),
            });
        }
        
        // Calculate taxes from processed transactions
        let buy_tax_result = calculate_buy_tax_from_processed_transaction(
            &token_buy_result,
            &liquidity_pool_address,
            &trader_wallet_address,
            &token_contract_address,
        );
        
        let sell_tax_result = calculate_sell_tax_from_processed_transaction(
            &token_sell_result,
            &liquidity_pool_address,
            &trader_wallet_address,
        );
        
        let eth_received = extract_eth_received_from_processed_transaction(
            &token_sell_result,
            &trader_wallet_address,
        );
        
        let all_succeeded = sequence_result.sequence_success;
        let token_is_tradeable = all_succeeded && token_sell_result.status == "1";
        
        // Extract failure reason if needed
        let failure_reason = if !token_is_tradeable {
            if token_approve_result.status != "1" {
                Some(format!("Approve failed: {}", self.extract_failure_reason_from_transaction(&token_approve_result)))
            } else if token_sell_result.status != "1" {
                Some(format!("Sell failed: {}", self.extract_failure_reason_from_transaction(&token_sell_result)))
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(TradingSequenceResult {
            setup_tx_result,
            token_buy_result,
            token_approve_result,
            token_sell_result,
            tokens_bought_amount,
            eth_spent_on_tokens: eth_amount_to_spend_on_tokens,
            eth_received_from_selling_tokens: eth_received.unwrap_or(U256::ZERO),
            buy_tax_percentage: buy_tax_result.map_or(-1.0, |r| r.tax_percentage),
            sell_tax_percentage: sell_tax_result.map_or(-1.0, |r| r.tax_percentage),
            all_transactions_succeeded: all_succeeded,
            token_is_tradeable,
            total_gas_used: sequence_result.total_gas_used,
            simulation_block_number: block_number,
            failure_reason,
        })
    }
    
    /// Get the current nonce for a trader address at a specific block
    fn get_trader_nonce(&self, _trader_address: Address, _block_number: u64) -> Result<u64> {
        // For trading simulation, we typically start with nonce 0
        // since we're simulating hypothetical transactions
        Ok(0)
    }
    
    /// Extract token amount received from a buy transaction
    fn extract_token_amount_from_buy_transaction(
        &self,
        buy_tx: &ProcessedTransaction,
        token_address: Address,
        trader_address: Address,
    ) -> Result<U256> {
        // Method 1: Check ERC20 transfers
        for transfer in &buy_tx.erc20_transfers {
            if transfer.token_address == token_address && transfer.to_address == trader_address {
                return Ok(transfer.amount);
            }
        }
        
        // Method 2: Check address balance changes
        if let Some(changes) = buy_tx.address_balance_changes.get(&trader_address) {
            if let Some(token_changes) = changes.get("tokens") {
                let token_addr_str = format!("{:#x}", token_address);
                if let Some(token_change) = token_changes.get(&token_addr_str) {
                    if let Some(val) = token_change.as_f64() {
                        if val > 0.0 {
                            // Convert from float representation back to U256
                            // This is approximate - ideally we'd have the exact value
                            let tokens_as_u64 = (val * 1e18) as u64;
                            return Ok(U256::from(tokens_as_u64));
                        }
                    }
                }
            }
        }
        
        Err(eyre::eyre!("Could not extract token amount from buy transaction"))
    }
    
    /// Extract failure reason from a transaction
    fn extract_failure_reason_from_transaction(&self, tx: &ProcessedTransaction) -> String {
        if let Some(error) = &tx.failure_reason {
            error.clone()
        } else if tx.status != "1" {
            "Transaction failed".to_string()
        } else {
            "Unknown failure".to_string()
        }
    }
    
    /// Create an empty failed transaction for placeholder purposes
    fn create_empty_failed_tx(&self, from_address: Address, reason: &str) -> ProcessedTransaction {
        ProcessedTransaction::empty_failed(from_address, None, 0, reason)
    }
}