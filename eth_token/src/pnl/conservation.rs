use serde::{Deserialize, Serialize};

use super::PoolPnlTracker;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolPnlConservationCheck {
    pub token_is_conserved: bool,
    pub denom_is_conserved: bool,
    pub token_delta_raw: String,
    pub denom_delta_raw: String,
    pub token_transfer_count: u64,
    pub denom_transfer_count: u64,
}

impl PoolPnlTracker {
    pub fn conservation_check(&self) -> PoolPnlConservationCheck {
        let summary = self.conservation_summary();
        PoolPnlConservationCheck {
            token_is_conserved: summary.token_is_conserved,
            denom_is_conserved: summary.denom_is_conserved,
            token_delta_raw: summary.token_delta_raw,
            denom_delta_raw: summary.denom_delta_raw,
            token_transfer_count: summary.token_transfer_count,
            denom_transfer_count: summary.denom_transfer_count,
        }
    }
}
