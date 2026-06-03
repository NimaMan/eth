use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::state::evidence::EvidenceRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CounterpartyRole {
    UserTrader,
    RouterPassThrough,
    Pool,
    WethBridge,
    TokenContract,
    LpController,
    Unknown,
}

impl CounterpartyRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserTrader => "user_trader",
            Self::RouterPassThrough => "router_pass_through",
            Self::Pool => "pool",
            Self::WethBridge => "weth_bridge",
            Self::TokenContract => "token_contract",
            Self::LpController => "lp_controller",
            Self::Unknown => "unknown",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::UserTrader => "User trader",
            Self::RouterPassThrough => "Router pass-through",
            Self::Pool => "Pool",
            Self::WethBridge => "WETH bridge",
            Self::TokenContract => "Token contract",
            Self::LpController => "LP controller",
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CounterpartyRoleEntry {
    pub address: Option<String>,
    pub role: CounterpartyRole,
    pub label: String,
    pub evidence: Vec<EvidenceRef>,
}

impl CounterpartyRoleEntry {
    pub fn new(
        address: Option<String>,
        role: CounterpartyRole,
        evidence: Vec<EvidenceRef>,
    ) -> Self {
        Self {
            address,
            role,
            label: role.label().to_string(),
            evidence,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CounterpartyRolesTrack {
    pub roles: Vec<CounterpartyRoleEntry>,
}

impl CounterpartyRolesTrack {
    pub fn from_base_pool(base: &BasePool) -> Self {
        Self {
            roles: vec![CounterpartyRoleEntry::new(
                Some(base.identity.pool_address.clone()),
                CounterpartyRole::Pool,
                vec![EvidenceRef::base_projection()],
            )],
        }
    }

    pub fn has_role(&self, role: CounterpartyRole) -> bool {
        self.roles.iter().any(|entry| entry.role == role)
    }
}
