use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use alloy_primitives::U256;
use chrono::Utc;
use eth_alpha_core::{
    amount::Amount, execution::ExecutionReport, ids::TokenPoolId, market::PoolSnapshot,
    risk::RiskEvent,
};
use eth_alpha_store::{PostgresTradingStore, StrategyObservationRecord};
use eth_ops_events::{JsonlOpsEventSink, MultiOpsEventSink, TracingOpsEventSink};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use tracing::info;

use crate::wire::{parse_address, LiveStatusResponse, MempoolSignalWire, PoolWire};

use super::{
    ALPHA_DATABASE_CONFIG_KEY, ALPHA_TRADER_LOG_DIR_CONFIG, CHAIN_SERVER_BIND_CONFIG,
    DEFAULT_ALPHA_TRADER_LOG_DIR, LEGACY_MINED_POOL_RISK_SOURCE,
    LEGACY_RETH_MINED_POOL_RISK_SOURCE, MEMPOOL_SIGNAL_SOURCE, MINED_POOL_RISK_SOURCE,
    POOL_UPDATE_SOURCE,
};

pub(super) fn init_alpha_trader_ops_events(
    run_id: &str,
    config: &HashMap<String, String>,
) -> Result<()> {
    let root = optional_shared_config_value(config, ALPHA_TRADER_LOG_DIR_CONFIG)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ALPHA_TRADER_LOG_DIR));
    let run_dir = root.join(sanitize_path_segment(run_id));
    let sink = MultiOpsEventSink::new(vec![
        Arc::new(JsonlOpsEventSink::open(&run_dir)?),
        Arc::new(TracingOpsEventSink),
    ]);
    let initialized = eth_ops_events::init_global_sink(Arc::new(sink));
    info!(
        run_id,
        run_dir = %run_dir.display(),
        initialized,
        "initialized alpha trader ops events"
    );
    Ok(())
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "alpha-trader".to_string()
    } else {
        sanitized
    }
}

pub(super) async fn load_persisted_watermarks(
    store: &PostgresTradingStore,
    strategy_name: &str,
) -> Result<(HashMap<TokenPoolId, u64>, HashSet<String>, HashSet<String>)> {
    let cursors = store
        .load_strategy_observation_cursors(strategy_name)
        .await
        .wrap_err("failed to load alpha trader observation watermarks")?;
    let mut pool_blocks = HashMap::new();
    let mut signal_ids = HashSet::new();
    let mut mined_pool_risk_keys = HashSet::new();

    for cursor in cursors {
        match cursor.event_source.as_str() {
            POOL_UPDATE_SOURCE => {
                if is_mined_pool_risk_event_key(&cursor.event_key) {
                    mined_pool_risk_keys.insert(cursor.event_key);
                    continue;
                }
                if cursor.event_key.starts_with("position_monitor:") {
                    continue;
                }
                let Some(block_number) = cursor.block_number else {
                    continue;
                };
                let Some(pool_id) = cursor_pool_id(&cursor)? else {
                    continue;
                };
                pool_blocks
                    .entry(pool_id)
                    .and_modify(|current: &mut u64| *current = (*current).max(block_number))
                    .or_insert(block_number);
            }
            MEMPOOL_SIGNAL_SOURCE => {
                signal_ids.insert(cursor.event_key);
            }
            LEGACY_MINED_POOL_RISK_SOURCE | LEGACY_RETH_MINED_POOL_RISK_SOURCE => {
                mined_pool_risk_keys.insert(cursor.event_key);
            }
            _ => {}
        }
    }

    Ok((pool_blocks, signal_ids, mined_pool_risk_keys))
}

fn is_mined_pool_risk_event_key(event_key: &str) -> bool {
    event_key.starts_with("lp_approval:") || event_key.starts_with("liquidity_removal:")
}

fn cursor_pool_id(
    cursor: &eth_alpha_store::StrategyObservationCursor,
) -> Result<Option<TokenPoolId>> {
    let Some(pool_identity) = cursor.pool_address.as_ref() else {
        return Ok(None);
    };
    if pool_identity.contains(':') {
        return Ok(Some(TokenPoolId::from(pool_identity.as_str())));
    }
    let Some(token_address) = cursor.token_address.as_ref() else {
        return Ok(None);
    };
    let token_address = parse_address(token_address)?;
    Ok(Some(TokenPoolId::new(token_address, pool_identity)))
}

pub(super) async fn record_pool_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    pool_wire: &PoolWire,
    pool: &PoolSnapshot,
    previous_block: Option<u64>,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    store
        .record_strategy_observation(StrategyObservationRecord {
            strategy_name: strategy_name.to_string(),
            event_source: POOL_UPDATE_SOURCE.to_string(),
            event_key: format!("{}:{}", pool.address, pool.latest_block),
            token_address: Some(pool.token_address.to_string()),
            pool_address: Some(pool.address.to_string()),
            block_number: Some(pool.latest_block),
            event_timestamp: None,
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "pool": pool_wire,
                "previous_block": previous_block,
                "latest_block": pool.latest_block,
                "first_poll": first_poll,
                "suppress_events": suppress_events,
                "live_status": status.progress.status,
                "live_current_block": status.progress.current_block,
                "live_blocks_processed": status.progress.blocks_processed,
                "live_warmup_total_blocks": status.progress.warmup_total_blocks,
                "extra": extra,
            }),
        })
        .await
        .wrap_err("failed to record pool strategy observation")
}

pub(super) async fn record_position_monitor_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    block_number: u64,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    store
        .record_strategy_observation(StrategyObservationRecord {
            strategy_name: strategy_name.to_string(),
            event_source: POOL_UPDATE_SOURCE.to_string(),
            event_key: format!("position_monitor:{block_number}"),
            token_address: None,
            pool_address: None,
            block_number: Some(block_number),
            event_timestamp: None,
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "block_number": block_number,
                "first_poll": first_poll,
                "suppress_events": suppress_events,
                "live_status": status.progress.status,
                "live_current_block": status.progress.current_block,
                "live_blocks_processed": status.progress.blocks_processed,
                "live_warmup_total_blocks": status.progress.warmup_total_blocks,
                "extra": extra,
            }),
        })
        .await
        .wrap_err("failed to record position monitor strategy observation")
}

pub(super) async fn record_mined_pool_risk_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    event_key: &str,
    risk: &RiskEvent,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    store
        .record_strategy_observation(StrategyObservationRecord {
            strategy_name: strategy_name.to_string(),
            event_source: MINED_POOL_RISK_SOURCE.to_string(),
            event_key: event_key.to_string(),
            token_address: Some(risk.token_address.to_string()),
            pool_address: risk.pool_address.as_ref().map(ToString::to_string),
            block_number: risk.observed_block,
            event_timestamp: None,
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "risk": risk,
                "first_poll": first_poll,
                "suppress_events": suppress_events,
                "live_status": status.progress.status,
                "live_current_block": status.progress.current_block,
                "live_blocks_processed": status.progress.blocks_processed,
                "live_warmup_total_blocks": status.progress.warmup_total_blocks,
                "extra": extra,
            }),
        })
        .await
        .wrap_err("failed to record mined pool risk strategy observation")
}

pub(super) async fn record_signal_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    signal: &MempoolSignalWire,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    record_signal_observation_with_source(
        store,
        strategy_name,
        MEMPOOL_SIGNAL_SOURCE,
        &signal.signal_id,
        signal,
        decision,
        report_count,
        first_poll,
        suppress_events,
        status,
        extra,
    )
    .await
}

pub(super) async fn record_deferred_signal_observation(
    store: &PostgresTradingStore,
    strategy_name: &str,
    signal: &MempoolSignalWire,
    decision: &str,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    record_signal_observation_with_source(
        store,
        strategy_name,
        MEMPOOL_SIGNAL_SOURCE,
        &format!("deferred:{}", signal.signal_id),
        signal,
        decision,
        0,
        first_poll,
        suppress_events,
        status,
        extra,
    )
    .await
}

async fn record_signal_observation_with_source(
    store: &PostgresTradingStore,
    strategy_name: &str,
    event_source: &str,
    event_key: &str,
    signal: &MempoolSignalWire,
    decision: &str,
    report_count: usize,
    first_poll: bool,
    suppress_events: bool,
    status: &LiveStatusResponse,
    extra: Value,
) -> Result<()> {
    store
        .record_strategy_observation(StrategyObservationRecord {
            strategy_name: strategy_name.to_string(),
            event_source: event_source.to_string(),
            event_key: event_key.to_string(),
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            block_number: status.progress.current_block,
            event_timestamp: signal.detection_timestamp.clone(),
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "signal": signal,
                "signal_source": signal.signal_source.clone(),
                "signal_created_at": signal.signal_created_at.clone(),
                "mempool_first_seen_at": signal.mempool_first_seen_at.clone(),
                "mempool_first_seen_ms": signal.mempool_first_seen_ms,
                "first_poll": first_poll,
                "suppress_events": suppress_events,
                "live_status": status.progress.status,
                "live_current_block": status.progress.current_block,
                "live_blocks_processed": status.progress.blocks_processed,
                "live_warmup_total_blocks": status.progress.warmup_total_blocks,
                "extra": extra,
            }),
        })
        .await
        .wrap_err("failed to record mempool signal strategy observation")
}

pub(super) fn reports_payload(reports: &[ExecutionReport]) -> Vec<Value> {
    reports
        .iter()
        .filter_map(|report| serde_json::to_value(report).ok())
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TraderExecutionMode {
    ChainSim,
    KartalReal,
}

impl TraderExecutionMode {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ChainSim => "chain-sim",
            Self::KartalReal => "kartal-real",
        }
    }

    pub(super) fn execution_model(self) -> &'static str {
        match self {
            Self::ChainSim => "chain_state_evm_simulation",
            Self::KartalReal => "kartal_tx_executor",
        }
    }

    pub(super) fn uses_kartal(self) -> bool {
        matches!(self, Self::KartalReal)
    }
}

fn shared_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config.env")
}

fn shared_toml_config_path() -> PathBuf {
    shared_config_path().with_file_name("config.toml")
}

pub(super) fn load_shared_config() -> Result<HashMap<String, String>> {
    let path = shared_config_path();
    let contents = fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read shared config file {}", path.display()))?;
    let mut values = parse_shared_config(&contents);
    merge_toml_database_config(&mut values)?;
    Ok(values)
}

fn merge_toml_database_config(values: &mut HashMap<String, String>) -> Result<()> {
    let path = shared_toml_config_path();
    let contents = fs::read_to_string(&path)
        .wrap_err_with(|| format!("failed to read shared TOML config file {}", path.display()))?;
    let root = contents
        .parse::<toml::Value>()
        .wrap_err_with(|| format!("failed to parse shared TOML config file {}", path.display()))?;
    if let Some(url) = root
        .get("databases")
        .and_then(|value| value.get("alpha"))
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        values.insert(ALPHA_DATABASE_CONFIG_KEY.to_string(), url.to_string());
    }
    Ok(())
}

fn parse_shared_config(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        values.insert(
            key.to_string(),
            unquote_config_value(value.trim()).to_string(),
        );
    }
    values
}

fn optional_shared_config_value(config: &HashMap<String, String>, key: &str) -> Option<String> {
    config
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn required_shared_config_value(
    config: &HashMap<String, String>,
    key: &str,
) -> Result<String> {
    optional_shared_config_value(config, key).ok_or_else(|| {
        eyre!(
            "{key} must be set in shared config file {}",
            if key == ALPHA_DATABASE_CONFIG_KEY {
                shared_toml_config_path()
            } else {
                shared_config_path()
            }
            .display()
        )
    })
}

pub(super) fn chain_server_url_from_config(config: &HashMap<String, String>) -> Result<String> {
    let bind = required_shared_config_value(config, CHAIN_SERVER_BIND_CONFIG)?;
    Ok(format!("http://{bind}"))
}

fn unquote_config_value(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

pub(super) fn parse_u256_decimal(value: &str) -> Result<U256> {
    U256::from_str_radix(value, 10).map_err(|error| eyre!("invalid decimal U256 {value}: {error}"))
}

pub(super) fn parse_eth_decimal_to_wei(value: &str, label: &str) -> Result<U256> {
    let decimal = value
        .parse::<Decimal>()
        .wrap_err_with(|| format!("invalid {label} decimal {value:?}"))?;
    if decimal < Decimal::ZERO {
        return Err(eyre!("{label} must be non-negative; got {value}"));
    }
    Ok(Amount::from_decimal(decimal, 18).raw)
}

pub(super) fn resolve_database_url(config: &HashMap<String, String>) -> Result<String> {
    required_shared_config_value(config, ALPHA_DATABASE_CONFIG_KEY)
}

pub(super) fn default_run_id() -> String {
    let stamp = Utc::now().format("%Y%m%d-%H%M%SZ");
    format!("alpha-trader-{stamp}-pid-{}", std::process::id())
}

#[cfg(unix)]
pub(super) struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            interrupt: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .wrap_err("failed to install SIGINT handler")?,
            terminate: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .wrap_err("failed to install SIGTERM handler")?,
        })
    }

    pub(super) async fn recv(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.terminate.recv() => {}
        }
    }
}

#[cfg(not(unix))]
pub(super) struct ShutdownSignals;

#[cfg(not(unix))]
impl ShutdownSignals {
    pub(super) fn new() -> Result<Self> {
        Ok(Self)
    }

    pub(super) async fn recv(&mut self) {
        let _ = tokio::signal::ctrl_c().await;
    }
}
