use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Top-level configuration for a backtest run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub run_id: String,
    pub strategy_name: String,
    pub mode: String,
    pub simulation: SimulationConfig,
}

/// Simulation assumptions that control how the execution adapter behaves.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Additional slippage in basis points applied on top of the intent's
    /// `max_slippage_bps`.  Only affects sell-side worst-case fills by default.
    pub slippage_bps: u32,

    /// Gas cost charged per simulated transaction (wei).
    pub gas_cost_wei: u64,

    /// Probability of a random transaction failure in basis points (0–10_000).
    pub failure_rate_bps: u32,

    /// Minimum denom reserve a pool must have to be considered liquid enough
    /// for a trade.
    pub min_denom_reserve: Decimal,

    /// If `true`, trades on pools marked `is_scam = true` are rejected.
    pub reject_scam: bool,

    /// If `true`, trades on pools with `denom_reserve < min_denom_reserve`
    /// are rejected.
    pub reject_insufficient_liquidity: bool,

    /// If `true`, sell fills are reduced by `slippage_bps` to model worst-case
    /// execution.  Buys keep the intended amount (capital deployed) unchanged.
    pub worst_case_fill: bool,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 100,
            gas_cost_wei: 150_000,
            failure_rate_bps: 0,
            min_denom_reserve: Decimal::ZERO,
            reject_scam: true,
            reject_insufficient_liquidity: true,
            worst_case_fill: true,
        }
    }
}
