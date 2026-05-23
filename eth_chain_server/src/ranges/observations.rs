use eth_token::token_analytics::collect_current_observations;
use eth_token::tracking::TokenBlockUpdateReport;

use crate::ranges::RangeIndexState;

pub fn collect_observations(state: &mut RangeIndexState, report: &TokenBlockUpdateReport) {
    let observations = collect_current_observations(
        &state.processor.registry,
        report,
        &mut state.active_observation_counts_by_pool,
    );
    state.observations.extend(observations);
}
