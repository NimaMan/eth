//! ERC-20 token models, metadata, and snapshots.

mod address;
pub mod token;
mod types;

pub use token::ERC20Token;
pub use types::{ERC20TokenMetadata, PoolStateSnapshot, TokenLifecycleState, TokenSummary};
