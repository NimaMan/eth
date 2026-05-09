use std::collections::{HashMap, HashSet};
use std::env;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alloy_primitives::{Address, B256, U256};
use clap::Parser;
use eth_alpha_core::{
    amount::Amount,
    execution::ExecutionReport,
    ids::TokenPoolId,
    market::{MarketEvent, PoolProtocol, PoolSnapshot},
    portfolio::PortfolioState,
    risk::{RiskEvent, RiskKind, RiskSeverity},
};
use eth_alpha_engine::{AlphaEngine, BlockCriticalRiskPolicy, EngineEvent, PaperExecutionAdapter};
use eth_alpha_store::{PostgresTradingStore, StrategyObservationRecord};
use eth_strategies::{SnipeAllConfig, SnipeAllStrategy};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time;
use tracing::{info, warn};

const STRATEGY_NAME: &str = "snipe-all-v1";
const STRATEGY_LABEL: &str = "Snipe All v1";
const POOL_UPDATE_SOURCE: &str = "pool_update";
const MEMPOOL_SIGNAL_SOURCE: &str = "mempool_signal";

#[derive(Debug, Parser)]
struct Args {
    #[arg(
        long,
        env = "ALPHA_TOKEN_SERVER_URL",
        default_value = "http://127.0.0.1:8765"
    )]
    token_server_url: String,

    #[arg(long, default_value_t = 2_000)]
    poll_interval_ms: u64,

    #[arg(long, default_value_t = 14)]
    mempool_since_days: i64,

    #[arg(long, default_value_t = 200)]
    signal_limit: i64,

    #[arg(long, default_value = "10000000000000000")]
    paper_buy_wei: String,

    #[arg(long, default_value = "0")]
    min_liquidity_eth: String,

    #[arg(long, env = "ALPHA_DATABASE_URL")]
    database_url: Option<String>,

    #[arg(long, env = "ALPHA_TRADER_RUN_ID")]
    run_id: Option<String>,

    #[arg(long, env = "ALPHA_TRADER_MODE", default_value = "paper")]
    mode: String,

    /// Process the current token-server snapshot immediately instead of only priming watermarks.
    #[arg(long, default_value_t = false)]
    replay_current: bool,

    #[arg(long, default_value_t = false)]
    once: bool,
}

#[derive(Clone)]
struct TokenServerClient {
    base_url: String,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize, Serialize)]
struct LiveStatusResponse {
    progress: LiveProgressWire,
}

#[derive(Debug, Deserialize, Serialize)]
struct LivePoolListResponse {
    count: usize,
    pools: Vec<PoolWire>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LiveProgressWire {
    status: String,
    current_block: Option<u64>,
    blocks_processed: u64,
    warmup_total_blocks: u64,
    tracked_tokens: usize,
    #[serde(default)]
    tracked_pools: usize,
    #[serde(default)]
    tracked_v2_pools: usize,
    #[serde(default)]
    tracked_v3_pools: usize,
    #[serde(default)]
    tracked_v4_pools: usize,
    last_error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PoolWire {
    token_address: String,
    pool_address: String,
    protocol: String,
    denom_reserve: Option<f64>,
    token_reserve: Option<f64>,
    price: Option<f64>,
    creation_block: Option<u64>,
    latest_block_number: Option<u64>,
    runtime_state: Option<PoolRuntimeStateWire>,
    can_buy: bool,
    can_sell: bool,
    is_scam: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct PoolRuntimeStateWire {
    last_update_block: Option<u64>,
    last_sync_block: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MempoolSignalsResponse {
    count: usize,
    signals: Vec<MempoolSignalWire>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MempoolSignalWire {
    signal_id: String,
    signal_type: String,
    detection_timestamp: Option<String>,
    detection_tx_hash: Option<String>,
    token_address: Option<String>,
    pool_address: Option<String>,
    headline: Option<String>,
    flag: Option<String>,
}

impl TokenServerClient {
    fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    async fn status(&self) -> Result<LiveStatusResponse> {
        self.get_json("/live/status").await
    }

    async fn pools(&self) -> Result<LivePoolListResponse> {
        self.get_json("/live/pools").await
    }

    async fn mempool_signals(&self, limit: i64, since_days: i64) -> Result<MempoolSignalsResponse> {
        let path = format!("/mempool/signals?limit={limit}&since_days={since_days}");
        self.get_json(&path).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = format!("{}{}", self.base_url, path);
        self.http
            .get(&url)
            .send()
            .await
            .wrap_err_with(|| format!("request failed: {url}"))?
            .error_for_status()
            .wrap_err_with(|| format!("token server returned an error: {url}"))?
            .json::<T>()
            .await
            .wrap_err_with(|| format!("failed to decode token server response: {url}"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let paper_buy_wei = parse_u256_decimal(&args.paper_buy_wei)?;
    let min_liquidity_eth = Decimal::from_str(&args.min_liquidity_eth)
        .wrap_err("invalid --min-liquidity-eth decimal")?;
    let database_url = resolve_database_url(&args)?;
    let run_id = args.run_id.clone().unwrap_or_else(default_run_id);

    let store = PostgresTradingStore::connect(&database_url, run_id.clone())
        .await
        .wrap_err("failed to initialize Postgres trading store")?;
    store
        .start_run(
            &args.mode,
            json!({
                "strategy_name": STRATEGY_NAME,
                "strategy_label": STRATEGY_LABEL,
                "token_server_url": &args.token_server_url,
                "poll_interval_ms": args.poll_interval_ms,
                "mempool_since_days": args.mempool_since_days,
                "signal_limit": args.signal_limit,
                "paper_buy_wei": &args.paper_buy_wei,
                "min_liquidity_eth": &args.min_liquidity_eth,
                "replay_current": args.replay_current,
            }),
        )
        .await
        .wrap_err("failed to record alpha trader run")?;
    let stale_runs = store
        .mark_stale_runs(60)
        .await
        .wrap_err("failed to mark stale alpha trader runs")?;
    let restored_positions = store
        .load_active_positions(STRATEGY_NAME)
        .await
        .wrap_err("failed to restore active alpha positions")?;
    let mut portfolio = PortfolioState::default();
    for position in restored_positions {
        portfolio.positions.insert(position.id.clone(), position);
    }
    let restored_position_count = portfolio.active_position_count();

    let mut engine = AlphaEngine::new(
        BlockCriticalRiskPolicy,
        store.clone(),
        PaperExecutionAdapter::new(),
    )
    .with_portfolio(portfolio);
    engine.add_strategy(Box::new(SnipeAllStrategy::new(SnipeAllConfig {
        buy_amount: Amount {
            raw: paper_buy_wei,
            decimals: 18,
        },
        sell_amount: Amount {
            raw: paper_buy_wei,
            decimals: 18,
        },
        min_denom_reserve: min_liquidity_eth,
        ..SnipeAllConfig::default()
    })));

    let client = TokenServerClient::new(args.token_server_url.clone());
    let (mut seen_pool_blocks, mut seen_signal_ids) = load_persisted_watermarks(&store).await?;
    let mut primed = false;
    let mut shutdown = ShutdownSignals::new()?;

    info!(
        token_server_url = %args.token_server_url,
        run_id = %run_id,
        mode = %args.mode,
        stale_runs,
        replay_current = args.replay_current,
        restored_pool_watermarks = seen_pool_blocks.len(),
        restored_signal_watermarks = seen_signal_ids.len(),
        restored_positions = restored_position_count,
        "starting alpha trader"
    );

    loop {
        let first_poll = !primed;
        let poll_result = async {
            let status = client.status().await?;
            let pools = client.pools().await?;
            let signals = client
                .mempool_signals(args.signal_limit, args.mempool_since_days)
                .await?;
            Ok::<_, eyre::Report>((status, pools, signals))
        }
        .await;
        let (status, pools, signals) = match poll_result {
            Ok(result) => result,
            Err(error) => {
                warn!(error = %error, "alpha trader poll failed");
                let metadata = json!({
                    "token_server_url": &args.token_server_url,
                    "poll_error": error.to_string(),
                    "trading_enabled": false,
                    "positions": engine.portfolio().active_position_count(),
                });
                store
                    .heartbeat(metadata.clone())
                    .await
                    .wrap_err("failed to write alpha trader error heartbeat")?;
                if args.once {
                    store
                        .mark_stopped("failed", metadata)
                        .await
                        .wrap_err("failed to mark alpha trader run failed")?;
                    break;
                }
                tokio::select! {
                    _ = time::sleep(Duration::from_millis(args.poll_interval_ms)) => {}
                    _ = shutdown.recv() => {
                        store
                            .mark_stopped(
                                "stopped",
                                json!({
                                    "reason": "shutdown_signal",
                                    "positions": engine.portfolio().active_position_count(),
                                }),
                            )
                            .await
                            .wrap_err("failed to mark alpha trader run stopped")?;
                        break;
                    }
                }
                continue;
            }
        };
        let live_ready = status.progress.status == "live";
        let suppress_events = !args.replay_current && !live_ready;

        let mut market_events = 0usize;
        let mut risk_events = 0usize;
        let mut reports = 0usize;

        for pool_wire in pools.pools {
            let pool = match pool_wire.to_pool_snapshot() {
                Ok(pool) => pool,
                Err(error) => {
                    warn!(error = %error, "skipping pool snapshot");
                    continue;
                }
            };
            let previous_block = seen_pool_blocks.get(&pool.address).copied();
            let changed = previous_block
                .map(|previous| pool.latest_block > previous)
                .unwrap_or(true);
            if !changed {
                continue;
            }
            seen_pool_blocks.insert(pool.address.clone(), pool.latest_block);

            if suppress_events || (first_poll && !args.replay_current) {
                record_pool_observation(
                    &store,
                    &pool_wire,
                    &pool,
                    previous_block,
                    "primed",
                    0,
                    first_poll,
                    suppress_events,
                    &status,
                    Value::Null,
                )
                .await?;
                continue;
            }

            let event = MarketEvent::PoolUpdated {
                block_number: pool.latest_block,
                pool: pool.clone(),
            };
            let event_reports = engine.handle_event(EngineEvent::Market(event)).await?;
            let report_count = event_reports.len();
            let decision = if report_count > 0 {
                "submitted"
            } else {
                "hold"
            };
            record_pool_observation(
                &store,
                &pool_wire,
                &pool,
                previous_block,
                decision,
                report_count,
                first_poll,
                suppress_events,
                &status,
                json!({ "reports": reports_payload(&event_reports) }),
            )
            .await?;
            reports += report_count;
            market_events += 1;
            for report in event_reports {
                info!(
                    order_id = %report.order_id.0,
                    status = ?report.status,
                    "paper execution report"
                );
            }
        }

        for signal in signals.signals {
            let is_new = seen_signal_ids.insert(signal.signal_id.clone());
            if !is_new || suppress_events || (first_poll && !args.replay_current) {
                if is_new {
                    record_signal_observation(
                        &store,
                        &signal,
                        "primed",
                        0,
                        first_poll,
                        suppress_events,
                        &status,
                        Value::Null,
                    )
                    .await?;
                }
                continue;
            }
            let event = match signal.to_risk_event() {
                Ok(Some(event)) => event,
                Ok(None) => {
                    record_signal_observation(
                        &store,
                        &signal,
                        "ignored",
                        0,
                        first_poll,
                        suppress_events,
                        &status,
                        json!({ "reason": "missing_token_address" }),
                    )
                    .await?;
                    continue;
                }
                Err(error) => {
                    record_signal_observation(
                        &store,
                        &signal,
                        "invalid",
                        0,
                        first_poll,
                        suppress_events,
                        &status,
                        json!({ "error": error.to_string() }),
                    )
                    .await?;
                    warn!(error = %error, signal_id = %signal.signal_id, "skipping mempool signal");
                    continue;
                }
            };
            let event_reports = engine.handle_event(EngineEvent::Risk(event)).await?;
            let report_count = event_reports.len();
            let decision = if report_count > 0 {
                "submitted"
            } else {
                "hold"
            };
            record_signal_observation(
                &store,
                &signal,
                decision,
                report_count,
                first_poll,
                suppress_events,
                &status,
                json!({ "reports": reports_payload(&event_reports) }),
            )
            .await?;
            reports += report_count;
            risk_events += 1;
            for report in event_reports {
                info!(
                    order_id = %report.order_id.0,
                    status = ?report.status,
                    "paper execution report"
                );
            }
        }

        if first_poll && !args.replay_current {
            info!(
                pools = seen_pool_blocks.len(),
                signals = seen_signal_ids.len(),
                "primed alpha trader watermarks"
            );
        }
        primed = true;

        info!(
            live_status = %status.progress.status,
            live_current_block = ?status.progress.current_block,
            live_blocks_processed = status.progress.blocks_processed,
            live_warmup_total_blocks = status.progress.warmup_total_blocks,
            live_tracked_tokens = status.progress.tracked_tokens,
            live_tracked_pools = status.progress.tracked_pool_count(),
            live_tracked_v2_pools = status.progress.tracked_v2_pools,
            live_tracked_v3_pools = status.progress.tracked_v3_pools,
            live_tracked_v4_pools = status.progress.tracked_v4_pools,
            live_last_error = ?status.progress.last_error,
            trading_enabled = !suppress_events,
            pools_seen = seen_pool_blocks.len(),
            token_server_pool_count = pools.count,
            signal_count = signals.count,
            market_events,
            risk_events,
            reports,
            positions = engine.portfolio().active_position_count(),
            "alpha trader tick"
        );
        let heartbeat_metadata = json!({
            "live_status": status.progress.status,
            "live_current_block": status.progress.current_block,
            "live_blocks_processed": status.progress.blocks_processed,
            "live_warmup_total_blocks": status.progress.warmup_total_blocks,
            "live_tracked_tokens": status.progress.tracked_tokens,
            "live_tracked_pools": status.progress.tracked_pool_count(),
            "live_tracked_v2_pools": status.progress.tracked_v2_pools,
            "live_tracked_v3_pools": status.progress.tracked_v3_pools,
            "live_tracked_v4_pools": status.progress.tracked_v4_pools,
            "live_last_error": status.progress.last_error,
            "trading_enabled": !suppress_events,
            "pools_seen": seen_pool_blocks.len(),
            "token_server_pool_count": pools.count,
            "signal_count": signals.count,
            "market_events": market_events,
            "risk_events": risk_events,
            "reports": reports,
            "positions": engine.portfolio().active_position_count(),
        });
        store
            .heartbeat(heartbeat_metadata.clone())
            .await
            .wrap_err("failed to write alpha trader heartbeat")?;

        if args.once {
            store
                .mark_stopped("completed", heartbeat_metadata)
                .await
                .wrap_err("failed to mark alpha trader run completed")?;
            break;
        }
        tokio::select! {
            _ = time::sleep(Duration::from_millis(args.poll_interval_ms)) => {}
            _ = shutdown.recv() => {
                store
                    .mark_stopped(
                        "stopped",
                        json!({
                            "reason": "shutdown_signal",
                            "positions": engine.portfolio().active_position_count(),
                        }),
                    )
                    .await
                    .wrap_err("failed to mark alpha trader run stopped")?;
                break;
            }
        }
    }

    Ok(())
}

async fn load_persisted_watermarks(
    store: &PostgresTradingStore,
) -> Result<(HashMap<TokenPoolId, u64>, HashSet<String>)> {
    let cursors = store
        .load_strategy_observation_cursors(STRATEGY_NAME)
        .await
        .wrap_err("failed to load alpha trader observation watermarks")?;
    let mut pool_blocks = HashMap::new();
    let mut signal_ids = HashSet::new();

    for cursor in cursors {
        match cursor.event_source.as_str() {
            POOL_UPDATE_SOURCE => {
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
            _ => {}
        }
    }

    Ok((pool_blocks, signal_ids))
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

async fn record_pool_observation(
    store: &PostgresTradingStore,
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
            strategy_name: STRATEGY_NAME.to_string(),
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

async fn record_signal_observation(
    store: &PostgresTradingStore,
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
            strategy_name: STRATEGY_NAME.to_string(),
            event_source: MEMPOOL_SIGNAL_SOURCE.to_string(),
            event_key: signal.signal_id.clone(),
            token_address: signal.token_address.clone(),
            pool_address: signal.pool_address.clone(),
            block_number: None,
            event_timestamp: signal.detection_timestamp.clone(),
            decision: decision.to_string(),
            report_count,
            payload: json!({
                "signal": signal,
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

fn reports_payload(reports: &[ExecutionReport]) -> Vec<Value> {
    reports
        .iter()
        .filter_map(|report| serde_json::to_value(report).ok())
        .collect()
}

impl PoolWire {
    fn latest_block_number(&self) -> Option<u64> {
        self.latest_block_number
            .filter(|block| *block > 0)
            .or_else(|| {
                self.runtime_state.as_ref().and_then(|state| {
                    state
                        .last_update_block
                        .filter(|block| *block > 0)
                        .or_else(|| state.last_sync_block.filter(|block| *block > 0))
                })
            })
            .or_else(|| self.creation_block.filter(|block| *block > 0))
    }

    fn pool_identity(&self) -> String {
        self.pool_address.clone()
    }

    fn to_pool_snapshot(&self) -> Result<PoolSnapshot> {
        let token_address = parse_address(&self.token_address)?;
        let pool_id = TokenPoolId::new(token_address, self.pool_identity());
        let denom_reserve = required_pool_float(self.denom_reserve, "denom_reserve", self)?;
        let token_reserve = self.token_reserve.unwrap_or_default();
        let Some(latest_block) = self.latest_block_number() else {
            return Err(eyre!(
                "pool {} for token {} has no latest block",
                self.pool_address,
                self.token_address
            ));
        };
        Ok(PoolSnapshot {
            address: pool_id,
            token_address,
            protocol: parse_protocol(&self.protocol),
            denom_reserve: decimal_from_f64(denom_reserve),
            token_reserve: decimal_from_f64(token_reserve),
            price_denom_per_token: self.price.map(decimal_from_f64),
            latest_block,
            can_buy: self.can_buy,
            can_sell: self.can_sell,
            is_scam: self.is_scam,
        })
    }
}

impl LiveProgressWire {
    fn tracked_pool_count(&self) -> usize {
        if self.tracked_pools > 0 {
            self.tracked_pools
        } else {
            self.tracked_v2_pools + self.tracked_v3_pools + self.tracked_v4_pools
        }
    }
}

impl MempoolSignalWire {
    fn to_risk_event(&self) -> Result<Option<RiskEvent>> {
        let Some(token_address) = self.token_address.as_ref() else {
            return Ok(None);
        };
        let token_address = parse_address(token_address)?;
        let pool_address = self
            .pool_address
            .as_ref()
            .map(|value| TokenPoolId::new(token_address, value));
        let pending_tx_hash = self
            .detection_tx_hash
            .as_ref()
            .map(|value| B256::from_str(value))
            .transpose()
            .map_err(|error| eyre!("invalid tx hash: {error}"))?;
        let (kind, severity) = signal_kind_and_severity(self);
        Ok(Some(RiskEvent {
            kind,
            severity,
            token_address,
            pool_address,
            pending_tx_hash,
            observed_block: None,
            message: self.message(),
        }))
    }

    fn message(&self) -> String {
        match (&self.detection_timestamp, &self.headline) {
            (Some(ts), Some(headline)) => format!("{headline} at {ts}"),
            (Some(ts), None) => format!("{} at {ts}", self.signal_type),
            (None, Some(headline)) => headline.clone(),
            (None, None) => self.signal_type.clone(),
        }
    }
}

fn signal_kind_and_severity(signal: &MempoolSignalWire) -> (RiskKind, RiskSeverity) {
    match signal.signal_type.as_str() {
        "trading_enabled" => (RiskKind::TradingEnabled, RiskSeverity::Info),
        "liquidity_removal" => (RiskKind::LiquidityRemoval, RiskSeverity::Critical),
        "lp_approval" => (RiskKind::LpApproval, RiskSeverity::Warning),
        "tax_signal" => {
            let critical = signal
                .flag
                .as_deref()
                .map(|flag| matches!(flag, "true" | "t" | "1"))
                .unwrap_or(false)
                || signal
                    .headline
                    .as_deref()
                    .map(|headline| headline.to_ascii_lowercase().contains("honeypot"))
                    .unwrap_or(false);
            (
                RiskKind::TaxChange,
                if critical {
                    RiskSeverity::Critical
                } else {
                    RiskSeverity::Warning
                },
            )
        }
        other => (RiskKind::Custom(other.to_string()), RiskSeverity::Warning),
    }
}

fn parse_protocol(value: &str) -> PoolProtocol {
    match value.to_ascii_lowercase().as_str() {
        "uniswapv2" | "uniswap_v2" | "v2" => PoolProtocol::UniswapV2,
        "uniswapv3" | "uniswap_v3" | "v3" => PoolProtocol::UniswapV3,
        "uniswapv4" | "uniswap_v4" | "v4" => PoolProtocol::UniswapV4,
        other => PoolProtocol::Unknown(other.to_string()),
    }
}

fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|error| eyre!("invalid address {value}: {error}"))
}

fn required_pool_float(value: Option<f64>, field: &str, pool: &PoolWire) -> Result<f64> {
    value.ok_or_else(|| {
        eyre!(
            "pool {} for token {} is missing {field}",
            pool.pool_address,
            pool.token_address
        )
    })
}

fn parse_u256_decimal(value: &str) -> Result<U256> {
    U256::from_str_radix(value, 10).map_err(|error| eyre!("invalid decimal U256 {value}: {error}"))
}

fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or(Decimal::ZERO)
}

fn resolve_database_url(args: &Args) -> Result<String> {
    args.database_url
        .clone()
        .or_else(|| env::var("MEMPOOL_DATABASE_URL").ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| eyre!("set ALPHA_DATABASE_URL or MEMPOOL_DATABASE_URL for alpha trader"))
}

fn default_run_id() -> String {
    let unix_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("alpha-trader-{unix_secs}-{}", std::process::id())
}

#[cfg(unix)]
struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignals {
    fn new() -> Result<Self> {
        Ok(Self {
            interrupt: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .wrap_err("failed to install SIGINT handler")?,
            terminate: tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .wrap_err("failed to install SIGTERM handler")?,
        })
    }

    async fn recv(&mut self) {
        tokio::select! {
            _ = self.interrupt.recv() => {}
            _ = self.terminate.recv() => {}
        }
    }
}

#[cfg(not(unix))]
struct ShutdownSignals;

#[cfg(not(unix))]
impl ShutdownSignals {
    fn new() -> Result<Self> {
        Ok(Self)
    }

    async fn recv(&mut self) {
        let _ = tokio::signal::ctrl_c().await;
    }
}
