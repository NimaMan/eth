use alloy_primitives::Address;
use ethers::types::H256;
use tx_processor::PoolBuySellSimulationResult;

use crate::mempool_fetcher::MempoolTransaction;
use crate::simulator::LiquidityRemovalResult;
use crate::tx_router::{SimulationPriority, TransactionCategory};

/// Types of simulation to perform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimulationType {
    /// Only simulate the transaction itself.
    TransactionOnly,
    /// Simulate transaction then buy/sell sequence.
    TransactionWithBuySell,
    /// Only simulate buy/sell (for existing tokens).
    BuySellOnly,
}

/// Request for simulation.
#[derive(Debug, Clone)]
pub struct TxSimulationJob {
    pub tx: MempoolTransaction,
    pub category: TransactionCategory,
    pub priority: SimulationPriority,
    pub simulation_type: SimulationType,
    pub tx_hash: H256,
}

/// Result of simulation.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub request: TxSimulationJob,
    pub error: Option<String>,
    pub simulation_time_ms: f64,
    // Addresses needed for compatibility.
    pub token_address: Option<Address>,
    pub pool_address: Option<Address>,
    pub pool_type: Option<String>, // Pool type (V2, V3, V4).
    // Debug info for error analysis.
    pub debug_info: Option<String>,
    pub pool_viability_result: Option<PoolBuySellSimulationResult>,
    // Liquidity removal result (only populated for liquidity removal transactions).
    pub liquidity_removal_result: Option<LiquidityRemovalResult>,
}

impl SimulationResult {
    /// Get buy/sell result for backward compatibility.
    pub fn buy_sell_result(&self) -> Option<BuySellResult> {
        self.pool_viability_result.as_ref().map(BuySellResult::from)
    }
}

/// Buy/sell simulation result wrapper for backward compatibility.
#[derive(Debug, Clone)]
pub struct BuySellResult {
    pub can_buy: bool,
    pub can_sell: bool,
    pub buy_tax: Option<f64>,  // 0-100% or None if calculation failed.
    pub sell_tax: Option<f64>, // 0-100% or None if calculation failed.
    pub buy_tax_error: Option<String>, // Error message if buy tax calculation failed.
    pub sell_tax_error: Option<String>, // Error message if sell tax calculation failed.
}

impl From<&PoolBuySellSimulationResult> for BuySellResult {
    fn from(result: &PoolBuySellSimulationResult) -> Self {
        Self {
            can_buy: result.can_buy,
            can_sell: result.can_sell,
            buy_tax: if result.buy_tax_percent >= 0.0 {
                Some(result.buy_tax_percent)
            } else {
                None
            },
            sell_tax: if result.sell_tax_percent >= 0.0 {
                Some(result.sell_tax_percent)
            } else {
                None
            },
            buy_tax_error: if result.buy_tax_percent < 0.0 {
                Some("Failed to calculate buy tax".to_string())
            } else {
                None
            },
            sell_tax_error: if result.sell_tax_percent < 0.0 {
                Some("Failed to calculate sell tax".to_string())
            } else {
                None
            },
        }
    }
}
