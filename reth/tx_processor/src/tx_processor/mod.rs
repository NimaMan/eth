//! High-Performance Transaction Processor
//! 
//! Fast transaction processing using direct Reth database access with optional REVM simulation.
//! 
//! ## Architecture
//! 
//! **Phase 1: Direct DB Access (0.351ms)**
//! - Get signed transaction + receipt + logs from Reth database
//! - Parse ERC20 transfers and contract events  
//! - Covers 90% of transaction analysis needs
//! 
//! **Phase 2: Optional REVM Simulation (additional 1-2ms)**
//! - Use CallTracer to extract internal ETH transfers
//! - Only when internal transfers are specifically needed
//! 
//! ## Usage
//! 
//! ```rust
//! // Fast path (most common)
//! let processor = DirectDbProcessor::new(config)?;
//! let result = processor.process_transaction(tx_hash)?; // 0.351ms
//! 
//! // With internal transfers (when needed)
//! let config = Config { enable_internal_transfers: true, ..Default::default() };
//! let result = processor.process_transaction_with_simulation(tx_hash)?; // ~2ms
//! ```

pub mod types;
pub mod direct_db_processor;

// Re-export main types
pub use direct_db_processor::{DirectDbProcessor, DirectDbProcessorConfig};
pub use types::{ProcessedTransaction, InternalTransfer, StateChange, ProcessedLog};