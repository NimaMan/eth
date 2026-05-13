use crate::token_tracking::cache::{
    CacheApplyOutcome, CacheStatusPolicy, CacheUpdateContext, TokenTrackingCache,
};
use crate::token_tracking::types::{Address, TokenUpdate, TokenWithPools};
use std::collections::HashMap;
use tracing::{debug, warn};

pub async fn apply_snapshot_map_to_cache_with_context(
    cache: &TokenTrackingCache,
    block_number: u64,
    timestamp: f64,
    source: &str,
    status: Option<String>,
    status_policy: CacheStatusPolicy,
    token_map: HashMap<Address, TokenWithPools>,
) -> CacheApplyOutcome {
    if token_map.is_empty() {
        return CacheApplyOutcome::SkippedEmpty;
    }

    let update = TokenUpdate {
        message_type: source.to_string(),
        token_count: token_map.len(),
        block_number,
        timestamp,
        data: token_map,
    };

    let outcome = cache
        .batch_update_with_context(
            update,
            CacheUpdateContext {
                source: source.to_string(),
                status,
                status_policy,
            },
        )
        .await;

    match &outcome {
        CacheApplyOutcome::Applied(result) => {
            debug!(
                "Applied token-server cache snapshot @block {} ({} tokens, {} pools updated)",
                block_number, result.tokens_updated, result.pools_updated
            );
        }
        CacheApplyOutcome::Rejected(rejection) => {
            warn!(
                "Rejected token cache snapshot source={} status={:?} block={} current_block={} reason={:?}",
                rejection.source,
                rejection.status,
                rejection.block_number,
                rejection.current_block,
                rejection.reason
            );
        }
        CacheApplyOutcome::SkippedEmpty => {}
    }

    outcome
}
