#![allow(ambiguous_glob_reexports)]
//! AMM-related chain queries (read-only)
//!
//! This module exposes helpers to read AMM pool state from the local Reth DB
//! via TxSimulator. It avoids RPC and provides deterministic historical reads.

mod balancer;
mod curve;
mod liquidity;
mod uniswap_v2;
mod uniswap_v3;
mod uniswap_v4;

pub use balancer::*;
pub use curve::*;
pub use liquidity::*;
pub use uniswap_v2::*;
pub use uniswap_v3::*;
pub use uniswap_v4::*;
