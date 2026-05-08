//! Labels and confidence metadata for network nodes.

use serde::{Deserialize, Serialize};

use crate::network::model::evidence::ObservationRange;

/// Coarse confidence bucket for labels and inferred relationships.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    Observed,
    High,
    Medium,
    Low,
    Unknown,
}

impl Default for ConfidenceLevel {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Confidence metadata kept separate from the label itself.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NetworkConfidence {
    pub level: ConfidenceLevel,
    pub score: Option<f64>,
    pub reason: Option<String>,
}

impl NetworkConfidence {
    pub fn observed(reason: impl Into<String>) -> Self {
        Self {
            level: ConfidenceLevel::Observed,
            score: Some(1.0),
            reason: Some(reason.into()),
        }
    }

    pub fn inferred(level: ConfidenceLevel, score: Option<f64>, reason: impl Into<String>) -> Self {
        Self {
            level,
            score,
            reason: Some(reason.into()),
        }
    }
}

/// Source of a node label.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkLabelSource {
    ChainMetadata,
    KnownAddressBook,
    TokenState,
    PoolState,
    TransferFlow,
    Heuristic,
    Manual,
    Unknown,
}

impl Default for NetworkLabelSource {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Role or classification attached to a node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum NetworkLabelKind {
    Wallet,
    Contract,
    TokenContract,
    Pool,
    Router,
    Cex,
    Bridge,
    ZeroAddress,
    BurnAddress,
    Creator,
    Owner,
    PendingOwner,
    Admin,
    ProxyAdmin,
    TaxWallet,
    ControlActor,
    LpHolder,
    LpApprover,
    LiquidityActor,
    Funder,
    DepositAddress,
    Intermediary,
    Supernode,
    TimeWindow,
    Unknown,
    Custom(String),
}

impl NetworkLabelKind {
    pub fn stable_key(&self) -> String {
        match self {
            Self::Wallet => "wallet",
            Self::Contract => "contract",
            Self::TokenContract => "token_contract",
            Self::Pool => "pool",
            Self::Router => "router",
            Self::Cex => "cex",
            Self::Bridge => "bridge",
            Self::ZeroAddress => "zero_address",
            Self::BurnAddress => "burn_address",
            Self::Creator => "creator",
            Self::Owner => "owner",
            Self::PendingOwner => "pending_owner",
            Self::Admin => "admin",
            Self::ProxyAdmin => "proxy_admin",
            Self::TaxWallet => "tax_wallet",
            Self::ControlActor => "control_actor",
            Self::LpHolder => "lp_holder",
            Self::LpApprover => "lp_approver",
            Self::LiquidityActor => "liquidity_actor",
            Self::Funder => "funder",
            Self::DepositAddress => "deposit_address",
            Self::Intermediary => "intermediary",
            Self::Supernode => "supernode",
            Self::TimeWindow => "time_window",
            Self::Unknown => "unknown",
            Self::Custom(value) => value.as_str(),
        }
        .to_string()
    }
}

/// Label attached to a network node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkLabel {
    pub kind: NetworkLabelKind,
    pub value: Option<String>,
    pub source: NetworkLabelSource,
    pub confidence: NetworkConfidence,
    pub observed: ObservationRange,
}

impl NetworkLabel {
    pub fn observed(kind: NetworkLabelKind, source: NetworkLabelSource) -> Self {
        let reason = format!("observed label {}", kind.stable_key());
        Self {
            kind,
            value: None,
            source,
            confidence: NetworkConfidence::observed(reason),
            observed: ObservationRange::default(),
        }
    }

    pub fn inferred(
        kind: NetworkLabelKind,
        source: NetworkLabelSource,
        confidence: NetworkConfidence,
    ) -> Self {
        Self {
            kind,
            value: None,
            source,
            confidence,
            observed: ObservationRange::default(),
        }
    }
}
