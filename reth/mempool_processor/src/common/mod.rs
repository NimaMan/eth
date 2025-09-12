/// Common utilities shared across the mempool processor
/// 
/// This module contains shared utilities used throughout the system:
/// - address: Ethereum address formatting and checksumming
/// - config: System configuration structures
/// - convert: IPC to CallRequest conversion for simulations

pub mod config;
pub mod convert;
pub use config::MempoolProcessorConfig;
