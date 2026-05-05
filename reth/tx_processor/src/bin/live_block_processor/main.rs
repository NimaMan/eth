use std::{env, path::PathBuf, sync::Arc};

use eyre::Result;
use reth_chain_query::RethQueryProvider;
use tx_processor::live_pipeline::{LiveBlockProcessorConfig, LiveBlockService};
use tx_simulator::config::repo;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let reth_datadir = repo::reth_datadir()?;
    let execution_rpc = env_or_config("EXECUTION_RPC", repo::reth_http_rpc)?;
    let execution_ws = env_or_config("EXECUTION_WS", repo::reth_ws_rpc)?;
    let redis_url = Some(env_or_config("REDIS_URL", repo::live_data_redis_url)?);
    let notifier_channel = env::var("REDIS_BLOCK_CHANNEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| Some("eth/live/block_notifications".into()));
    let block_limit: Option<usize> = env::var("LIVE_BLOCK_LIMIT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| value.parse().ok());
    let log_path = env::var("LIVE_BLOCK_LOG").ok().map(PathBuf::from);

    let processor_config = LiveBlockProcessorConfig::default()
        .with_execution_rpc(execution_rpc.clone())
        .with_execution_ws(execution_ws.clone());

    let provider = Arc::new(RethQueryProvider::new(&reth_datadir)?);

    let service = LiveBlockService::new(
        provider,
        processor_config,
        redis_url.clone(),
        notifier_channel,
        log_path,
    )
    .await?;

    println!(
        "Running live block service (limit={:?}, redis={:?}, datadir={}, rpc={}, ws={})",
        block_limit, redis_url, reth_datadir, execution_rpc, execution_ws
    );

    if let Some(limit) = block_limit {
        service.run_for_blocks(limit).await?;
    } else {
        service.run().await?;
    }
    Ok(())
}

fn env_or_config<F>(env_key: &str, resolver: F) -> Result<String>
where
    F: FnOnce() -> Result<String>,
{
    if let Ok(value) = env::var(env_key) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    resolver()
}
