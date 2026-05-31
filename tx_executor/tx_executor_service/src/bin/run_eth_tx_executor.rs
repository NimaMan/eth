use anyhow::Result;
use axum::{extract::DefaultBodyLimit, routing::get, Router};
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use tx_executor_service::{
    build_eth_tx_executor_routes, load_eth_tx_execution_config,
    load_eth_tx_executor_service_config, EthTxExecutorAppState, EthTxPolicyRepository,
};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let execution_config = load_eth_tx_execution_config()?;
    let service_config = load_eth_tx_executor_service_config()?;
    let http_config = service_config.http.clone();
    let broadcast_mode = service_config.executor.broadcast_mode;
    let policy_repository = EthTxPolicyRepository::connect(&execution_config).await?;
    let state = EthTxExecutorAppState::from_config(service_config, policy_repository)?;

    let app = Router::new()
        .route("/health", get(health))
        .merge(build_eth_tx_executor_routes(state))
        .layer(DefaultBodyLimit::max(http_config.max_request_body_bytes));

    let listener = TcpListener::bind(http_config.bind_addr).await?;
    if http_config.bind_addr.ip().is_unspecified() {
        warn!("ETH tx executor is listening on all interfaces");
    }
    info!(
        bind = %http_config.bind_addr,
        ?broadcast_mode,
        "ETH tx executor service listening"
    );
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
