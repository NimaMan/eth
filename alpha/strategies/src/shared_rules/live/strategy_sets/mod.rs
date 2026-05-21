//! Named live strategy sets.
//!
//! A strategy set expands one CLI/config name into one or more concrete
//! `LiveStrategySpec` rows. Each submodule owns one named set and documents why
//! it exists next to the Rust implementation.

pub mod alpha11_hold_sweep;
pub mod lp_approval_warning;
pub mod mempool_exit_diagnostics;
pub mod risk_atlas;
