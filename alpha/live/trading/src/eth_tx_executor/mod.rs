//! ETH tx executor ETH transaction executor HTTP client surface.
//!
//! This module is for live-trading integration and calibration. It keeps the
//! HTTP/status/journal contract close to the existing direct-raw request types.

pub mod client;
pub mod policy_journal;
pub mod wire;

pub use client::{
    EthTxExecutorClient, EthTxExecutorClientConfig, EthTxExecutorClientError,
    EthTxExecutorServerError,
};
pub use policy_journal::{EthTxPolicyDecision, EthTxPolicyDecisionList};
pub use wire::{
    EthTxDailySpendStatus, EthTxExecutorBroadcastMode, EthTxExecutorStatus, EthTxPolicyStatus,
    EthTxSubmitDirectRawResult, EthTxSubmitTransactionRequest, EthTxSubmitTransactionResult,
};
