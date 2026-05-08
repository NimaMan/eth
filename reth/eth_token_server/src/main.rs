use eth_token_server::{config::shared_config_value, server, TokenServerConfig};
use std::path::PathBuf;
use tracing::{Level, Metadata};
use tracing_subscriber::{
    filter::filter_fn, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

const DEFAULT_LOG_DIR: &str = "/home/nima/code/crypto/blockchains/eth/logs/eth_token_server";
const DEFAULT_SIMULATOR_LOG_DIR: &str = "/home/nima/code/crypto/blockchains/eth/logs/simulators";
const TOKEN_SERVER_LOG_DIR_CONFIG: &str = "TOKEN_SERVER_LOG_DIR";
const SIMULATOR_LOG_DIR_CONFIG: &str = "SIMULATOR_LOG_DIR";

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let _log_guards = init_logging()?;

    let config = TokenServerConfig::from_config_file()?;
    server::serve(config).await
}

struct LogGuards {
    _token_server: tracing_appender::non_blocking::WorkerGuard,
    _simulator: tracing_appender::non_blocking::WorkerGuard,
    _simulation_errors: tracing_appender::non_blocking::WorkerGuard,
}

fn init_logging() -> eyre::Result<LogGuards> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "info,pool_buy_sell_sim=debug,replay_parity_sim=debug,token_safety_lab=debug",
        )
    });
    let log_dir = config_path(TOKEN_SERVER_LOG_DIR_CONFIG, DEFAULT_LOG_DIR)?;
    let simulator_log_dir = config_path(SIMULATOR_LOG_DIR_CONFIG, DEFAULT_SIMULATOR_LOG_DIR)?;
    std::fs::create_dir_all(&log_dir)?;
    std::fs::create_dir_all(&simulator_log_dir)?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "eth_token_server.log");
    let (file_writer, token_server_guard) = tracing_appender::non_blocking(file_appender);
    let simulator_appender = tracing_appender::rolling::daily(&simulator_log_dir, "simulator.log");
    let (simulator_writer, simulator_guard) = tracing_appender::non_blocking(simulator_appender);
    let simulation_errors_appender =
        tracing_appender::rolling::daily(&simulator_log_dir, "simulation_errors.log");
    let (simulation_errors_writer, simulation_errors_guard) =
        tracing_appender::non_blocking(simulation_errors_appender);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_writer(std::io::stdout)
        .with_filter(filter_fn(|metadata| !is_simulator_target(metadata)));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(filter_fn(|metadata| !is_simulator_target(metadata)));
    let simulator_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_ansi(false)
        .with_writer(simulator_writer)
        .with_filter(filter_fn(is_simulator_log_metadata));
    let simulation_errors_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_ansi(false)
        .with_writer(simulation_errors_writer)
        .with_filter(filter_fn(|metadata| {
            metadata.name() == "range_block_apply"
                || (is_simulator_target(metadata)
                    && matches!(*metadata.level(), Level::WARN | Level::ERROR))
        }));

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(simulator_layer)
        .with(simulation_errors_layer)
        .init();

    tracing::info!(
        log_dir = %log_dir.display(),
        simulator_log_dir = %simulator_log_dir.display(),
        "initialized eth_token_server file logger"
    );
    Ok(LogGuards {
        _token_server: token_server_guard,
        _simulator: simulator_guard,
        _simulation_errors: simulation_errors_guard,
    })
}

fn config_path(key: &str, default: &str) -> eyre::Result<PathBuf> {
    Ok(shared_config_value(key)?
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default)))
}

fn is_simulator_target(metadata: &Metadata<'_>) -> bool {
    matches!(
        metadata.target(),
        "pool_buy_sell_sim" | "replay_parity_sim" | "token_safety_lab"
    )
}

fn is_simulator_log_metadata(metadata: &Metadata<'_>) -> bool {
    is_simulator_target(metadata) || metadata.name() == "range_block_apply"
}
