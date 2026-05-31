use anyhow::Result;
use tracing_subscriber::EnvFilter;
use tx_executor_signer::{load_eth_signer_config, run_eth_signer};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = load_eth_signer_config()?;
    run_eth_signer(config).await
}
