//! Shared rules usable by any strategy.
//!
//! Exit rules in this module are strategy-agnostic: they inspect the portfolio
//! and risk event, then recommend `Exit` when a position is threatened.
//!
//! Strategies compose these rules in `on_risk_event` and decide whether to act.

pub mod entry;
pub mod exit;
