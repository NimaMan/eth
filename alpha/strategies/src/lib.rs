//! Strategy implementations for the alpha engine.
//!
//! Structure: `core` is the generic engine (config, state, shared rules, live
//! wrapper, and the resolved-spec registry) that every strategy composes.
//! Each strategy under `strategies` (e.g. `alpha11`) is a named config preset
//! plus a factory over that core engine; it adds no trait-level logic.

pub mod alpha11;
pub mod core;
mod market_tracker;

pub use core::dispatch as shared_rules_live;
pub use core::{
    config_from_spec, resolve, strategy_set_specs, EngineRuntimeInputs, LiveStrategyConfig,
    LiveStrategyEngine, RestoredEntryBankroll, StrategyConfig, StrategyEngine, StrategyId,
    StrategySpec,
};

pub use alpha11::{
    Alpha11Config, Alpha11Strategy, LiveAlpha11Config, LiveAlpha11Strategy,
    HOLD15_STRATEGY_NAME as ALPHA11_HOLD15_STRATEGY_NAME,
    HOLD16_ALL_POOLS_STRATEGY_NAME as ALPHA11_HOLD16_ALL_POOLS_STRATEGY_NAME,
    HOLD16_STRATEGY_NAME as ALPHA11_HOLD16_STRATEGY_NAME,
    HOLD3_VALIDATION_STRATEGY_NAME as ALPHA11_HOLD3_VALIDATION_STRATEGY_NAME,
    HOLD_SWEEP_SET_NAME as ALPHA11_HOLD_SWEEP_SET_NAME, STRATEGY_IMPL as ALPHA11_STRATEGY_IMPL,
};
pub use market_tracker::{MarketTrackerConfig, MarketTrackerStrategy};

/// Compatibility surface for consumers that import the shared rules / live
/// dispatch via `eth_strategies::shared_rules::...`. The rules themselves now
/// live in `core::rules`; the resolved-spec types live in `core::dispatch`.
pub mod shared_rules {
    pub use crate::core::dispatch as live;
    pub use crate::core::rules::{
        entry, exit, lp_approval, lp_approval_warning_exit,
    };
}
