/// Python bindings for ChainQuery module
/// 
/// This module provides a structured organization of blockchain query functionality:
/// - account.rs: Account-related queries (balance, nonce, code)
/// - token.rs: ERC20 token queries (balance, supply, metadata) 
/// - storage.rs: Direct contract storage access
/// - block.rs: Block information queries
/// - analysis.rs: High-level analysis methods
/// - entities.rs: Entity analysis (stablecoins, CEX, ETF)
/// - main.rs: Main ChainQuery class that orchestrates all modules

pub mod account;
pub mod token;
pub mod storage;
pub mod block;
pub mod analysis;
pub mod entities;
pub mod time_utils;
pub mod main;

// Re-export the main ChainQuery class
pub use main::PyChainQuery;