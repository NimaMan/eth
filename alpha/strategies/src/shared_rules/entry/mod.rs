//! Strategy-agnostic entry rules.
//!
//! Entry rules filter pools before strategy-specific logic runs.
//! They are evaluated in `on_market_event` and return `Hold` when a pool
//! should not be considered for entry.

pub mod eligibility;
