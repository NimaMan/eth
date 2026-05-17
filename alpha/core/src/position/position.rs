use crate::{
    amount::{Amount, DecimalAmount},
    error::{AlphaCoreError, Result},
    execution::{ExecutionReport, ExecutionStatus},
    ids::{
        BlockNumber, OrderId, PoolAddress, PortfolioId, PositionId, StrategyName, TokenAddress,
        TradeId, WalletId,
    },
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
    #[serde(default = "legacy_trade_id")]
    pub trade_id: TradeId,
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
    /// Human-scale token amount received from the buy.
    /// Kept for reporting and strategy thresholds.
    pub entry_token_amount: Option<DecimalAmount>,
    /// Exact raw token amount received from the buy.
    /// Preferred for follow-up sell simulation because it preserves the token
    /// decimals and avoids reconstructing raw amounts from display values.
    #[serde(default)]
    pub entry_token_raw_amount: Option<Amount>,
    /// Block number at which the buy was confirmed.
    /// Used for time-based exits (e.g., max hold duration).
    #[serde(default)]
    pub entry_block: Option<BlockNumber>,
    /// Block number at which the sell was confirmed.
    #[serde(default)]
    pub exit_block: Option<BlockNumber>,
    /// Total ETH spent on execution gas for this position.
    #[serde(default)]
    pub gas_cost_eth: DecimalAmount,
    /// True if the pool was drained/scammed while position was open.
    /// Used for honest baseline PnL even when no exit is attempted.
    #[serde(default)]
    pub drained: bool,
    /// Last sell failure seen for this position, if any.
    #[serde(default)]
    pub exit_failure_reason: Option<String>,
    /// Number of failed sell reports seen for this position.
    #[serde(default)]
    pub exit_failure_count: u32,
    /// Block number of the latest failed sell report, if known.
    #[serde(default)]
    pub last_exit_failure_block: Option<BlockNumber>,
    /// False when the latest sell failure is simulator infrastructure rather
    /// than a retryable chain outcome.
    #[serde(default = "default_exit_retryable")]
    pub exit_retryable: bool,
}

impl Position {
    pub fn new(id: PositionId, key: PositionKey) -> Self {
        let trade_id = TradeId(id.0.clone());
        Self::with_trade_id(id, trade_id, key)
    }

    pub fn with_trade_id(id: PositionId, trade_id: TradeId, key: PositionKey) -> Self {
        Self {
            id,
            trade_id,
            key,
            state: PositionState::Init,
            entry_order_id: None,
            exit_order_id: None,
            entry_cost_basis: None,
            exit_proceeds: None,
            entry_price: None,
            entry_token_amount: None,
            entry_token_raw_amount: None,
            entry_block: None,
            exit_block: None,
            gas_cost_eth: DecimalAmount::ZERO,
            drained: false,
            exit_failure_reason: None,
            exit_failure_count: 0,
            last_exit_failure_block: None,
            exit_retryable: true,
        }
    }

    pub fn mark_drained(&mut self) {
        self.drained = true;
    }

    pub fn mark_intent_created(&mut self, side: OrderSide) -> Result<()> {
        match (&self.state, side) {
            (PositionState::Init, OrderSide::Buy) => {
                self.state = PositionState::BuyIntentCreated;
                Ok(())
            }
            (
                PositionState::BuyConfirmed
                | PositionState::SellFailed
                | PositionState::SellCancelled,
                OrderSide::Sell,
            ) if self.exit_retryable => {
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
        if let Some(gas_cost) = &report.gas_cost {
            self.gas_cost_eth += gas_cost.to_decimal();
        }
        match report.status {
            ExecutionStatus::Confirmed => self.apply_confirmed_report(report, fill_price),
            ExecutionStatus::Failed => self.apply_failed_report(report),
            ExecutionStatus::Cancelled => self.apply_cancelled_report(report),
            ExecutionStatus::Submitted | ExecutionStatus::Pending => Ok(()),
        }
    }

    fn apply_failed_report(&mut self, report: &ExecutionReport) -> Result<()> {
        if self.entry_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::BuySubmitted
        {
            self.state = PositionState::BuyFailed;
            return Ok(());
        }

        if self.exit_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::SellSubmitted
        {
            self.state = PositionState::SellFailed;
            self.exit_failure_reason = report.error.clone();
            self.exit_failure_count = self.exit_failure_count.saturating_add(1);
            self.last_exit_failure_block = report.block_number;
            self.exit_retryable = report
                .error
                .as_deref()
                .map(is_retryable_exit_failure)
                .unwrap_or(true);
            return Ok(());
        }

        Err(AlphaCoreError::InvalidPositionTransition(format!(
            "failed report {:?} does not match position {:?}",
            report.order_id, self.state
        )))
    }

    fn apply_cancelled_report(&mut self, report: &ExecutionReport) -> Result<()> {
        if self.entry_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::BuySubmitted
        {
            self.state = PositionState::BuyCancelled;
            return Ok(());
        }

        if self.exit_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::SellSubmitted
        {
            self.state = PositionState::SellCancelled;
            self.exit_failure_reason = None;
            self.exit_retryable = true;
            return Ok(());
        }

        self.state = PositionState::Cancelled;
        Ok(())
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
            self.entry_token_amount = report
                .token_amount
                .as_ref()
                .map(|amount| amount.to_decimal());
            self.entry_token_raw_amount = report.token_amount.clone();
            self.entry_block = report.block_number;
            self.exit_failure_reason = None;
            self.exit_retryable = true;
            return Ok(());
        }

        if self.exit_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::SellSubmitted
        {
            self.state = PositionState::SellConfirmed;
            if let Some(amount) = &report.filled_amount {
                self.exit_proceeds = Some(amount_to_decimal(amount));
            }
            self.exit_block = report.block_number;
            self.exit_failure_reason = None;
            self.exit_retryable = true;
            return Ok(());
        }

        Err(AlphaCoreError::InvalidPositionTransition(format!(
            "confirmed report {:?} does not match position {:?}",
            report.order_id, self.state
        )))
    }

    /// Total realized PnL for a closed position.
    pub fn realized_pnl(&self) -> DecimalAmount {
        match (self.entry_cost_basis, self.exit_proceeds) {
            (Some(cost), Some(proceeds)) => proceeds - cost - self.gas_cost_eth,
            _ => -self.gas_cost_eth,
        }
    }

    pub fn is_open(&self) -> bool {
        self.has_exposure()
    }

    pub fn has_exposure(&self) -> bool {
        self.state.has_exposure()
    }

    pub fn can_submit_exit(&self) -> bool {
        self.state.can_submit_exit() && self.exit_retryable
    }

    pub fn is_closed(&self) -> bool {
        self.state == PositionState::SellConfirmed
    }

    pub fn mark_scammed(&mut self) {
        self.state = PositionState::Scammed;
    }
}

fn legacy_trade_id() -> TradeId {
    TradeId("legacy-unset".to_string())
}

fn default_exit_retryable() -> bool {
    true
}

fn is_retryable_exit_failure(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    !(normalized.contains("unsupported balance storage layout")
        || normalized.contains("unable to inject synthetic erc20 balance"))
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
