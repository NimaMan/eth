//! Live strategy suite rules.
//!
//! These modules define reusable live-only strategy configurations. Runtime
//! binaries select and persist these specs, while the actual entry/exit rules
//! stay in `shared_rules::entry` and `shared_rules::exit`.

mod spec;
pub mod suites;

pub use spec::{
    default_strategy_spec, observation_strategy_name, LiveStrategySpec, LiveStrategySpecOptions,
    STRATEGY_RUNTIME,
};
pub use suites::suite_specs;
