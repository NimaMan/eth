//! Live strategy set rules.
//!
//! These modules define reusable live-only strategy configurations. Runtime
//! binaries select and persist these specs, while the actual entry/exit rules
//! stay in `core::rules::entry` and `core::rules::exit`.

mod registry;
mod spec;
pub mod strategy_sets;

pub use registry::strategy_set_specs;

pub use spec::{
    default_strategy_spec, observation_strategy_name, LiveEntryInitPolicySpec, LiveStrategySpec,
    LiveStrategySpecOptions, STRATEGY_RUNTIME,
};
