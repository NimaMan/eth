//! Diagnostic strategies: named configs composing the core engine.
//!
//! Each is a named config (one or more complete resolved [`crate::core::spec::StrategySpec`]
//! rows) used to quantify a single exit-rule's effect. They add no trait-level
//! logic — they only flip core engine config — so they compose the same shared
//! core rules every strategy uses (incl. the fundamental mempool/mined
//! liquidity-removal, scam, tax, and LP-approval exits).

pub mod lp_approval_warning;
pub mod mempool_exit_diagnostics;
pub mod risk_atlas;
