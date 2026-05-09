use std::time::Instant;

use crate::tracking::token_update_router::ProcessedTokenUpdateProfile;

use super::processor::BlockTokenProcessor;

const TOKEN_BLOCK_PROCESSOR_PROFILE_LOG_TARGET: &str = "token_block_processor";

#[derive(Default)]
pub(in crate::tracking::block_processor) struct BlockTokenProcessorProfile {
    pub(in crate::tracking::block_processor) sort_ms: u128,
    pub(in crate::tracking::block_processor) token_metadata_ms: u128,
    pub(in crate::tracking::block_processor) router_ms: u128,
    pub(in crate::tracking::block_processor) index_refresh_ms: u128,
    pub(in crate::tracking::block_processor) network_update_ms: u128,
    pub(in crate::tracking::block_processor) finalize_ms: u128,
    pub(in crate::tracking::block_processor) skipped_transactions: usize,
    pub(in crate::tracking::block_processor) processing_error_transactions: usize,
    pub(in crate::tracking::block_processor) applicable_transactions: usize,
    pub(in crate::tracking::block_processor) applier: ProcessedTokenUpdateProfile,
}

pub(in crate::tracking::block_processor) fn log_block_token_processor_profile(
    processor: &BlockTokenProcessor,
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
    let total_ms = elapsed_millis(block_started);
    let applier = &profile.applier;
    tracing::info!(
        target: TOKEN_BLOCK_PROCESSOR_PROFILE_LOG_TARGET,
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
        total_ms,
        sort_ms = profile.sort_ms,
        token_metadata_ms = profile.token_metadata_ms,
        router_ms = profile.router_ms,
        index_refresh_ms = profile.index_refresh_ms,
        network_update_ms = profile.network_update_ms,
        finalize_ms = profile.finalize_ms,
        applier_candidate_ms = applier.candidate_ms,
        applier_token_state_ms = applier.token_state_ms,
        applier_pool_discovery_ms = applier.pool_discovery_ms,
        applier_pool_update_ms = applier.pool_update_ms,
        applier_simulation_v2_ms = applier.simulation_v2_ms,
        applier_simulation_v3_ms = applier.simulation_v3_ms,
        applier_simulation_v4_ms = applier.simulation_v4_ms,
        applier_report_ms = applier.report_ms,
        applier_candidate_tokens = applier.candidate_tokens,
        applier_visited_tokens = applier.visited_tokens,
        applier_update_reports = applier.update_reports,
        simulated_v2_pools = applier.simulated_v2_pools,
        simulated_v3_pools = applier.simulated_v3_pools,
        simulated_v4_pools = applier.simulated_v4_pools,
        "block token processor profile"
    );
}

pub(in crate::tracking::block_processor) fn elapsed_millis(started: Instant) -> u128 {
    started.elapsed().as_millis()
}
