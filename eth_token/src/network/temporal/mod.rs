//! Temporal analysis of network activity.
//!
//! Analyzes ordering patterns within and across blocks:
//! fund-then-buy, buy-then-transfer, remove-then-dump, etc.

pub mod ordering;
pub mod windows;
