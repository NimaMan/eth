/// Ethereum Mempool Processor Module
/// 
/// This module contains components for processing Ethereum mempool transactions.
/// It's structured to allow for better testing and separation of concerns.

// Our component modules
pub mod types;
pub mod fetcher;
pub mod devp2p_client; // DevP2P peer-to-peer transaction fetching
pub mod processor;
pub mod pools;
pub mod db_logger;

// Re-export key types and functions
pub use types::*;
pub use fetcher::*;
pub use processor::*;
pub use db_logger::*;

// Re-export tx_simulator functionality 
pub use crate::tx_simulator::*;