//! AMM-related chain queries (read-only)
//!
//! This module exposes helpers to read AMM pool state from the local Reth DB
//! via TxSimulator. It avoids RPC and provides deterministic historical reads.

mod uniswap_v2;

pub use uniswap_v2::*;

