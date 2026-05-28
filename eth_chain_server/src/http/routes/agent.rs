use std::convert::Infallible;

use serde_json::json;
use warp::http::StatusCode;

use crate::http::reply::json_response;
use crate::http::ServerState;
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
                    "base_path": "/api/v1/eth/live-trading",
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
                "live_token_tracker_status": "/api/v1/eth/live-token-tracker/status",
                "live_token_tracker_block_applied_updates": "/api/v1/eth/live-token-tracker/block-applied-updates?after_block=<block>",
                "live_token_tracker_tokens": "/api/v1/eth/live-token-tracker/tokens",
                "live_token_tracker_pools": "/api/v1/eth/live-token-tracker/pools",
                "live_tx_simulator_status": "/api/v1/eth/live-tx-simulator/status",
                "live_tx_simulator_latest_block_state": "/api/v1/eth/live-tx-simulator/latest-block-state",
                "live_tx_simulator_unsigned_sequence": "/api/v1/eth/live-tx-simulator/simulations/unsigned-transaction-sequence",
                "live_tx_simulator_pool_buy_sell": "/api/v1/eth/live-tx-simulator/simulations/pool-buy-sell",
                "trading_live_status": "/api/v1/eth/live-trading/status",
                "trading_live_block_applied_updates": "/api/v1/eth/live-trading/block-applied-updates?after_block=<block>",
                "ranges": "/api/v1/eth/ranges",
                "active_range": "/api/v1/eth/ranges/active",
                "risk_atlas": "/api/v1/eth/analytics/risk-atlas"
            },
            "contracts": {
                "frontend": "May return read-model snapshots and page DTOs. These endpoints can be stale by a bounded interval during active builds.",
                "trading": "Only committed block state is valid. Trading consumers should react after BlockApplied/chain-state-applied, not from frontend DTO refreshes.",
                "agents": "Agents should start from manifest/status, then call versioned HTTP resources. Agents should not depend on legacy /eth/tokens/api paths."
            },
            "risk_atlas_execution_modes": [
                {
                    "mode": "headless_generation",
                    "choose_when": [
                        "large ranges such as 100K or 500K blocks",
                        "model-data generation",
                        "training or evaluation datasets",
                        "scam-label and target distributions without frontend inspection"
                    ],
                    "server_required": false,
                    "state_refresh": "no frontend state; write bounded batches to the Risk Atlas DB",
                    "canonical_output": "risk_atlas_* DB rows and derived aggregates"
                },
                {
                    "mode": "server_backed_range_builder",
                    "choose_when": [
                        "the user needs Asena/token-builder access to generated tokens or pools",
                        "visual token/pool debugging",
                        "small smoke ranges meant for page inspection"
                    ],
                    "server_required": true,
                    "state_refresh": "cheap progress every block; heavy read models every configured interval and at completion",
                    "initial_large_run_interval_blocks": 1000,
                    "canonical_output": "range read-model snapshots plus optional Risk Atlas DB export"
                },
                {
                    "mode": "db_backed_read_mode",
                    "choose_when": [
                        "Risk Atlas page display",
                        "decision-question review",
                        "comparing already imported runs"
                    ],
                    "server_required": true,
                    "state_refresh": "no block processing",
                    "canonical_output": "RiskAtlasReader page view"
                }
            ],
            "agent_decision_rules": [
                "Default to headless_generation for large modelling/data requests.",
                "Use server_backed_range_builder only when frontend range-builder inspection is part of the request.",
                "Use db_backed_read_mode when the requested data already exists in the Risk Atlas DB.",
                "Use the trading committed-state path for live execution/risk decisions, not frontend read models."
            ]
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
                "Use /api/v1/eth/live-token-tracker/status for frontend live-token-tracker status.",
                "Use /api/v1/eth/live-trading/block-applied-updates for committed block wakeups.",
                "Use /api/v1/eth/analytics/risk-atlas for DB-backed atlas pages."
            ]
        }),
        StatusCode::OK,
    ))
}
