pub mod routes;
pub mod sse;
pub mod state;

pub use state::ServerState;

use crate::config::TokenServerConfig;
use crate::live::StartLiveTrackerRequest;

pub async fn serve(config: TokenServerConfig) -> eyre::Result<()> {
    let state = ServerState::new(config.clone())?;
    if let Err(error) = state
        .live_tracker
        .start(StartLiveTrackerRequest {
            start_block: None,
            end_block: None,
            warmup_blocks: None,
            history_limit: None,
        })
        .await
    {
        tracing::warn!(error = %error, "failed to auto-start live token tracker");
    }
    let routes = routes::routes(state);

    tracing::info!(
        bind = %config.bind,
        datadir = %config.reth_datadir.display(),
        processed_block_cache_dir = %config
            .processed_block_cache_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "disabled".to_string()),
        processed_block_cache_blocks = config.processed_block_cache_blocks,
        "starting eth_token_server"
    );

    warp::serve(routes).run(config.bind).await;
    Ok(())
}
