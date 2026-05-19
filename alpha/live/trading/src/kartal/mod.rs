//! Kartal ETH transaction executor HTTP client surface.
//!
//! This module is for live-trading integration and calibration. It keeps the
//! HTTP/status/journal contract close to the existing direct-raw request types.

pub mod client;
pub mod policy_journal;
pub mod wire;

pub use client::{KartalClient, KartalClientConfig, KartalClientError, KartalServerError};
pub use policy_journal::{KartalPolicyDecision, KartalPolicyDecisionList};
pub use wire::{
    KartalDailySpendStatus, KartalEthTxExecutorStatus, KartalEthTxPolicyStatus,
    KartalStatusBroadcastMode, KartalSubmitDirectRawResult,
};
