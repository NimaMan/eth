/// Trading Enabled Simulator
/// 
/// DEPRECATED: This module now delegates to erc20_token_trading_viability for core functionality.
/// Kept for backward compatibility.
/// 
/// Simulates a sequence of transactions to determine if trading is enabled for a token.
/// Returns ProcessedTransaction objects for each step with full state changes.
/// 
/// Sequence: [Enable Trading TX] → Buy → Approve → Sell

pub mod config;
pub mod tx_builder;
pub mod tax_calculator;

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, U256};
use tx_processor::TxProcessor;
use tx_processor::data_models::ProcessedTransaction;

pub use config::BuySellConfig;

// Import the viability module from tx_processor
use tx_processor::erc20_token_trading_viability::{
    analyze_pool_viability,
    PoolViabilityConfig,
    PoolType,
    PoolViabilityResult,
};

/// Result of a trading sequence simulation
#[derive(Debug, Clone)]
pub struct TradingSequenceResult {
    /// The simulated prior transaction (if provided)
    /// This is a NEW ProcessedTransaction generated from simulating the prior_tx
    pub prior_tx: Option<ProcessedTransaction>,
    /// Buy transaction result
    pub buy_tx: ProcessedTransaction,
    /// Approve transaction result
    pub approve_tx: ProcessedTransaction,
    /// Sell transaction result
    pub sell_tx: ProcessedTransaction,
    /// Whether trading is enabled (buy and sell both succeed)
    pub trading_enabled: bool,
    /// Calculated buy tax percentage (-1 if failed)
    pub buy_tax: f64,
    /// Calculated sell tax percentage (-1 if failed)
    pub sell_tax: f64,
    /// Block number used for simulation
    pub block_number: u64,
}

/// Trading Enabled Simulator
/// 
/// Now acts as a compatibility wrapper around erc20_token_trading_viability
pub struct TradingEnabledSimulator {
    /// Shared TxProcessor for transaction processing
    tx_processor: Arc<TxProcessor>,
    /// Configuration for buy/sell simulation
    config: BuySellConfig,
}

impl TradingEnabledSimulator {
    /// Create new simulator with shared processor
    pub fn new(tx_processor: Arc<TxProcessor>) -> Result<Self> {
        let config = BuySellConfig::default();
        
        Ok(Self {
            tx_processor,
            config,
        })
    }
    
    /// Create with custom configuration
    pub fn with_config(tx_processor: Arc<TxProcessor>, config: BuySellConfig) -> Result<Self> {
        Ok(Self {
            tx_processor,
            config,
        })
    }
    
    /// Get reference to the TxProcessor
    pub fn get_tx_processor(&self) -> &Arc<TxProcessor> {
        &self.tx_processor
    }
    
    /// Simulate trading sequence from a ProcessedTransaction
    /// 
    /// Takes an optional prior transaction (could be any tx) and simulates buy/sell
    /// Now delegates to erc20_token_trading_viability module
    pub async fn simulate_trading_sequence(
        &self,
        prior_tx: Option<&ProcessedTransaction>,
        token_address: Address,
        pool_address: Address,
        block_number: Option<u64>,
    ) -> Result<TradingSequenceResult> {
        // Get simulator from processor's chain_query
        let chain_query = self.tx_processor.chain_query.clone();
        let simulator = chain_query.get_simulator();
        
        // Build configuration for pool viability analysis
        let mut viability_config = PoolViabilityConfig::new(
            token_address,
            pool_address,
            PoolType::UniswapV2, // Default to V2 for backward compatibility
        )
        .with_test_amount(self.config.test_buy_amount)
        .with_buyer(self.config.buyer_address);
        
        // Add prior transaction if provided
        if let Some(tx) = prior_tx {
            viability_config = viability_config.with_prior_tx(tx.clone());
        }
        
        // Add block number if provided
        if let Some(block) = block_number {
            viability_config = viability_config.with_block(block);
        }
        
        // Run the viability analysis
        let result = analyze_pool_viability(
            simulator,
            self.tx_processor.clone(),
            viability_config,
        ).await?;
        
        // Convert PoolViabilityResult to TradingSequenceResult
        Ok(TradingSequenceResult {
            prior_tx: result.prior_transaction,
            buy_tx: result.buy_transaction,
            approve_tx: result.approve_transaction,
            sell_tx: result.sell_transaction,
            trading_enabled: result.is_tradeable,
            buy_tax: result.buy_tax_percent,
            sell_tax: result.sell_tax_percent,
            block_number: result.block_number,
        })
    }
}