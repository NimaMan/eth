use serde::{Deserialize, Serialize};

use crate::pools::state::evidence::EvidenceRef;
use crate::pools::state::tracks::roles::CounterpartyRole;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PnlCounterpartyRole {
    UserTrader,
    RouterPassThrough,
    Pool,
    WethBridge,
    TokenContract,
    LpController,
    Unknown,
}

impl From<PnlCounterpartyRole> for CounterpartyRole {
    fn from(role: PnlCounterpartyRole) -> Self {
        match role {
            PnlCounterpartyRole::UserTrader => Self::UserTrader,
            PnlCounterpartyRole::RouterPassThrough => Self::RouterPassThrough,
            PnlCounterpartyRole::Pool => Self::Pool,
            PnlCounterpartyRole::WethBridge => Self::WethBridge,
            PnlCounterpartyRole::TokenContract => Self::TokenContract,
            PnlCounterpartyRole::LpController => Self::LpController,
            PnlCounterpartyRole::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PnlRoleObservation {
    pub tx_hash: Option<String>,
    pub address: Option<String>,
    pub role: PnlCounterpartyRole,
    pub value_delta_denom: Option<f64>,
    pub token_delta: Option<f64>,
    pub evidence: EvidenceRef,
}
