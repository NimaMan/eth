use crate::{
    amount::Amount,
    error::{AlphaCoreError, Result},
    portfolio::PortfolioState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortfolioLimits {
    pub max_open_positions: usize,
    pub max_order_amount: Option<Amount>,
}

impl PortfolioLimits {
    pub fn validate_open_capacity(&self, portfolio: &PortfolioState) -> Result<()> {
        if portfolio.active_position_count() >= self.max_open_positions {
            return Err(AlphaCoreError::PortfolioLimit(format!(
                "open positions {} reached max {}",
                portfolio.active_position_count(),
                self.max_open_positions
            )));
        }
        Ok(())
    }
}
