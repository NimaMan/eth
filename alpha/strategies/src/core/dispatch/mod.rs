//! Live dispatch compatibility surface.
//!
//! The canonical resolved spec ([`crate::core::spec`]) and the registry
//! ([`crate::core::registry`]) now live directly under `core`. This module
//! re-exports them under the historical `dispatch` path and owns the named
//! diagnostic `strategy_sets`. Runtime binaries select and persist these specs;
//! the actual entry/exit rules stay in `core::rules::entry` and
//! `core::rules::exit`.

pub mod strategy_sets;

pub use crate::core::registry::strategy_set_specs;

pub use crate::core::spec::{
    default_strategy_spec, observation_strategy_name, LiveEntryInitPolicySpec, LiveStrategySpec,
    LiveStrategySpecOptions, StrategyId, StrategySpec, STRATEGY_RUNTIME,
};
