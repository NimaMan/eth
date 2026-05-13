use std::{
    backtrace::Backtrace,
    env, panic,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use eth_pipeline_telemetry::{JsonlTelemetrySink, MultiTelemetrySink, TracingTelemetrySink};
use serde_json::json;
use tracing::{Level, Metadata};
use tracing_subscriber::{filter::filter_fn, layer::SubscriberExt, util::SubscriberInitExt, Layer};

use super::config::shared_config_value;

const DEFAULT_LOG_DIR: &str = "/home/nima/code/crypto/blockchains/eth/logs/eth_chain_server";
const CHAIN_SERVER_LOG_DIR_CONFIG: &str = "CHAIN_SERVER_LOG_DIR";
const CHAIN_SERVER_LOG_RUN_ID_CONFIG: &str = "CHAIN_SERVER_LOG_RUN_ID";
const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";
const TOKEN_RANGE_APPLY_PROFILE_LOG_TARGET: &str = "token_range_apply_profile";
const TOKEN_BLOCK_PROCESSOR_PROFILE_LOG_TARGET: &str = "token_block_processor_profile";
const TOKEN_SIM_SESSION_PROFILE_LOG_TARGET: &str = "token_sim_session_profile";
const LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET: &str = "live_token_apply_profile";
const POOL_BUY_SELL_SIM_LOG_TARGET: &str = "pool_buy_sell_sim";

pub struct LogGuards {
    _server: tracing_appender::non_blocking::WorkerGuard,
    _events: tracing_appender::non_blocking::WorkerGuard,
    _live_token_tracker: tracing_appender::non_blocking::WorkerGuard,
    _pool_buy_sell_sim_failures: tracing_appender::non_blocking::WorkerGuard,
    _simulation_failures: tracing_appender::non_blocking::WorkerGuard,
    _token_pipeline_profile: tracing_appender::non_blocking::WorkerGuard,
}

struct RunLogDir {
    run_id: String,
    path: PathBuf,
}

pub fn init_logging() -> eyre::Result<LogGuards> {
    let log_root = config_path(CHAIN_SERVER_LOG_DIR_CONFIG, DEFAULT_LOG_DIR)?;
    let run_log = create_run_log_dir(&log_root)?;
    let run_dir = run_log.path.clone();

    let file_appender = tracing_appender::rolling::never(&run_dir, "server.log");
    let (file_writer, server_guard) = tracing_appender::non_blocking(file_appender);
    let events_appender = tracing_appender::rolling::never(&run_dir, "events.jsonl");
    let (events_writer, events_guard) = tracing_appender::non_blocking(events_appender);
    let live_token_tracker_appender =
        tracing_appender::rolling::never(&run_dir, "live_token_tracker.jsonl");
    let (live_token_tracker_writer, live_token_tracker_guard) =
        tracing_appender::non_blocking(live_token_tracker_appender);
    let pool_buy_sell_sim_failures_appender =
        tracing_appender::rolling::never(&run_dir, "pool_buy_sell_sim_failures.jsonl");
    let (pool_buy_sell_sim_failures_writer, pool_buy_sell_sim_failures_guard) =
        tracing_appender::non_blocking(pool_buy_sell_sim_failures_appender);
    let simulation_failures_appender =
        tracing_appender::rolling::never(&run_dir, "simulation_failures.jsonl");
    let (simulation_failures_writer, simulation_failures_guard) =
        tracing_appender::non_blocking(simulation_failures_appender);
    let token_pipeline_profile_appender =
        tracing_appender::rolling::never(&run_dir, "token_pipeline_profile.jsonl");
    let (token_pipeline_profile_writer, token_pipeline_profile_guard) =
        tracing_appender::non_blocking(token_pipeline_profile_appender);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_writer(std::io::stdout)
        .with_filter(filter_fn(is_server_log_metadata));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(filter_fn(is_server_log_metadata));
    let events_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_ansi(false)
        .with_writer(events_writer)
        .with_filter(filter_fn(is_event_metadata));
    let live_token_tracker_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_ansi(false)
        .with_writer(live_token_tracker_writer)
        .with_filter(filter_fn(is_live_token_tracker_metadata));
    let pool_buy_sell_sim_failures_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_ansi(false)
        .with_writer(pool_buy_sell_sim_failures_writer)
        .with_filter(filter_fn(is_pool_buy_sell_sim_failure_metadata));
    let simulation_failures_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_ansi(false)
        .with_writer(simulation_failures_writer)
        .with_filter(filter_fn(is_non_pool_simulator_failure_metadata));
    let token_pipeline_profile_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_ansi(false)
        .with_writer(token_pipeline_profile_writer)
        .with_filter(filter_fn(is_token_pipeline_profile_metadata));

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .with(events_layer)
        .with(live_token_tracker_layer)
        .with(pool_buy_sell_sim_failures_layer)
        .with(simulation_failures_layer)
        .with(token_pipeline_profile_layer)
        .init();
    install_panic_hook();

    let telemetry_sink = MultiTelemetrySink::new(vec![
        Arc::new(JsonlTelemetrySink::open(&run_dir)?),
        Arc::new(TracingTelemetrySink),
    ]);
    let telemetry_initialized = eth_pipeline_telemetry::init_global_sink(Arc::new(telemetry_sink));

    tracing::info!(
        log_root = %log_root.display(),
        run_dir = %run_dir.display(),
        run_id = %run_log.run_id,
        telemetry_initialized,
        "initialized eth_chain_server file logger"
    );
    Ok(LogGuards {
        _server: server_guard,
        _events: events_guard,
        _live_token_tracker: live_token_tracker_guard,
        _pool_buy_sell_sim_failures: pool_buy_sell_sim_failures_guard,
        _simulation_failures: simulation_failures_guard,
        _token_pipeline_profile: token_pipeline_profile_guard,
    })
}

fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let thread = thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");
        let message = panic_message(info.payload());
        let location = info.location();
        let file = location
            .map(|location| location.file())
            .unwrap_or("unknown");
        let line = location.map(|location| location.line()).unwrap_or_default();
        let column = location
            .map(|location| location.column())
            .unwrap_or_default();
        let backtrace = Backtrace::force_capture().to_string();

        tracing::error!(
            target: "runtime_panic",
            thread = %thread_name,
            panic = %message,
            file,
            line,
            column,
            backtrace = %backtrace,
            "thread panicked"
        );

        default_hook(info);
    }));
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_string()
}

fn config_path(key: &str, default: &str) -> eyre::Result<PathBuf> {
    Ok(config_value(key)?
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default)))
}

fn config_value(key: &str) -> eyre::Result<Option<String>> {
    if let Some(value) = env::var_os(key) {
        let value = value.to_string_lossy().trim().to_string();
        if !value.is_empty() {
            return Ok(Some(value));
        }
    }
    shared_config_value(key)
}

fn create_run_log_dir(log_root: &Path) -> eyre::Result<RunLogDir> {
    let run_id = config_value(CHAIN_SERVER_LOG_RUN_ID_CONFIG)?
        .map(|value| sanitize_run_id(&value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(default_run_id);
    let run_dir = log_root.join(&run_id);
    std::fs::create_dir_all(&run_dir)?;
    write_run_manifest(log_root, &run_dir, &run_id)?;
    Ok(RunLogDir {
        run_id,
        path: run_dir,
    })
}

fn default_run_id() -> String {
    let stamp = Utc::now().format("%Y%m%d-%H%M%SZ");
    format!("run-{stamp}-pid-{}", std::process::id())
}

fn write_run_manifest(log_root: &Path, run_dir: &Path, run_id: &str) -> eyre::Result<()> {
    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let manifest = json!({
        "schema_version": 1,
        "service": "eth_chain_server",
        "run_id": run_id,
        "pid": std::process::id(),
        "started_at_unix_ms": unix_ms,
        "log_root": log_root.display().to_string(),
        "run_dir": run_dir.display().to_string(),
        "files": [
            "server.log",
            "events.jsonl",
            "live_token_tracker.jsonl",
            "token_pipeline_profile.jsonl",
            "pool_buy_sell_sim_failures.jsonl",
            "simulation_failures.jsonl",
            "pipeline_issues.jsonl",
            "pipeline_health.jsonl",
            "pipeline_bottlenecks.jsonl"
        ]
    });
    let manifest = serde_json::to_vec_pretty(&manifest)?;
    std::fs::write(run_dir.join("run_manifest.json"), manifest)?;
    Ok(())
}

fn sanitize_run_id(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn is_server_log_metadata(metadata: &Metadata<'_>) -> bool {
    if is_token_pipeline_profile_metadata(metadata) {
        return false;
    }
    if matches!(*metadata.level(), Level::WARN | Level::ERROR) {
        return true;
    }
    matches!(*metadata.level(), Level::INFO)
        && (metadata.target().starts_with("eth_chain_server")
            || is_live_token_tracker_metadata(metadata))
}

fn is_event_metadata(metadata: &Metadata<'_>) -> bool {
    matches!(*metadata.level(), Level::WARN | Level::ERROR)
}

fn is_simulator_target(metadata: &Metadata<'_>) -> bool {
    matches!(
        metadata.target(),
        POOL_BUY_SELL_SIM_LOG_TARGET | "replay_parity_sim" | "token_lab"
    )
}

fn is_pool_buy_sell_sim_failure_metadata(metadata: &Metadata<'_>) -> bool {
    metadata.target() == POOL_BUY_SELL_SIM_LOG_TARGET && is_failure_metadata(metadata)
}

fn is_non_pool_simulator_failure_metadata(metadata: &Metadata<'_>) -> bool {
    metadata.target() != POOL_BUY_SELL_SIM_LOG_TARGET
        && is_simulator_target(metadata)
        && is_failure_metadata(metadata)
}

fn is_failure_metadata(metadata: &Metadata<'_>) -> bool {
    matches!(*metadata.level(), Level::WARN | Level::ERROR)
}

fn is_live_token_tracker_metadata(metadata: &Metadata<'_>) -> bool {
    matches!(*metadata.level(), Level::INFO | Level::WARN | Level::ERROR)
        && (metadata.target() == LIVE_TOKEN_TRACKER_LOG_TARGET
            || metadata.target().starts_with("eth_live_feed::runtime"))
}

fn is_token_pipeline_profile_metadata(metadata: &Metadata<'_>) -> bool {
    matches!(
        metadata.target(),
        TOKEN_RANGE_APPLY_PROFILE_LOG_TARGET
            | TOKEN_BLOCK_PROCESSOR_PROFILE_LOG_TARGET
            | TOKEN_SIM_SESSION_PROFILE_LOG_TARGET
            | LIVE_TOKEN_APPLY_PROFILE_LOG_TARGET
    ) && matches!(*metadata.level(), Level::INFO | Level::WARN | Level::ERROR)
}
