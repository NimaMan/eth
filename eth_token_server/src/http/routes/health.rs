use std::convert::Infallible;

use serde_json::json;
use warp::http::StatusCode;

use crate::http::reply::json_response;
use crate::http::ServerState;

pub(super) async fn health(state: ServerState) -> Result<warp::reply::Response, Infallible> {
    Ok(json_response(
        &json!({
            "status": "ok",
            "bind": state.config.bind.to_string(),
            "reth_datadir": state.config.reth_datadir,
            "reth_index_dir": state.config.reth_index_dir,
            "auto_start_live": state.config.auto_start_live,
            "history_limit": state.config.history_limit,
            "default_blocks": state.config.default_blocks,
            "live_warmup_blocks": state.config.live_warmup_blocks,
            "processed_block_disk_cache_dir": state.config.processed_block_disk_cache_dir,
            "processed_block_disk_cache_blocks": state.config.processed_block_disk_cache_blocks,
            "redis_url": state.config.redis_url,
            "live_block_stream": state.config.live_block_stream,
            "live_block_apply_timeout_ms": state.config.live_block_apply_timeout_ms,
            "mempool_signal_limit": state.config.mempool_signal_limit,
            "alpha_trading_enabled": true,
        }),
        StatusCode::OK,
    ))
}
