use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractEvidenceSource {
    Metadata,
    Supply,
    Authority,
    Transfer,
    Pools,
    Status,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractFieldStatus {
    Present,
    Empty,
    Invalid,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Erc20InterfaceQuality {
    Complete,
    MetadataIncomplete,
    NonStandard,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorFlagKind {
    MetadataIncomplete,
    InvalidMetadata,
    HiddenMintEvidence,
    RawTradingEventWithoutPoolTrading,
    ActiveOwnerControl,
    CannotSellPool,
    PoolScamEvidence,
}

impl BehaviorFlagKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MetadataIncomplete => "metadata_incomplete",
            Self::InvalidMetadata => "invalid_metadata",
            Self::HiddenMintEvidence => "hidden_mint_evidence",
            Self::RawTradingEventWithoutPoolTrading => "raw_trading_event_without_pool_trading",
            Self::ActiveOwnerControl => "active_owner_control",
            Self::CannotSellPool => "cannot_sell_pool",
            Self::PoolScamEvidence => "pool_scam_evidence",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContractEvidence {
    pub kind: BehaviorFlagKind,
    pub severity: ContractSeverity,
    pub source: ContractEvidenceSource,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
}

impl ContractEvidence {
    pub fn new(
        kind: BehaviorFlagKind,
        severity: ContractSeverity,
        source: ContractEvidenceSource,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            severity,
            source,
            message: message.into(),
            block_number: None,
            tx_hash: None,
        }
    }

    pub fn with_block(mut self, block_number: Option<u64>) -> Self {
        self.block_number = block_number;
        self
    }

    pub fn with_tx_hash(mut self, tx_hash: Option<String>) -> Self {
        self.tx_hash = tx_hash;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Erc20InterfaceReport {
    pub quality: Erc20InterfaceQuality,
    pub metadata_complete: bool,
    pub observed_transfers: bool,
    pub observed_approvals: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenMetadataQualityReport {
    pub name: ContractFieldStatus,
    pub symbol: ContractFieldStatus,
    pub decimals: ContractFieldStatus,
    pub total_supply: ContractFieldStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SupplySurfaceReport {
    pub declared_total_supply_raw: String,
    pub declared_total_supply_scaled: Option<f64>,
    pub minted_from_transfers: f64,
    pub minted_to_declared_ratio: Option<f64>,
    pub hidden_mint_detected: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuthoritySurfaceReport {
    pub current_owner: Option<String>,
    pub ownership_renounced: bool,
    pub control_address_count: usize,
    pub owner_event_count: usize,
    pub has_active_owner_control: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransferSurfaceReport {
    pub transfer_tx_count: usize,
    pub approval_count: usize,
    pub approved_spender_count: usize,
    pub unique_address_count: usize,
    pub bribe_total_eth: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoolSurfaceReport {
    pub pool_count: usize,
    pub protocols: Vec<String>,
    pub pool_count_by_protocol: BTreeMap<String, usize>,
    pub trading_pool_count: usize,
    pub cannot_sell_pool_count: usize,
    pub scam_pool_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContractAnalysisReport {
    pub address: String,
    pub block_number: Option<u64>,
    pub timestamp: Option<u64>,
    pub interface: Erc20InterfaceReport,
    pub metadata: TokenMetadataQualityReport,
    pub supply: SupplySurfaceReport,
    pub authority: AuthoritySurfaceReport,
    pub transfer: TransferSurfaceReport,
    pub pools: PoolSurfaceReport,
    #[serde(default)]
    pub behavior_flags: Vec<BehaviorFlagKind>,
    #[serde(default)]
    pub evidence: Vec<ContractEvidence>,
}
