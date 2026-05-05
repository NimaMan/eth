use crate::amount::DecimalAmount;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PortfolioMetrics {
    pub total_positions: usize,
    pub active_positions: usize,
    pub realized_profit_eth: DecimalAmount,
    pub unrealized_profit_eth: DecimalAmount,
}
