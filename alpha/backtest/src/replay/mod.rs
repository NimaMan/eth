pub mod blocks;
pub mod risk_atlas;
pub mod strategy_observations;

use crate::strategy_suites::BacktestStrategySpec;

pub use risk_atlas::load_events_from_risk_atlas;
pub use strategy_observations::load_events_from_observations;

pub fn common_nonempty_allowed_protocols(strategy_specs: &[BacktestStrategySpec]) -> Vec<String> {
    let Some(first_spec) = strategy_specs.first() else {
        return Vec::new();
    };
    if first_spec.allowed_protocols.is_empty() {
        return Vec::new();
    }
    if strategy_specs
        .iter()
        .all(|spec| spec.allowed_protocols == first_spec.allowed_protocols)
    {
        first_spec.allowed_protocols.clone()
    } else {
        Vec::new()
    }
}
