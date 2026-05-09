//! Contract-level evidence for tracked tokens.

pub mod erc20;
pub mod report;

pub use erc20::{analyze_erc20_token, Erc20ContractAnalyzer};
pub use report::{
    AuthoritySurfaceReport, BehaviorFlagKind, ContractAnalysisReport, ContractEvidence,
    ContractEvidenceSource, ContractFieldStatus, ContractSeverity, Erc20InterfaceQuality,
    Erc20InterfaceReport, PoolSurfaceReport, SupplySurfaceReport, TokenMetadataQualityReport,
    TransferSurfaceReport,
};
