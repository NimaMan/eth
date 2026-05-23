use crate::ids::{BlockNumber, PoolAddress, TokenAddress, TxHash};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RISK_SOURCE_MEMPOOL_SIGNAL: &str = "mempool_signal";
pub const RISK_SOURCE_HISTORICAL_MEMPOOL_SIGNAL: &str = "historical_mempool_signal";
pub const RISK_SOURCE_RISK_ATLAS_MINED_CHAIN: &str = "risk_atlas_mined_chain";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskKind {
    LiquidityRemoval,
    MempoolLiquidityRemoval,
    TaxChange,
    Honeypot,
    TradingDisabled,
    TradingEnabled,
    LpApproval,
    ScamConfirmed,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskEvent {
    pub kind: RiskKind,
    pub severity: RiskSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub token_address: TokenAddress,
    pub pool_address: Option<PoolAddress>,
    pub pending_tx_hash: Option<TxHash>,
    pub observed_block: Option<BlockNumber>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Value>,
}
