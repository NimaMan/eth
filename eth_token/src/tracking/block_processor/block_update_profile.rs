use std::time::Instant;

use crate::tracking::token_update_router::ProcessedTokenUpdateProfile;

use super::processor::BlockTokenProcessor;

const TOKEN_BLOCK_PROCESSOR_PROFILE_LOG_TARGET: &str = "token_block_processor_profile";

#[derive(Default)]
pub(in crate::tracking::block_processor) struct BlockTokenProcessorProfile {
    pub(in crate::tracking::block_processor) sort_us: u128,
    pub(in crate::tracking::block_processor) token_metadata_us: u128,
    pub(in crate::tracking::block_processor) router_us: u128,
    pub(in crate::tracking::block_processor) index_refresh_us: u128,
    pub(in crate::tracking::block_processor) network_update_us: u128,
    pub(in crate::tracking::block_processor) finalize_us: u128,
    pub(in crate::tracking::block_processor) skipped_transactions: usize,
    pub(in crate::tracking::block_processor) processing_error_transactions: usize,
    pub(in crate::tracking::block_processor) applicable_transactions: usize,
    pub(in crate::tracking::block_processor) applier: ProcessedTokenUpdateProfile,
}

pub(in crate::tracking::block_processor) fn log_block_token_processor_profile(
    processor: &BlockTokenProcessor,
    run_id: Option<&str>,
    block_number: u64,
    transaction_count: usize,
    processed_transaction_count: usize,
    created_token_count: usize,
    updated_token_count: usize,
    token_update_count: usize,
    transaction_error_count: usize,
    block_started: Instant,
    profile: &BlockTokenProcessorProfile,
) {
    let total_us = elapsed_micros(block_started);
    let applier = &profile.applier;
    tracing::info!(
        target: TOKEN_BLOCK_PROCESSOR_PROFILE_LOG_TARGET,
        run_id = run_id.unwrap_or(""),
        block_number,
        transaction_count,
        processed_transaction_count,
        applicable_transactions = profile.applicable_transactions,
        skipped_transactions = profile.skipped_transactions,
        processing_error_transactions = profile.processing_error_transactions,
        created_token_count,
        updated_token_count,
        token_update_count,
        transaction_error_count,
        registry_tokens = processor.registry.tokens.len(),
        indexed_tokens = processor.token_index.entries.len(),
        indexed_pools = processor.token_index.pool_to_token.len(),
        processed_blocks = processor.processed_blocks.len(),
        total_us,
        total_ms = micros_to_millis(total_us),
        sort_us = profile.sort_us,
        sort_ms = micros_to_millis(profile.sort_us),
        token_metadata_us = profile.token_metadata_us,
        token_metadata_ms = micros_to_millis(profile.token_metadata_us),
        router_us = profile.router_us,
        router_ms = micros_to_millis(profile.router_us),
        index_refresh_us = profile.index_refresh_us,
        index_refresh_ms = micros_to_millis(profile.index_refresh_us),
        network_update_us = profile.network_update_us,
        network_update_ms = micros_to_millis(profile.network_update_us),
        finalize_us = profile.finalize_us,
        finalize_ms = micros_to_millis(profile.finalize_us),
        applier_candidate_us = applier.candidate_us,
        applier_candidate_ms = micros_to_millis(applier.candidate_us),
        applier_token_state_us = applier.token_state_us,
        applier_token_state_ms = micros_to_millis(applier.token_state_us),
        applier_pool_discovery_us = applier.pool_discovery_us,
        applier_pool_discovery_ms = micros_to_millis(applier.pool_discovery_us),
        applier_pool_update_us = applier.pool_update_us,
        applier_pool_update_ms = micros_to_millis(applier.pool_update_us),
        applier_simulation_v2_us = applier.simulation_v2_us,
        applier_simulation_v2_ms = micros_to_millis(applier.simulation_v2_us),
        applier_simulation_v3_us = applier.simulation_v3_us,
        applier_simulation_v3_ms = micros_to_millis(applier.simulation_v3_us),
        applier_simulation_v4_us = applier.simulation_v4_us,
        applier_simulation_v4_ms = micros_to_millis(applier.simulation_v4_us),
        applier_report_us = applier.report_us,
        applier_report_ms = micros_to_millis(applier.report_us),
        applier_candidate_tx_count = applier.candidate_tx_count,
        applier_candidate_tokens = applier.candidate_tokens,
        applier_visited_tokens = applier.visited_tokens,
        applier_token_state_updates = applier.token_state_updates,
        applier_token_control_replays = applier.token_control_replays,
        applier_update_reports = applier.update_reports,
        simulation_v2_candidate_pools = applier.simulation_v2_candidate_pools,
        simulation_v3_candidate_pools = applier.simulation_v3_candidate_pools,
        simulation_v4_candidate_pools = applier.simulation_v4_candidate_pools,
        simulation_v2_current_block_pools = applier.simulation_v2_current_block_pools,
        simulation_v3_current_block_pools = applier.simulation_v3_current_block_pools,
        simulation_v4_current_block_pools = applier.simulation_v4_current_block_pools,
        simulated_v2_pools = applier.simulated_v2_pools,
        simulated_v3_pools = applier.simulated_v3_pools,
        simulated_v4_pools = applier.simulated_v4_pools,
        "block token processor profile"
    );
}

pub(in crate::tracking::block_processor) fn elapsed_micros(started: Instant) -> u128 {
    started.elapsed().as_micros()
}

fn micros_to_millis(value: u128) -> u128 {
    value / 1_000
}
