//! Live strategy suite rules.
//!
//! These modules define reusable live-only strategy configurations. Runtime
//! binaries select and persist these specs, while the actual entry/exit rules
//! stay in `shared_rules::entry` and `shared_rules::exit`.

pub mod live_mempool_liquidity_removal_exit;
