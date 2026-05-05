use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::Local;
use eyre::Result;
use reth_chain_query::RethQueryProvider;
use tracing_subscriber::EnvFilter;
use tx_processor::live::{LiveBlockProcessorConfig, LiveBlockService};
use tx_simulator::config::repo;

const LIVE_BLOCK_WARMUP_BLOCKS: usize = 5;

#[tokio::main]
async fn main() -> Result<()> {
    let log_path = live_block_log_path()?;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
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

    let processor_config = LiveBlockProcessorConfig::default()
        .with_execution_rpc(execution_rpc.clone())
        .with_execution_ws(execution_ws.clone());

    let provider = Arc::new(RethQueryProvider::new(&reth_datadir)?);

    let mut service = LiveBlockService::new(
        provider,
        processor_config,
        redis_url.clone(),
        notifier_channel,
        Some(log_path.clone()),
    )
    .await?;

    println!(
        "Running live block service (limit={:?}, warmup_blocks={}, redis={:?}, datadir={}, rpc={}, ws={}, log={})",
        block_limit,
        LIVE_BLOCK_WARMUP_BLOCKS,
        redis_url,
        reth_datadir,
        execution_rpc,
        execution_ws,
        log_path.display()
    );

    service
        .warmup_recent_blocks(LIVE_BLOCK_WARMUP_BLOCKS)
        .await?;

    if let Some(limit) = block_limit {
        service.run_for_blocks(limit).await?;
    } else {
        service.run().await?;
    }
    Ok(())
}

fn live_block_log_path() -> Result<PathBuf> {
    if let Some(raw_path) = non_empty_env("LIVE_BLOCK_LOG") {
        let path = PathBuf::from(raw_path);
        if is_log_file_path(&path) {
            return Ok(path);
        }
        return Ok(timestamped_log_path(path));
    }

    if let Some(raw_dir) = non_empty_env("LIVE_BLOCK_LOG_DIR") {
        return Ok(timestamped_log_path(PathBuf::from(raw_dir)));
    }

    if let Some(raw_eth_log_dir) = non_empty_env("ETH_LOG_DIR") {
        return Ok(timestamped_log_path(
            PathBuf::from(raw_eth_log_dir).join("block_processor"),
        ));
    }

    Ok(timestamped_log_path(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("logs")
            .join("block_processor"),
    ))
}

fn timestamped_log_path(dir: PathBuf) -> PathBuf {
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    dir.join(format!("live_block_processor_{timestamp}.log"))
}

fn is_log_file_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("log"))
        .unwrap_or(false)
}

fn non_empty_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
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
