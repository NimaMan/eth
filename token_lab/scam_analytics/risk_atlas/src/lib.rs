//! Risk Atlas read model for scam analytics.
//!
//! This crate is lab-owned. Source token/pool features should be produced by
//! `eth_token::token_analytics`; this crate stores aggregate scam/risk views
//! and page-ready story data.

pub mod api;
pub mod atlas;
pub mod config;
pub mod db;
pub mod ingest;

pub use api::RiskAtlasPageView;
pub use config::RiskAtlasConfig;
pub use db::{
    ActiveTargetSummary, DecisionQuestion, DistributionBucket, ModelReadinessItem, NumericStat,
    ObservationRow, PoolEligibilityRow, ReviewExample, RiskAtlasReader, RiskAtlasRun,
    RiskAtlasWriter,
};
