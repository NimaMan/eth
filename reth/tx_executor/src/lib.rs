//! Gas-first Ethereum transaction execution core.
//!
//! This crate intentionally starts from prepared direct transactions. Route selection, pool
//! discovery, quoting, slippage math, and strategy policy should happen before a request reaches
//! this executor.

pub mod broadcast;
pub mod config;
pub mod error;
pub mod executor;
pub mod gas;
pub mod nonce;
pub mod position;
pub mod repository;
pub mod request;
pub mod service;
pub mod signer;
pub mod types;
pub mod validation;

pub use config::{BroadcastMode, EthTxExecutorConfig};
pub use error::{EthTxExecutorError, Result};
pub use executor::EthTxExecutor;
pub use request::{BribeRequest, DirectRawTransactionRequest, SimulationReference};
pub use types::{ExecutionStatus, SubmitDirectRawResult};
