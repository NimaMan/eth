/// Common utilities shared across the mempool processor
/// 
/// This module contains shared utilities used throughout the system:
/// - address: Ethereum address formatting and checksumming
/// - config: System configuration structures
/// - convert: IPC to CallRequest conversion for simulations

pub mod address;
pub mod config;
pub mod convert;

pub use address::{normalize_address, to_checksum_address, checksum_address};
pub use config::MempoolProcessorConfig;