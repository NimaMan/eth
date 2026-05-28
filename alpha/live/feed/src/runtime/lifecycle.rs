use eth_ops_events::{emit_issue, PipelineImpact, PipelineIssue, PipelineSeverity};
use serde_json::json;

use super::apply_report::push_issue;
use super::progress::{LiveTokenError, LiveTokenStatus};
use super::service::{LiveTokenRuntime, LIVE_TOKEN_TRACKER_LOG_TARGET};
use super::time::now_unix_secs;
use super::LiveTokenEvent;

impl LiveTokenRuntime {
    pub(super) async fn mark_live(&self) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTokenStatus::Live;
        state.progress.live_at_unix_secs = Some(now_unix_secs());
        state.progress.last_error = None;
        state.progress.updated_at_unix_secs = now_unix_secs();
        let event = LiveTokenEvent::RuntimeLive {
            id: state.progress.id.clone().unwrap_or_default(),
            current_block: state.progress.current_block,
        };
        tracing::info!(
            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
            live_id = ?state.progress.id,
            tracked_tokens = state.progress.tracked_tokens,
            tracked_pools = state.progress.tracked_pools,
            tracked_v2_pools = state.progress.tracked_v2_pools,
            tracked_v3_pools = state.progress.tracked_v3_pools,
            tracked_v4_pools = state.progress.tracked_v4_pools,
            "live token runtime warmup completed; entering live tail"
        );
        drop(state);
        let _ = self.inner.event_tx.send(event);
    }

    pub(super) async fn mark_stopped(&self) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTokenStatus::Stopped;
        state.progress.completed_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
        let event = LiveTokenEvent::RuntimeStopped {
            id: state.progress.id.clone(),
            current_block: state.progress.current_block,
        };
        drop(state);
        let _ = self.inner.event_tx.send(event);
    }

    pub(super) async fn mark_failed(&self, error: LiveTokenError) {
        let mut state = self.inner.state.write().await;
        state.progress.status = LiveTokenStatus::Failed;
        state.progress.last_error = Some(error.message.clone());
        state.progress.completed_at_unix_secs = Some(now_unix_secs());
        state.progress.updated_at_unix_secs = now_unix_secs();
        let mut issue = PipelineIssue::new(
            "eth_chain_server",
            "live_tracker",
            "runtime",
            PipelineSeverity::Error,
            PipelineImpact::ServiceDown,
            "live_tracker_failed",
            "Live token tracker failed",
        );
        issue.run_id = state.progress.id.clone();
        issue.fatal = true;
        issue.retryable = true;
        issue.block_number = error.block_number;
        issue.tx_index = error.tx_index;
        issue.tx_hash = error.tx_hash.clone();
        issue.detail = error.detail.clone().or_else(|| Some(error.message.clone()));
        for (key, value) in &error.context {
            issue.context.insert(key.clone(), json!(value));
        }
        issue.refresh_ids();
        let event = LiveTokenEvent::RuntimeFailed {
            id: state.progress.id.clone(),
            block_number: error.block_number,
            message: error.message.clone(),
        };
        tracing::error!(
            target: LIVE_TOKEN_TRACKER_LOG_TARGET,
            live_id = ?state.progress.id,
            status = ?state.progress.status,
            current_block = ?state.progress.current_block,
            warmup_start_block = ?state.progress.warmup_start_block,
            warmup_end_block = ?state.progress.warmup_end_block,
            blocks_processed = state.progress.blocks_processed,
            warmup_total_blocks = state.progress.warmup_total_blocks,
            live_blocks_processed = state.progress.live_blocks_processed,
            txs_processed = state.progress.txs_processed,
            transaction_failures = state.progress.transaction_failures,
            pool_simulation_failures = state.progress.pool_simulation_failures,
            tracked_tokens = state.progress.tracked_tokens,
            tracked_pools = state.progress.tracked_pools,
            tracked_v2_pools = state.progress.tracked_v2_pools,
            tracked_v3_pools = state.progress.tracked_v3_pools,
            tracked_v4_pools = state.progress.tracked_v4_pools,
            block_source = ?state.progress.last_block_source,
            last_block_upstream_ms = ?state.progress.last_block_upstream_ms,
            last_block_token_apply_ms = ?state.progress.last_block_token_apply_ms,
            last_block_disk_cache_read_ms = ?state.progress.last_block_disk_cache_read_ms,
            last_block_disk_cache_write_ms = ?state.progress.last_block_disk_cache_write_ms,
            error_block_number = ?error.block_number,
            error_tx_index = ?error.tx_index,
            error_tx_hash = ?error.tx_hash,
            error = %error.message,
            error_detail = ?error.detail,
            error_context = ?error.context,
            "live token tracker failed"
        );
        emit_issue(&issue);
        push_issue(&mut state, issue);
        state.errors.push(error);
        drop(state);
        let _ = self.inner.event_tx.send(event);
    }
}
