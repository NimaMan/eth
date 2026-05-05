use crate::ids::{BlockNumber, PoolAddress, TokenAddress, TxHash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskKind {
    LiquidityRemoval,
    TaxChange,
    Honeypot,
    TradingDisabled,
    ScamConfirmed,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RiskEvent {
    pub kind: RiskKind,
    pub severity: RiskSeverity,
    pub token_address: TokenAddress,
    pub pool_address: Option<PoolAddress>,
    pub pending_tx_hash: Option<TxHash>,
    pub observed_block: Option<BlockNumber>,
    pub message: String,
}
