//! ETH Risk Atlas read model and ingestion boundary.
//!
//! Source token/pool features should be produced by
//! `eth_token::token_analytics`; this crate owns the durable Risk Atlas DB
//! contract, aggregate risk views, and page-ready story data. Lab domains such
//! as scam analytics and modeling live under this module as consumers and
//! calibration inputs, not as owners of the atlas schema.

pub mod api;
pub mod atlas;
pub mod config;
pub mod db;
pub mod ingest;
pub mod scammer_analytics;

pub use api::RiskAtlasPageView;
pub use config::RiskAtlasConfig;
pub use db::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, EventEvidenceRow,
    ModelReadinessItem, NumericStat, ObservationRow, PoolEligibilityRow, ReviewExample,
    RiskAtlasReader, RiskAtlasRun, RiskAtlasWriter,
};
