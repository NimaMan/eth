use crate::{
    amount::DecimalAmount,
    ids::{BlockNumber, PositionId},
    position::PositionState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub position_id: PositionId,
    pub state: PositionState,
    pub block_number: BlockNumber,
    pub current_value_eth: DecimalAmount,
    pub realized_profit_eth: DecimalAmount,
    pub unrealized_profit_eth: DecimalAmount,
    pub roi: DecimalAmount,
}
