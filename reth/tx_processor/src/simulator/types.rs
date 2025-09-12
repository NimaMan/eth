/// Type definitions for trading viability analysis

use alloy_primitives::{Address, U256};
use crate::tx_processor::data_models::ProcessedTransaction;
use serde::{Serialize, Deserialize};

/// Supported DEX pool types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3 { fee_tier: u32 }, // 500, 3000, 10000 (0.05%, 0.3%, 1%)
    SushiSwap,
    Curve,
    Balancer,
    /// Placeholder for Uniswap V4 (PoolManager + PoolId based)
    /// Full swap support requires Router/Lock integration; not yet implemented
    UniswapV4,
}

/// Result of pool viability analysis
#[derive(Debug, Clone)]
pub struct PoolViabilityResult {
    pub pool_type: PoolType,
    pub pool_address: Address,
    pub token_address: Address,
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    pub is_tradeable: bool,
    pub buy_tax_percent: f64,
    pub sell_tax_percent: f64,
    pub tokens_received: U256,
    pub eth_spent: U256,
    pub eth_received: U256,
    pub buy_transaction: ProcessedTransaction,
    pub sell_transaction: ProcessedTransaction,
    pub approve_transaction: ProcessedTransaction,
    pub prior_transaction: Option<ProcessedTransaction>,
    pub failure_reason: Option<String>,
    pub block_number: u64,
}

#[derive(Debug, Clone)]
pub struct TradingSequenceResult {
    pub setup_tx_result: Option<ProcessedTransaction>,
    pub token_buy_result: ProcessedTransaction,
    pub token_approve_result: ProcessedTransaction,
    pub token_sell_result: ProcessedTransaction,
    pub tokens_bought_amount: U256,
    pub eth_spent_on_tokens: U256,
    pub eth_received_from_selling_tokens: U256,
    pub buy_tax_percentage: f64,
    pub sell_tax_percentage: f64,
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    pub all_transactions_succeeded: bool,
    pub token_is_tradeable: bool,
    pub total_gas_used: u64,
    pub simulation_block_number: u64,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OptionalSetupBuyApproveSellResult {
    pub setup_tx_result: Option<ProcessedTransaction>,
    pub token_buy_result: ProcessedTransaction,
    pub token_approve_result: ProcessedTransaction,
    pub token_sell_result: ProcessedTransaction,
    pub tokens_bought_amount: U256,
    pub eth_spent_on_tokens: f64,
    pub eth_received_from_selling_tokens: f64,
    pub buy_tax_percentage: f64,
    pub sell_tax_percentage: f64,
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    pub all_transactions_succeeded: bool,
    pub token_is_tradeable: bool,
    pub total_gas_used: u64,
    pub simulation_block_number: u64,
    pub failure_reason: Option<String>,
}

impl TradingSequenceResult {
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
