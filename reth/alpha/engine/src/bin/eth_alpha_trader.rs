use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use alloy_primitives::{Address, B256, U256};
use clap::Parser;
use eth_alpha_core::{
    amount::Amount,
    market::{MarketEvent, PoolProtocol, PoolSnapshot},
    risk::{RiskEvent, RiskKind, RiskSeverity},
};
use eth_alpha_engine::{
    AlphaEngine, BlockCriticalRiskPolicy, EngineEvent, MemoryTradingStore, PaperExecutionAdapter,
};
use eth_strategies::{MarketTrackerConfig, MarketTrackerStrategy};
use eyre::{eyre, Result, WrapErr};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::time;
use tracing::{info, warn};

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

#[derive(Debug, Deserialize)]
struct LiveStatusResponse {
    progress: LiveProgressWire,
}

#[derive(Debug, Deserialize)]
struct LivePoolListResponse {
    count: usize,
    pools: Vec<PoolWire>,
}

#[derive(Debug, Deserialize)]
struct LiveProgressWire {
    status: String,
    current_block: Option<u64>,
    blocks_processed: u64,
    warmup_total_blocks: u64,
    tracked_tokens: usize,
    tracked_v2_pools: usize,
    last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PoolWire {
    token_address: String,
    pool_address: String,
    protocol: String,
    denom_reserve: f64,
    token_reserve: f64,
    price: f64,
    latest_block_number: Option<u64>,
    can_buy: bool,
    can_sell: bool,
    is_scam: bool,
}

#[derive(Debug, Deserialize)]
struct MempoolSignalsResponse {
    count: usize,
    signals: Vec<MempoolSignalWire>,
}

#[derive(Debug, Deserialize)]
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

    let store = MemoryTradingStore::default();
    let mut engine = AlphaEngine::new(
        BlockCriticalRiskPolicy,
        store.clone(),
        PaperExecutionAdapter::new(),
    );
    engine.add_strategy(Box::new(MarketTrackerStrategy::new(MarketTrackerConfig {
        buy_amount: Amount {
            raw: paper_buy_wei,
            decimals: 18,
        },
        min_denom_reserve: min_liquidity_eth,
        ..MarketTrackerConfig::default()
    })));

    let client = TokenServerClient::new(args.token_server_url.clone());
    let mut seen_pool_blocks: HashMap<Address, u64> = HashMap::new();
    let mut seen_signal_ids: HashSet<String> = HashSet::new();
    let mut primed = false;

    info!(
        token_server_url = %args.token_server_url,
        replay_current = args.replay_current,
        "starting alpha trader"
    );

    loop {
        let first_poll = !primed;
        let status = client.status().await?;
        let pools = client.pools().await?;
        let signals = client
            .mempool_signals(args.signal_limit, args.mempool_since_days)
            .await?;

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
            let previous_block = seen_pool_blocks.insert(pool.address, pool.latest_block);
            let changed = previous_block
                .map(|previous| pool.latest_block > previous)
                .unwrap_or(true);
            if !changed || (first_poll && !args.replay_current) {
                continue;
            }

            let event = MarketEvent::PoolUpdated {
                block_number: pool.latest_block,
                pool,
            };
            let event_reports = engine.handle_event(EngineEvent::Market(event)).await?;
            reports += event_reports.len();
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
            if !is_new || (first_poll && !args.replay_current) {
                continue;
            }
            let event = match signal.to_risk_event() {
                Ok(Some(event)) => event,
                Ok(None) => continue,
                Err(error) => {
                    warn!(error = %error, signal_id = %signal.signal_id, "skipping mempool signal");
                    continue;
                }
            };
            let event_reports = engine.handle_event(EngineEvent::Risk(event)).await?;
            reports += event_reports.len();
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
            live_tracked_pools = status.progress.tracked_v2_pools,
            live_last_error = ?status.progress.last_error,
            pools_seen = seen_pool_blocks.len(),
            token_server_pool_count = pools.count,
            signal_count = signals.count,
            market_events,
            risk_events,
            reports,
            positions = engine.portfolio().active_position_count(),
            "alpha trader tick"
        );

        if args.once {
            break;
        }
        time::sleep(Duration::from_millis(args.poll_interval_ms)).await;
    }

    Ok(())
}

impl PoolWire {
    fn to_pool_snapshot(&self) -> Result<PoolSnapshot> {
        let address = parse_address(&self.pool_address)?;
        let token_address = parse_address(&self.token_address)?;
        let latest_block = self.latest_block_number.unwrap_or_default();
        Ok(PoolSnapshot {
            address,
            token_address,
            protocol: parse_protocol(&self.protocol),
            denom_reserve: decimal_from_f64(self.denom_reserve),
            token_reserve: decimal_from_f64(self.token_reserve),
            price_denom_per_token: Some(decimal_from_f64(self.price)),
            latest_block,
            can_buy: self.can_buy,
            can_sell: self.can_sell,
            is_scam: self.is_scam,
        })
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
            .map(|value| parse_address(value))
            .transpose()?;
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

fn parse_u256_decimal(value: &str) -> Result<U256> {
    U256::from_str_radix(value, 10).map_err(|error| eyre!("invalid decimal U256 {value}: {error}"))
}

fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::from_f64(value).unwrap_or(Decimal::ZERO)
}
