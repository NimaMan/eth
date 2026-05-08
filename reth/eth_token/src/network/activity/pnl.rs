//! Lightweight PnL proxy calculations for address activity.

use serde::{Deserialize, Serialize};

use crate::network::activity::movement::round_near_zero;

/// Inputs for the lightweight address-level PnL proxy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AddressPnlInput {
    pub token_balance: f64,
    pub token_latest_price: Option<f64>,
    pub total_denom_spent: f64,
    pub total_denom_received: f64,
    pub bribe_amount: f64,
    pub tx_fee_amount: f64,
}

/// PnL proxy used for live ranking and cluster summaries.
///
/// This is intentionally not accounting-grade PnL. It mirrors the old Python
/// feature set: realized denom balance minus fees/bribes, plus current holdings
/// valued at the latest available token price.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AddressPnlProxy {
    pub holdings_value: f64,
    pub realized_profit: f64,
    pub unrealized_profit: f64,
    pub total_profit: f64,
}

impl AddressPnlProxy {
    pub fn from_input(input: AddressPnlInput) -> Self {
        let latest_price = input
            .token_latest_price
            .filter(|price| price.is_finite() && *price > 0.0)
            .unwrap_or(0.0);
        let holdings_value = round_near_zero(input.token_balance * latest_price);
        let realized_profit = round_near_zero(
            input.total_denom_received
                - input.total_denom_spent
                - input.bribe_amount
                - input.tx_fee_amount,
        );
        let unrealized_profit = holdings_value;
        let total_profit = round_near_zero(realized_profit + unrealized_profit);

        Self {
            holdings_value,
            realized_profit,
            unrealized_profit,
            total_profit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pnl_proxy_uses_latest_holdings_value_and_costs() {
        let pnl = AddressPnlProxy::from_input(AddressPnlInput {
            token_balance: 10.0,
            token_latest_price: Some(0.25),
            total_denom_spent: 2.0,
            total_denom_received: 1.0,
            bribe_amount: 0.1,
            tx_fee_amount: 0.2,
        });

        assert_eq!(pnl.holdings_value, 2.5);
        assert_eq!(pnl.realized_profit, -1.3);
        assert_eq!(pnl.unrealized_profit, 2.5);
        assert_eq!(pnl.total_profit, 1.2);
    }
}
