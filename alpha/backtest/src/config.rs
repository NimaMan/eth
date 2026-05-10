use serde::{Deserialize, Serialize};

/// Top-level configuration for a backtest run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub run_id: String,
    pub strategy_name: String,
    pub mode: String,
}
