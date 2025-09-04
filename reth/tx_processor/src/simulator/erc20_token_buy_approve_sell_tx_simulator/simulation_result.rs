/// Result types for trading simulation
/// 
/// Contains all result structures used by the trading viability system

use alloy_primitives::{Address, U256};
use crate::tx_processor::data_models::ProcessedTransaction;

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
    /// Whether the buy transaction succeeded
    pub can_buy: bool,
    /// Whether the approve transaction succeeded
    pub can_approve: bool,
    /// Whether the sell transaction succeeded
    pub can_sell: bool,
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

/// Simplified result for the optional setup + buy + approve + sell simulator
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
    /// Amount of ETH originally spent on tokens (in ETH units, not wei)
    pub eth_spent_on_tokens: f64,
    /// Amount of ETH received from selling all tokens (in ETH units, not wei)
    pub eth_received_from_selling_tokens: f64,
    /// Buy tax percentage (ETH lost during token purchase)
    pub buy_tax_percentage: f64,
    /// Sell tax percentage (ETH lost during token sale)
    pub sell_tax_percentage: f64,
    /// Whether the buy transaction succeeded
    pub can_buy: bool,
    /// Whether the approve transaction succeeded
    pub can_approve: bool,
    /// Whether the sell transaction succeeded
    pub can_sell: bool,
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

impl TradingSequenceResult {
    /// Convert to the simplified optional setup result format
    pub fn to_optional_setup_result(&self) -> OptionalSetupBuyApproveSellResult {
        OptionalSetupBuyApproveSellResult {
            setup_tx_result: self.setup_tx_result.clone(),
            token_buy_result: self.token_buy_result.clone(),
            token_approve_result: self.token_approve_result.clone(),
            token_sell_result: self.token_sell_result.clone(),
            tokens_bought_amount: self.tokens_bought_amount,
            eth_spent_on_tokens: self.eth_spent_on_tokens.to_string().parse::<f64>().unwrap_or(0.0) / 1e18,
            eth_received_from_selling_tokens: self.eth_received_from_selling_tokens.to_string().parse::<f64>().unwrap_or(0.0) / 1e18,
            buy_tax_percentage: self.buy_tax_percentage,
            sell_tax_percentage: self.sell_tax_percentage,
            can_buy: self.can_buy,
            can_approve: self.can_approve,
            can_sell: self.can_sell,
            all_transactions_succeeded: self.all_transactions_succeeded,
            token_is_tradeable: self.token_is_tradeable,
            total_gas_used: self.total_gas_used,
            simulation_block_number: self.simulation_block_number,
            failure_reason: self.failure_reason.clone(),
        }
    }
}