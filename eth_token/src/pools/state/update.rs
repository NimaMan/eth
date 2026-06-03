use serde::{Deserialize, Serialize};

use crate::pools::state::observations::{
    CustodyObservation, LiquidityObservation, PnlRoleObservation, PoolEventObservation,
    TradeSimulationObservation,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PoolStateUpdate {
    PoolEvent {
        observation: PoolEventObservation,
    },
    Simulation {
        observation: TradeSimulationObservation,
    },
    Custody {
        observation: CustodyObservation,
    },
    PnlRole {
        observation: PnlRoleObservation,
    },
    Liquidity {
        observation: LiquidityObservation,
    },
}
