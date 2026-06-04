//! Strategy-agnostic entry rules.
//!
//! Entry rules filter pools before strategy-specific logic runs.
//! They are evaluated in `on_market_event` and return `Hold` when a pool
//! should not be considered for entry.
//!
//! `buy_once` ("buy each eligible pool at most once") is the engine's default
//! entry rule — it is the nature of the engine itself, not a per-strategy knob.

pub mod buy_once;
pub mod eligibility;
pub mod init_policy;
pub mod mempool_entry;
