#![recursion_limit = "512"]

use eth_token_server::{app::logging, http, TokenServerConfig};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _log_guards = logging::init_logging()?;

    let config = TokenServerConfig::from_config_file()?;
    http::serve(config).await
}
