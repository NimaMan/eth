mod reply;
pub mod routes;
pub mod sse;
pub mod assets;

pub use crate::app::state::ServerState;

use crate::app::config::TokenServerConfig;
use crate::live::StartLiveTrackerRequest;

pub async fn serve(config: TokenServerConfig) -> eyre::Result<()> {
    let state = ServerState::new(config.clone())?;
    let routes = routes::routes(state.clone());
    let (bound_addr, server) = warp::serve(routes).try_bind_ephemeral(config.bind)?;

    tracing::info!(
        bind = %bound_addr,
        datadir = %config.reth_datadir.display(),
        processed_block_disk_cache_dir = %config
            .processed_block_disk_cache_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "disabled".to_string()),
        processed_block_disk_cache_blocks = config.processed_block_disk_cache_blocks,
        "starting eth_token_server"
    );

    if config.auto_start_live {
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
    } else {
        tracing::info!("live token tracker auto-start disabled");
    }

    server.await;
    Ok(())
}
