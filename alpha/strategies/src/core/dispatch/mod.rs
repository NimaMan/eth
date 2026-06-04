//! Live dispatch compatibility surface.
//!
//! The canonical resolved spec ([`crate::core::spec`]) and the registry
//! ([`crate::core::registry`]) live directly under `core`. This module
//! re-exports them under the historical `dispatch` path used by the live
//! runtime. The diagnostic strategies that used to live here now live under
//! [`crate::strategies::diagnostics`]. Runtime binaries select and persist
//! these specs; the actual entry/exit rules stay in `core::rules`.

pub use crate::core::registry::{resolve, strategy_set_specs};

pub use crate::core::spec::{
    observation_strategy_name, LiveEntryInitPolicySpec, LiveStrategySpec, LiveStrategySpecOptions,
    StrategyId, StrategySpec, STRATEGY_RUNTIME,
};
