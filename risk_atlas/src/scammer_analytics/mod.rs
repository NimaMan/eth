//! Actor-centric scammer analytics for Risk Atlas.
//!
//! `scam_analytics` stays token/pool-centric. This module follows the actor:
//! creator, funder, remover, forwarder, final sink, and related token campaigns.

pub mod analyzer;
pub mod model;
pub mod writer;

pub use analyzer::ScammerCaseAnalyzer;
pub use model::{
    ForwarderTrace, KeyTransactionConfig, ScammerCaseConfig, ScammerCaseReport, StagedTokenConfig,
    TokenMovementEvidence, TransactionEvidence,
};
