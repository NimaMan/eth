pub mod routes;
pub mod sse;
pub mod state;

pub use state::ServerState;

use crate::config::TokenServerConfig;

pub async fn serve(config: TokenServerConfig) -> eyre::Result<()> {
    let state = ServerState::new(config.clone())?;
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
