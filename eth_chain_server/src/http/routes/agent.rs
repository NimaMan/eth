use std::convert::Infallible;

use serde_json::json;
use warp::http::StatusCode;

use crate::http::ServerState;
use crate::http::reply::json_response;
use crate::read_models as views;

pub(super) async fn manifest(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &json!({
            "api_version": "v1",
            "chain": "ethereum",
            "bind": state.config.bind.to_string(),
            "surfaces": [
                {
                    "name": "frontend",
                    "transport": "http_json_sse",
                    "base_path": "/api/v1/eth",
                    "purpose": "Asena, lab pages, human inspection, and read-model views.",
                    "use_for": [
                        "live token and pool pages",
                        "historical range builder",
                        "risk atlas pages",
                        "ops dashboards"
                    ],
                    "avoid_for": [
                        "latency-sensitive trading decisions",
                        "per-block trading state transfer"
                    ]
                },
                {
                    "name": "trading",
                    "transport": "in_process_events_now_grpc_later",
                    "base_path": "/api/v1/eth/trading",
                    "purpose": "Committed-chain trading state and low-latency block-applied events.",
                    "use_for": [
                        "reading committed live status",
                        "waiting for committed block updates",
                        "future out-of-process trading stream"
                    ],
                    "avoid_for": [
                        "frontend table rendering",
                        "risk atlas aggregation",
                        "large token or pool scans"
                    ]
                },
                {
                    "name": "agents",
                    "transport": "http_json",
                    "base_path": "/api/v1/eth/agents",
                    "purpose": "Stable, tool-friendly orientation and automation endpoints for coding/research agents.",
                    "use_for": [
                        "discovering API shape",
                        "checking current live/range status",
                        "choosing versioned read endpoints"
                    ],
                    "avoid_for": [
                        "direct access to internal hot-path trading state",
                        "unbounded read-model polling"
                    ]
                }
            ],
            "entrypoints": {
                "agent_manifest": "/api/v1/eth/agents/manifest",
                "agent_status": "/api/v1/eth/agents/status",
                "health": "/api/v1/eth/health",
                "live_status": "/api/v1/eth/live/status",
                "live_updates": "/api/v1/eth/live/updates?after_block=<block>",
                "trading_live_status": "/api/v1/eth/trading/live/status",
                "trading_live_updates": "/api/v1/eth/trading/live/updates?after_block=<block>",
                "ranges": "/api/v1/eth/ranges",
                "active_range": "/api/v1/eth/ranges/active",
                "risk_atlas": "/api/v1/eth/analytics/risk-atlas"
            },
            "contracts": {
                "frontend": "May return read-model snapshots and page DTOs. These endpoints can be stale by a bounded interval during active builds.",
                "trading": "Only committed block state is valid. Trading consumers should react after BlockApplied/chain-state-applied, not from frontend DTO refreshes.",
                "agents": "Agents should start from manifest/status, then call versioned HTTP resources. Agents should not depend on legacy /eth/tokens/api paths."
            }
        }),
        StatusCode::OK,
    ))
}

pub(super) async fn status(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    let live = views::live::status(&state.live_tracker).await;
    let active_range = match state.range_indexer.active_run().await {
        Some(run) => Some(views::run::progress_response(&run).await),
        None => None,
    };
    let risk_atlas = match state.risk_atlas.runs(1).await {
        Ok(mut runs) => json!({
            "available": true,
            "latest_run": runs.pop(),
        }),
        Err(error) => json!({
            "available": false,
            "error": error.to_string(),
        }),
    };

    Ok(json_response(
        &json!({
            "api_version": "v1",
            "chain": "ethereum",
            "server": {
                "status": "ok",
                "bind": state.config.bind.to_string(),
            },
            "live": {
                "progress": live.progress,
                "summary": live.summary,
                "error_count": live.errors.len(),
                "issue_count": live.issues.len(),
                "bottleneck_count": live.bottlenecks.len(),
            },
            "active_range": active_range,
            "risk_atlas": risk_atlas,
            "recommended_next": [
                "Use /api/v1/eth/live/status for frontend live status.",
                "Use /api/v1/eth/trading/live/updates for committed block wakeups.",
                "Use /api/v1/eth/analytics/risk-atlas for DB-backed atlas pages."
            ]
        }),
        StatusCode::OK,
    ))
}
