use alloy_primitives::U256;
use eth_alpha_store::PostgresTradingStore;
use eth_ops_events::{emit_health, PipelineHealth, PipelineHealthStatus};
use eyre::{Result, WrapErr};
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::wire::LiveStatusResponse;

use super::constants::ALPHA_TRADER_STALE_RUN_MAX_AGE_SECS;
use super::support::TraderExecutionMode;

pub(super) struct HeartbeatInput<'a> {
    pub(super) runner_name: &'a str,
    pub(super) execution_mode: TraderExecutionMode,
    pub(super) chain_state_payload: Option<Value>,
    pub(super) process_started_at_text: &'a str,
    pub(super) process_started_at_unix_secs: i64,
    pub(super) status: &'a LiveStatusResponse,
    pub(super) live_ready: bool,
    pub(super) suppress_events: bool,
    pub(super) seen_pool_count: usize,
    pub(super) frame_pool_count: usize,
    pub(super) signal_count: usize,
    pub(super) market_events: usize,
    pub(super) risk_events: usize,
    pub(super) position_monitor_events: usize,
    pub(super) manual_close_requests: usize,
    pub(super) manual_close_failed: usize,
    pub(super) manual_close_reports: usize,
    pub(super) chain_sim_settlement_loaded: usize,
    pub(super) chain_sim_settlement_reports: usize,
    pub(super) chain_sim_settlement_pending: usize,
    pub(super) chain_sim_settlement_waiting_state: usize,
    pub(super) chain_sim_settlement_missing_block: usize,
    pub(super) receipt_reports: usize,
    pub(super) receipt_unresolved: usize,
    pub(super) reports: usize,
    pub(super) strategy_count: usize,
    pub(super) observation_strategy_name: &'a str,
    pub(super) positions: usize,
    pub(super) entry_enabled: bool,
    pub(super) single_max_entry_pools: Option<usize>,
    pub(super) single_entry_bankroll_eth: &'a Option<String>,
    pub(super) single_entry_bankroll_wei: Option<U256>,
    pub(super) entry_bankroll_summary: &'a [Value],
    pub(super) run_id: &'a str,
    pub(super) store: &'a PostgresTradingStore,
}

pub(super) async fn emit_tick_heartbeat(input: HeartbeatInput<'_>) -> Result<Value> {
    info!(
        live_status = %input.status.progress.status,
        live_current_block = ?input.status.progress.current_block,
        live_blocks_processed = input.status.progress.blocks_processed,
        live_warmup_total_blocks = input.status.progress.warmup_total_blocks,
        live_tracked_tokens = input.status.progress.tracked_tokens,
        live_tracked_pools = input.status.progress.tracked_pool_count(),
        live_tracked_v2_pools = input.status.progress.tracked_v2_pools,
        live_tracked_v3_pools = input.status.progress.tracked_v3_pools,
        live_tracked_v4_pools = input.status.progress.tracked_v4_pools,
        live_last_error = ?input.status.progress.last_error,
        trading_enabled = !input.suppress_events,
        pools_seen = input.seen_pool_count,
        block_frame_pool_count = input.frame_pool_count,
        signal_count = input.signal_count,
        market_events = input.market_events,
        risk_events = input.risk_events,
        position_monitor_events = input.position_monitor_events,
        manual_close_requests = input.manual_close_requests,
        manual_close_failed = input.manual_close_failed,
        manual_close_reports = input.manual_close_reports,
        chain_sim_settlement_loaded = input.chain_sim_settlement_loaded,
        chain_sim_settlement_reports = input.chain_sim_settlement_reports,
        chain_sim_settlement_pending = input.chain_sim_settlement_pending,
        chain_sim_settlement_waiting_state = input.chain_sim_settlement_waiting_state,
        chain_sim_settlement_missing_block = input.chain_sim_settlement_missing_block,
        receipt_reports = input.receipt_reports,
        receipt_unresolved = input.receipt_unresolved,
        reports = input.reports,
        strategy_count = input.strategy_count,
        positions = input.positions,
        chain_sim_selected_block = ?input.chain_state_payload.as_ref().and_then(|state| state.get("selected_block_number")).and_then(serde_json::Value::as_u64),
        chain_sim_state_source = ?input.chain_state_payload.as_ref().and_then(|state| state.get("source")).and_then(serde_json::Value::as_str),
        "alpha trader tick"
    );

    let heartbeat_metadata = json!({
        "execution_model": input.execution_mode.execution_model(),
        "chain_sim_state": input.chain_state_payload,
        "process_started_at": input.process_started_at_text,
        "process_started_at_unix_secs": input.process_started_at_unix_secs,
        "live_status": input.status.progress.status,
        "live_current_block": input.status.progress.current_block,
        "live_blocks_processed": input.status.progress.blocks_processed,
        "live_warmup_total_blocks": input.status.progress.warmup_total_blocks,
        "live_tracked_tokens": input.status.progress.tracked_tokens,
        "live_tracked_pools": input.status.progress.tracked_pool_count(),
        "live_tracked_v2_pools": input.status.progress.tracked_v2_pools,
        "live_tracked_v3_pools": input.status.progress.tracked_v3_pools,
        "live_tracked_v4_pools": input.status.progress.tracked_v4_pools,
        "live_last_error": input.status.progress.last_error,
        "trading_enabled": !input.suppress_events,
        "pools_seen": input.seen_pool_count,
        "block_frame_pool_count": input.frame_pool_count,
        "signal_count": input.signal_count,
        "market_events": input.market_events,
        "risk_events": input.risk_events,
        "position_monitor_events": input.position_monitor_events,
        "manual_close_requests": input.manual_close_requests,
        "manual_close_failed": input.manual_close_failed,
        "manual_close_reports": input.manual_close_reports,
        "chain_sim_settlement_loaded": input.chain_sim_settlement_loaded,
        "chain_sim_settlement_reports": input.chain_sim_settlement_reports,
        "chain_sim_settlement_pending": input.chain_sim_settlement_pending,
        "chain_sim_settlement_waiting_state": input.chain_sim_settlement_waiting_state,
        "chain_sim_settlement_missing_block": input.chain_sim_settlement_missing_block,
        "receipt_reports": input.receipt_reports,
        "receipt_unresolved": input.receipt_unresolved,
        "reports": input.reports,
        "strategy_count": input.strategy_count,
        "observation_strategy_name": input.observation_strategy_name,
        "positions": input.positions,
        "entry_enabled": input.entry_enabled,
        "max_entry_pools": input.single_max_entry_pools,
        "entry_bankroll_eth": input.single_entry_bankroll_eth,
        "entry_bankroll_wei": input.single_entry_bankroll_wei.map(|value| value.to_string()),
        "entry_bankrolls": input.entry_bankroll_summary,
        "kartal_enabled": input.execution_mode.uses_kartal(),
    });

    let mut health = PipelineHealth::new(
        input.runner_name,
        "alpha_trader",
        "main_loop",
        if input.live_ready {
            PipelineHealthStatus::Healthy
        } else {
            PipelineHealthStatus::Watch
        },
    );
    health.run_id = Some(input.run_id.to_string());
    health.current_block = input.status.progress.current_block;
    health.metrics.insert(
        "live_status".to_string(),
        json!(input.status.progress.status),
    );
    health.metrics.insert(
        "live_blocks_processed".to_string(),
        json!(input.status.progress.blocks_processed),
    );
    health.metrics.insert(
        "block_frame_pool_count".to_string(),
        json!(input.frame_pool_count),
    );
    health
        .metrics
        .insert("signal_count".to_string(), json!(input.signal_count));
    health
        .metrics
        .insert("market_events".to_string(), json!(input.market_events));
    health
        .metrics
        .insert("risk_events".to_string(), json!(input.risk_events));
    health.metrics.insert(
        "position_monitor_events".to_string(),
        json!(input.position_monitor_events),
    );
    health.metrics.insert(
        "manual_close_requests".to_string(),
        json!(input.manual_close_requests),
    );
    health.metrics.insert(
        "manual_close_failed".to_string(),
        json!(input.manual_close_failed),
    );
    health.metrics.insert(
        "manual_close_reports".to_string(),
        json!(input.manual_close_reports),
    );
    health.metrics.insert(
        "chain_sim_settlement_loaded".to_string(),
        json!(input.chain_sim_settlement_loaded),
    );
    health.metrics.insert(
        "chain_sim_settlement_reports".to_string(),
        json!(input.chain_sim_settlement_reports),
    );
    health.metrics.insert(
        "chain_sim_settlement_pending".to_string(),
        json!(input.chain_sim_settlement_pending),
    );
    health.metrics.insert(
        "chain_sim_settlement_waiting_state".to_string(),
        json!(input.chain_sim_settlement_waiting_state),
    );
    health.metrics.insert(
        "chain_sim_settlement_missing_block".to_string(),
        json!(input.chain_sim_settlement_missing_block),
    );
    health
        .metrics
        .insert("receipt_reports".to_string(), json!(input.receipt_reports));
    health.metrics.insert(
        "receipt_unresolved".to_string(),
        json!(input.receipt_unresolved),
    );
    health
        .metrics
        .insert("reports".to_string(), json!(input.reports));
    health
        .metrics
        .insert("strategy_count".to_string(), json!(input.strategy_count));
    health
        .metrics
        .insert("positions".to_string(), json!(input.positions));
    health
        .metrics
        .insert("trading_enabled".to_string(), json!(!input.suppress_events));
    emit_health(&health);

    input
        .store
        .heartbeat(heartbeat_metadata.clone())
        .await
        .wrap_err("failed to write alpha trader heartbeat")?;
    match input
        .store
        .mark_stale_runs(ALPHA_TRADER_STALE_RUN_MAX_AGE_SECS)
        .await
    {
        Ok(stale_runs) if stale_runs > 0 => {
            info!(
                stale_runs,
                max_age_secs = ALPHA_TRADER_STALE_RUN_MAX_AGE_SECS,
                "marked stale alpha trader runs during heartbeat"
            );
        }
        Ok(_) => {}
        Err(error) => {
            warn!(
                error = %error,
                max_age_secs = ALPHA_TRADER_STALE_RUN_MAX_AGE_SECS,
                "failed to mark stale alpha trader runs during heartbeat"
            );
        }
    }
    Ok(heartbeat_metadata)
}
