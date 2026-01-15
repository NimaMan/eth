use std::{env, path::PathBuf, sync::Arc};

use eyre::Result;
use reth_chain_query::RethQueryProvider;
use tx_processor::live_pipeline::{LiveBlockProcessorConfig, LiveBlockService};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let reth_datadir = env::var("RETH_DATA_DIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    let execution_rpc =
        env::var("EXECUTION_RPC").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string());
    let execution_ws =
        env::var("EXECUTION_WS").unwrap_or_else(|_| "ws://127.0.0.1:8546".to_string());
    let redis_url = env::var("REDIS_URL").ok();
    let notifier_channel = env::var("REDIS_BLOCK_CHANNEL")
        .ok()
        .or_else(|| Some("live:block_published".into()));
    let block_limit: usize = env::var("LIVE_BLOCK_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2);
    let log_path = env::var("LIVE_BLOCK_LOG")
        .ok()
        .map(|path| PathBuf::from(path));

    let processor_config = LiveBlockProcessorConfig::default()
        .with_execution_rpc(execution_rpc)
        .with_execution_ws(execution_ws);

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
        "Running live block service for {} blocks (redis={:?})",
        block_limit, redis_url
    );

    service.run_for_blocks(block_limit).await?;
    Ok(())
}
