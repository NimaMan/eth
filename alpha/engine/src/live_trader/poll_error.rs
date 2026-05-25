use std::time::Duration;

use eth_alpha_store::PostgresTradingStore;
use eth_ops_events::{
    emit_health, emit_issue, PipelineHealth, PipelineHealthStatus, PipelineImpact, PipelineIssue,
    PipelineSeverity,
};
use eyre::{Result, WrapErr};
use serde_json::json;
use tokio::time;
use tracing::warn;

use super::support::ShutdownSignals;

pub(super) async fn handle_poll_error(
    runner_name: &'static str,
    error: eyre::Report,
    run_id: &str,
    token_server_url: &str,
    process_started_at_text: &str,
    process_started_at_unix_secs: i64,
    positions: usize,
    store: &PostgresTradingStore,
    once: bool,
    shutdown: &mut ShutdownSignals,
    retry_delay: Duration,
) -> Result<bool> {
    warn!(error = %error, "alpha trader poll failed");
    let mut issue = PipelineIssue::new(
        runner_name,
        "alpha_trader",
        "token_server_poll",
        PipelineSeverity::Warn,
        PipelineImpact::ServiceDegraded,
        "alpha_trader_poll_failed",
        "Alpha trader token server poll failed",
    );
    issue.run_id = Some(run_id.to_string());
    issue.retryable = true;
    issue.detail = Some(error.to_string());
    issue
        .context
        .insert("token_server_url".to_string(), json!(token_server_url));
    issue
        .context
        .insert("positions".to_string(), json!(positions));
    issue.refresh_ids();
    emit_issue(&issue);

    let mut health = PipelineHealth::new(
        runner_name,
        "alpha_trader",
        "main_loop",
        PipelineHealthStatus::Degraded,
    );
    health.run_id = Some(run_id.to_string());
    health
        .metrics
        .insert("positions".to_string(), json!(positions));
    health
        .metrics
        .insert("poll_error".to_string(), json!(error.to_string()));
    emit_health(&health);

    let metadata = json!({
        "token_server_url": token_server_url,
        "poll_error": error.to_string(),
        "process_started_at": process_started_at_text,
        "process_started_at_unix_secs": process_started_at_unix_secs,
        "trading_enabled": false,
        "positions": positions,
    });
    store
        .heartbeat(metadata.clone())
        .await
        .wrap_err("failed to write alpha trader error heartbeat")?;

    if once {
        store
            .mark_stopped("failed", metadata)
            .await
            .wrap_err("failed to mark alpha trader run failed")?;
        return Ok(true);
    }

    tokio::select! {
        _ = time::sleep(retry_delay) => Ok(false),
        _ = shutdown.recv() => {
            store
                .mark_stopped(
                    "stopped",
                    json!({
                        "reason": "shutdown_signal",
                        "positions": positions,
                    }),
                )
                .await
                .wrap_err("failed to mark alpha trader run stopped")?;
            Ok(true)
        }
    }
}
