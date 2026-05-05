use crate::{
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
}

impl Position {
    pub fn new(id: PositionId, key: PositionKey) -> Self {
        Self {
            id,
            key,
            state: PositionState::Init,
            entry_order_id: None,
            exit_order_id: None,
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
        match report.status {
            ExecutionStatus::Confirmed => self.apply_confirmed_report(report),
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

    fn apply_confirmed_report(&mut self, report: &ExecutionReport) -> Result<()> {
        if self.entry_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::BuySubmitted
        {
            self.state = PositionState::BuyConfirmed;
            return Ok(());
        }

        if self.exit_order_id.as_ref() == Some(&report.order_id)
            && self.state == PositionState::SellSubmitted
        {
            self.state = PositionState::SellConfirmed;
            return Ok(());
        }

        Err(AlphaCoreError::InvalidPositionTransition(format!(
            "confirmed report {:?} does not match position {:?}",
            report.order_id, self.state
        )))
    }

    pub fn mark_scammed(&mut self) {
        self.state = PositionState::Scammed;
    }
}
