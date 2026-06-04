//! Core strategy engine.
//!
//! `core` is the generic engine every strategy composes. It owns:
//! - the [`StrategyEngine`] (entry/exit/hold decision loop),
//! - the [`StrategyConfig`] knobs that parameterise that loop,
//! - the engine [`state::EngineState`] (bought pools, active-hold counters, restored bankroll),
//! - the shared, strategy-agnostic [`rules`] (entry eligibility/init-policy, the
//!   `buy_eligible_pool_once` default entry rule, and the fundamental exit rules:
//!   mined + mempool liquidity-removal, scam, tax, lp-approval),
//! - a generic [`live`] wrapper,
//! - the canonical resolved [`dispatch::LiveStrategySpec`] and its registry.
//!
//! A strategy is then just a named config preset composing this engine; it adds
//! no trait-level logic.

mod config;
pub mod dispatch;
mod engine;
pub mod live;
pub mod rule;
mod state;

pub mod rules;

pub use config::StrategyConfig;
pub use engine::StrategyEngine;
pub use live::{LiveStrategyConfig, LiveStrategyEngine};
pub use state::RestoredEntryBankroll;
