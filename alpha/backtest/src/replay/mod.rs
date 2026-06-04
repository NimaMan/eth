pub mod blocks;
pub mod risk_atlas;
pub mod strategy_observations;
pub mod token_state;

use crate::strategy_suites::BacktestStrategySpec;

pub use blocks::sort_events_by_block;
pub use risk_atlas::load_events_from_risk_atlas;
pub use strategy_observations::load_events_from_observations;
pub use token_state::load_events_from_token_state;

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
