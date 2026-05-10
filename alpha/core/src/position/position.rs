use crate::{
    amount::{Amount, DecimalAmount},
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus},
    ids::{OrderId, PoolAddress, PortfolioId, PositionId, StrategyName, TokenAddress, WalletId},
    order::OrderSide,
    position::PositionState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PositionKey {
    pub portfolio_id: PortfolioId,
    pub wallet_id: WalletId,
    pub strategy_name: StrategyName,
    pub token_address: TokenAddress,
    pub pool_address: PoolAddress,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub id: PositionId,
    pub key: PositionKey,
    pub state: PositionState,
    pub entry_order_id: Option<OrderId>,
    pub exit_order_id: Option<OrderId>,
    /// ETH amount spent on the buy (cost basis).
    pub entry_cost_basis: Option<DecimalAmount>,
    /// ETH amount received from the sell (proceeds).
    pub exit_proceeds: Option<DecimalAmount>,
    /// Pool price at time of buy (denom per token).
    pub entry_price: Option<DecimalAmount>,
}

impl Position {
    pub fn new(id: PositionId, key: PositionKey) -> Self {
        Self {
            id,
            key,
            state: PositionState::Init,
            entry_order_id: None,
            exit_order_id: None,
            entry_cost_basis: None,
            exit_proceeds: None,
            entry_price: None,
        }
    }

    pub fn mark_intent_created(&mut self, side: OrderSide) -> Result<()> {
        match (&self.state, side) {
            (PositionState::Init, OrderSide::Buy) => {
                self.state = PositionState::BuyIntentCreated;
                Ok(())
            }
            (PositionState::BuyConfirmed, OrderSide::Sell) => {
                self.state = PositionState::SellIntentCreated;
                Ok(())
            }
            _ => Err(AlphaCoreError::InvalidPositionTransition(format!(
                "cannot create {side:?} intent from {:?}",
                self.state
            ))),
        }
    }

    pub fn mark_order_submitted(&mut self, order_id: OrderId, side: OrderSide) -> Result<()> {
        match (&self.state, side) {
            (PositionState::BuyIntentCreated, OrderSide::Buy) => {
                self.entry_order_id = Some(order_id);
                self.state = PositionState::BuySubmitted;
                Ok(())
            }
            (PositionState::SellIntentCreated, OrderSide::Sell) => {
                self.exit_order_id = Some(order_id);
                self.state = PositionState::SellSubmitted;
                Ok(())
            }
            _ => Err(AlphaCoreError::InvalidPositionTransition(format!(
                "cannot submit {side:?} from {:?}",
                self.state
            ))),
        }
    }

    pub fn apply_execution_report(&mut self, report: &ExecutionReport) -> Result<()> {
        self.apply_execution_report_with_price(report, None)
    }

    /// Apply an execution report with optional fill price for PnL tracking.
    pub fn apply_execution_report_with_price(
        &mut self,
        report: &ExecutionReport,
        fill_price: Option<DecimalAmount>,
    ) -> Result<()> {
        match report.status {
            ExecutionStatus::Confirmed => {
                self.apply_confirmed_report(report, fill_price)
            }
            ExecutionStatus::Failed => {
                self.state = PositionState::Failed;
                Ok(())
            }
            ExecutionStatus::Cancelled => {
                self.state = PositionState::Cancelled;
                Ok(())
            }
            ExecutionStatus::Submitted | ExecutionStatus::Pending => Ok(()),
        }
    }

    fn apply_confirmed_report(
        &mut self,
        report: &ExecutionReport,
        fill_price: Option<DecimalAmount>,
    ) -> Result<()> {
        if self.entry_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::BuySubmitted
        {
            self.state = PositionState::BuyConfirmed;
            if let Some(amount) = &report.filled_amount {
                self.entry_cost_basis = Some(amount_to_decimal(amount));
            }
            if let Some(price) = fill_price {
                self.entry_price = Some(price);
            }
            return Ok(());
        }

        if self.exit_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::SellSubmitted
        {
            self.state = PositionState::SellConfirmed;
            if let Some(amount) = &report.filled_amount {
                self.exit_proceeds = Some(amount_to_decimal(amount));
            }
            return Ok(());
        }

        Err(AlphaCoreError::InvalidPositionTransition(format!(
            "confirmed report {:?} does not match position {:?}",
            report.order_id, self.state
        )))
    }

    /// Compute unrealized PnL given the current pool snapshot.
    ///
    /// The current value is capped at the pool's denom reserve (you cannot
    /// extract more liquidity than exists in the pool). This prevents
    /// astronomical paper valuations on illiquid tokens.
    ///
    /// Returns (current_value, unrealized_pnl) in denom terms.
    pub fn unrealized_pnl(&self, pool: &crate::market::PoolSnapshot) -> (DecimalAmount, DecimalAmount) {
        let Some(cost_basis) = self.entry_cost_basis else {
            return (DecimalAmount::ZERO, DecimalAmount::ZERO);
        };
        let Some(entry_price) = self.entry_price else {
            return (cost_basis, DecimalAmount::ZERO);
        };
        if entry_price.is_zero() {
            return (cost_basis, DecimalAmount::ZERO);
        }
        let current_price = pool.price_denom_per_token.unwrap_or_default();
        if current_price.is_zero() {
            return (cost_basis, DecimalAmount::ZERO);
        }

        // Price-based theoretical value
        let ratio = current_price / entry_price;
        let price_value = cost_basis * ratio;

        // Liquidity cap: cannot extract more than pool denom reserve
        let liquidity_cap = pool.denom_reserve;
        let current_value = price_value.min(liquidity_cap);

        let unrealized = current_value - cost_basis;
        (current_value, unrealized)
    }

    /// Total realized PnL for a closed position.
    pub fn realized_pnl(&self) -> DecimalAmount {
        match (self.entry_cost_basis, self.exit_proceeds) {
            (Some(cost), Some(proceeds)) => proceeds - cost,
            _ => DecimalAmount::ZERO,
        }
    }

    pub fn is_open(&self) -> bool {
        self.state == PositionState::BuyConfirmed
    }

    pub fn is_closed(&self) -> bool {
        self.state == PositionState::SellConfirmed
    }

    pub fn mark_scammed(&mut self) {
        self.state = PositionState::Scammed;
    }
}

/// Convert an Amount (U256 raw + decimals) to a DecimalAmount.
fn amount_to_decimal(amount: &Amount) -> DecimalAmount {
    let s = amount.raw.to_string();
    let mut dec = DecimalAmount::from_str_exact(&s).unwrap_or_default();
    if amount.decimals > 0 {
        let divisor = DecimalAmount::from(10i64.pow(amount.decimals as u32));
        dec = dec / divisor;
    }
    dec
}
