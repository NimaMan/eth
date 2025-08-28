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
use tx_simulator::{TxSimulator, CallRequest};
use crate::data_models::ProcessedTransaction;
use crate::TxProcessor;
use crate::erc20_token_trading_viability::{
    pool_adapters::PoolAdapter,
    trading_sequence_simulator::{TradingSequenceSimulator, TradingSequenceResult},
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
    /// The underlying trading sequence simulator using the new tx_simulator
    trading_simulator: TradingSequenceSimulator,
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
        tx_simulator: Arc<TxSimulator>,
        tx_processor: Arc<TxProcessor>,
    ) -> Self {
        Self {
            trading_simulator: TradingSequenceSimulator::new(tx_simulator, tx_processor),
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
        &self,
        optional_setup_tx: Option<CallRequest>,
        eth_amount_to_spend_on_tokens: U256,
        token_contract_address: Address,
        liquidity_pool_address: Address,
        trader_wallet_address: Address,
        pool_adapter: Box<dyn PoolAdapter>,
        slippage_tolerance: f64,
        simulation_block: Option<u64>,
    ) -> Result<OptionalSetupBuyApproveSellResult> {
        // Delegate to the TradingSequenceSimulator for actual simulation
        let result = self.trading_simulator.simulate_trading_sequence(
            optional_setup_tx,
            eth_amount_to_spend_on_tokens,
            token_contract_address,
            liquidity_pool_address,
            trader_wallet_address,
            pool_adapter,
            slippage_tolerance,
            simulation_block,
        ).await?;
        
        // Convert TradingSequenceResult to OptionalSetupBuyApproveSellResult
        Ok(OptionalSetupBuyApproveSellResult {
            setup_tx_result: result.setup_tx_result,
            token_buy_result: result.token_buy_result,
            token_approve_result: result.token_approve_result,
            token_sell_result: result.token_sell_result,
            tokens_bought_amount: result.tokens_bought_amount,
            eth_spent_on_tokens: result.eth_spent_on_tokens,
            eth_received_from_selling_tokens: result.eth_received_from_selling_tokens,
            buy_tax_percentage: result.buy_tax_percentage,
            sell_tax_percentage: result.sell_tax_percentage,
            all_transactions_succeeded: result.all_transactions_succeeded,
            token_is_tradeable: result.token_is_tradeable,
            total_gas_used: result.total_gas_used,
            simulation_block_number: result.simulation_block_number,
            failure_reason: result.failure_reason,
        })
    }
    
    /// Just simulate the buy + approve + sell sequence (no setup transaction)
    /// 
    /// Convenience method for when no setup transaction is needed.
    pub async fn simulate_buy_approve_sell_token_sequence_without_setup(
        &self,
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
}
