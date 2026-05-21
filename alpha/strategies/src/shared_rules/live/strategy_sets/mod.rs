//! Named live strategy sets.
//!
//! A strategy set expands one CLI/config name into one or more concrete
//! `LiveStrategySpec` rows. Shared diagnostic sets live here; product strategy
//! families keep their launch specs next to their strategy module.

pub mod lp_approval_warning;
pub mod mempool_exit_diagnostics;
pub mod risk_atlas;
