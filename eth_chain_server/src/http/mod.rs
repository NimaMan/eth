mod reply;
pub mod routes;
pub mod sse;

pub use crate::app::state::ServerState;

use std::time::Duration;

use crate::app::config::ChainServerConfig;
use crate::live::StartLiveTrackerRequest;

const OPS_HEALTH_HEARTBEAT_SECS: u64 = 60;

pub async fn serve(config: ChainServerConfig) -> eyre::Result<()> {
    let state = ServerState::new(config.clone())?;
    state.spawn_recent_live_block_recorder();
    state.spawn_live_block_frame_recorder();
    state.spawn_recent_live_fee_sample_recorder();
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
        "starting eth_chain_server"
    );

    if config.auto_start_live {
        if let Err(error) = state
            .live_chain_runtime
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

    spawn_ops_health_heartbeat(state.clone());

    server.await;
    Ok(())
}

fn spawn_ops_health_heartbeat(state: ServerState) {
    let live_tracker = state.live_tracker.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(OPS_HEALTH_HEARTBEAT_SECS)).await;
            crate::read_models::ops::health(&live_tracker).await;
        }
    });
}
