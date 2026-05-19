use eth_alpha_core::{
    ids::BlockNumber, market::PoolSnapshot, order::OrderIntent, position::Position,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::TxPrepRequestContext;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlannerTxContext {
    pub tx: TxPrepRequestContext,
    pub current_block: BlockNumber,
    pub deadline_unix_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LivePrioritySellPlannerInput {
    pub context: PlannerTxContext,
    pub intent: OrderIntent,
    pub position: Position,
    pub pool: PoolSnapshot,
    #[serde(default)]
    pub min_output_amount: Option<String>,
    #[serde(default)]
    pub source_metadata: Value,
}
