#![recursion_limit = "512"]

use eth_chain_server::{app::logging, http, ChainServerConfig};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _log_guards = logging::init_logging()?;

    let config = ChainServerConfig::from_config_file()?;
    http::serve(config).await
}
