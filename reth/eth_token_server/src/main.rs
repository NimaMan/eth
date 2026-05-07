use eth_token_server::{server, TokenServerConfig};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const DEFAULT_LOG_DIR: &str = "/home/nima/code/crypto/blockchains/eth/logs/eth_token_server";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _log_guard = init_logging()?;

    let config = TokenServerConfig::from_env()?;
    server::serve(config).await
}

fn init_logging() -> eyre::Result<tracing_appender::non_blocking::WorkerGuard> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let log_dir = std::env::var_os("ETH_TOKEN_SERVER_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_LOG_DIR));
    std::fs::create_dir_all(&log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "eth_token_server.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_writer(std::io::stdout);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_writer(file_writer);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    tracing::info!(log_dir = %log_dir.display(), "initialized eth_token_server file logger");
    Ok(guard)
}
