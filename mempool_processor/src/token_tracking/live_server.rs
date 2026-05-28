use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eyre::{eyre, Result};
use reqwest::Client;
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use super::cache::{CacheStatusPolicy, TokenTrackingCache};
use super::snapshot_apply::apply_snapshot_map_to_cache_with_context;
use super::types::{
    Address, ConcentratedLiquidityPosition, Pool, PoolLifecycle, PoolType, Token, TokenWithPools,
};

const TOKEN_SERVER_UPDATE_WAIT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct LiveTokenServerHydrationReport {
    pub status: Option<String>,
    pub block_number: u64,
    pub tokens: usize,
    pub pools: usize,
    pub accepted: bool,
}

pub async fn hydrate_cache_from_live_token_server(
    cache: &TokenTrackingCache,
    base_url: &str,
) -> Result<LiveTokenServerHydrationReport> {
    let client = Client::new();
    let snapshot = fetch_live_token_server_snapshot(&client, base_url).await?;
    let report = snapshot.report;
    let outcome = apply_snapshot_map_to_cache_with_context(
        cache,
        report.block_number,
        0.0,
        "live_token_server_hydrate",
        report.status.clone(),
        CacheStatusPolicy::LiveOrWarmingUntilLive,
        snapshot.tokens,
    )
    .await;
    Ok(LiveTokenServerHydrationReport {
        accepted: outcome.applied(),
        ..report
    })
}

pub fn start_live_token_server_cache_sync(
    cache: Arc<TokenTrackingCache>,
    base_url: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let client = Client::new();
        let mut last_requested_block = 0u64;

        loop {
            match fetch_live_token_server_snapshot(&client, &base_url).await {
                Ok(snapshot) => {
                    let report = snapshot.report;
                    last_requested_block = last_requested_block.max(report.block_number);
                    let outcome = apply_snapshot_map_to_cache_with_context(
                        cache.as_ref(),
                        report.block_number,
                        0.0,
                        "live_token_server_sync",
                        report.status.clone(),
                        CacheStatusPolicy::LiveOrWarmingUntilLive,
                        snapshot.tokens,
                    )
                    .await;
                    if outcome.applied() {
                        debug!(
                            "Live token server cache sync applied: status={:?}, block={}, tokens={}, pools={}",
                            report.status, report.block_number, report.tokens, report.pools
                        );
                    } else {
                        debug!(
                            "Live token server cache sync skipped: status={:?}, block={}, tokens={}, pools={}, outcome={:?}",
                            report.status, report.block_number, report.tokens, report.pools, outcome
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        "Live token server cache sync failed for {}: {}",
                        base_url, err
                    );
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }

            match wait_for_live_token_server_update(
                &client,
                &base_url,
                last_requested_block,
                TOKEN_SERVER_UPDATE_WAIT,
            )
            .await
            {
                Ok(update) => {
                    if let Some(block_number) = update.block_number {
                        last_requested_block = last_requested_block.max(block_number);
                    }
                    if update.event != "timeout" {
                        debug!(
                            "Live token server update notification: event={}, status={:?}, block={:?}",
                            update.event, update.status, update.block_number
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        "Live token server update wait failed for {}: {}",
                        base_url, err
                    );
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    })
}

struct LiveTokenServerSnapshot {
    report: LiveTokenServerHydrationReport,
    tokens: HashMap<Address, TokenWithPools>,
}

async fn fetch_live_token_server_snapshot(
    client: &Client,
    base_url: &str,
) -> Result<LiveTokenServerSnapshot> {
    let started = Instant::now();
    let token_url = endpoint(base_url, "api/v1/eth/live-token-tracker/tokens");
    let pool_url = endpoint(base_url, "api/v1/eth/live-token-tracker/pools");

    let (tokens_response, pools_response) = tokio::try_join!(
        fetch_json::<LiveTokenListResponse>(client, &token_url),
        fetch_json::<LivePoolListResponse>(client, &pool_url)
    )?;

    let mut pools_by_token: HashMap<Address, HashMap<Address, Pool>> = HashMap::new();
    for pool in pools_response.pools {
        let Some(cache_pool) = pool.into_cache_pool() else {
            continue;
        };
        pools_by_token
            .entry(cache_pool.token_address.clone())
            .or_default()
            .insert(cache_pool.address.clone(), cache_pool);
    }

    let mut token_map = HashMap::new();
    let mut pool_count = 0usize;
    for token_view in tokens_response.tokens {
        let Some(token) = token_view.into_cache_token() else {
            continue;
        };
        let token_address = token.address.clone();
        let pools = pools_by_token.remove(&token_address).unwrap_or_default();
        pool_count += pools.len();
        token_map.insert(token_address, TokenWithPools { token, pools });
    }

    let block_number = tokens_response
        .progress
        .current_block
        .or(pools_response.progress.current_block)
        .unwrap_or_default();
    let report = LiveTokenServerHydrationReport {
        status: tokens_response
            .progress
            .status
            .or(pools_response.progress.status),
        block_number,
        tokens: token_map.len(),
        pools: pool_count,
        accepted: false,
    };

    info!(
        "Fetched live token tracker cache snapshot in {:.1}ms: status={:?}, block={}, tokens={}, pools={}",
        started.elapsed().as_secs_f64() * 1000.0,
        report.status,
        report.block_number,
        report.tokens,
        report.pools
    );

    Ok(LiveTokenServerSnapshot {
        report,
        tokens: token_map,
    })
}

#[derive(Debug, Deserialize, Default)]
struct LiveTokenServerUpdateNotification {
    #[serde(default)]
    event: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    block_number: Option<u64>,
}

async fn wait_for_live_token_server_update(
    client: &Client,
    base_url: &str,
    after_block: u64,
    max_wait: Duration,
) -> Result<LiveTokenServerUpdateNotification> {
    let update_url = format!(
        "{}?after_block={}&timeout_ms={}",
        endpoint(
            base_url,
            "api/v1/eth/live-token-tracker/block-applied-updates"
        ),
        after_block,
        max_wait.as_millis()
    );
    fetch_json::<LiveTokenServerUpdateNotification>(client, &update_url).await
}

async fn fetch_json<T>(client: &Client, url: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(eyre!("{} returned HTTP {}", url, status));
    }

    response
        .json::<T>()
        .await
        .map_err(|err| eyre!("failed to decode {}: {}", url, err))
}

fn endpoint(base_url: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

#[derive(Debug, Deserialize, Default)]
struct LiveTokenListResponse {
    #[serde(default)]
    progress: LiveProgressView,
    #[serde(default)]
    tokens: Vec<LiveTokenView>,
}

#[derive(Debug, Deserialize, Default)]
struct LivePoolListResponse {
    #[serde(default)]
    progress: LiveProgressView,
    #[serde(default)]
    pools: Vec<LivePoolView>,
}

#[derive(Debug, Deserialize, Default)]
struct LiveProgressView {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    current_block: Option<u64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LiveTokenView {
    contract_address: String,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: String,
    creation_block: Option<u64>,
    creation_timestamp: Option<u64>,
    creator_address: Option<String>,
    latest_block: Option<u64>,
    is_scam: bool,
    scam_label: Option<String>,
    current_owner: Option<String>,
    ownership_renounced: bool,
}

impl LiveTokenView {
    fn into_cache_token(self) -> Option<Token> {
        let address = normalize_address(&self.contract_address)?;
        let creator_address = normalize_optional_address(self.creator_address).unwrap_or_default();
        let current_owner = normalize_optional_address(self.current_owner)
            .unwrap_or_else(|| creator_address.clone());

        Some(Token {
            address,
            symbol: default_if_empty(self.symbol, "UNKNOWN"),
            name: default_if_empty(self.name, "Unknown"),
            decimals: if self.decimals == 0 {
                18
            } else {
                self.decimals
            },
            total_supply: if self.total_supply.is_empty() {
                None
            } else {
                Some(self.total_supply)
            },
            creator_address,
            current_owner,
            tax_setter_addresses: Vec::new(),
            ownership_renounced: self.ownership_renounced,
            renouncement_block: None,
            buy_tax: None,
            sell_tax: None,
            last_tax_change_block: None,
            tax_history: Vec::new(),
            creation_block: self.creation_block.unwrap_or_default(),
            creation_tx: String::new(),
            creation_timestamp: self.creation_timestamp.map(|value| value as f64),
            latest_activity_block: self
                .latest_block
                .or(self.creation_block)
                .unwrap_or_default(),
            is_scam: self.is_scam,
            scam_label: self.scam_label,
            total_liquidity: 0.0,
        })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LivePoolView {
    token_address: String,
    pool_address: String,
    protocol: String,
    denom_address: String,
    denom_symbol: Option<String>,
    currency: String,
    token_reserve: f64,
    denom_reserve: f64,
    can_buy: bool,
    can_sell: bool,
    trading_enabled: bool,
    buy_tax: Option<f64>,
    sell_tax: Option<f64>,
    is_scam: bool,
    scam_label: Option<String>,
    creation_block: Option<u64>,
    can_buy_block: Option<u64>,
    latest_block_number: Option<u64>,
    lp_approved_percentage: f64,
    lp_total_supply: Option<f64>,
    lp_token_address: Option<String>,
    liquidity_positions: Vec<LiveLiquidityPositionView>,
}

impl LivePoolView {
    fn into_cache_pool(self) -> Option<Pool> {
        let token_address = normalize_address(&self.token_address)?;
        let address = normalize_address(&self.pool_address)?;
        let pool_type = parse_pool_type(&self.protocol);
        let live_lp_token_address = normalize_optional_address(self.lp_token_address);
        let lp_token_address =
            liquidity_ownership_token_for_pool(&pool_type, &address, live_lp_token_address.clone());
        let position_manager_address =
            concentrated_position_manager_for_pool(&pool_type, live_lp_token_address);
        let denom_address = normalize_address(&self.denom_address).unwrap_or_default();
        let denom_currency = self
            .denom_symbol
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| default_if_empty(self.currency, "UNKNOWN"));
        let lifecycle = if self.is_scam {
            PoolLifecycle::Scam
        } else if self.trading_enabled || self.can_buy || self.can_sell {
            PoolLifecycle::Active
        } else if self.denom_reserve > 0.0 || self.token_reserve > 0.0 {
            PoolLifecycle::LiquidityDeposited
        } else {
            PoolLifecycle::Discovered
        };

        Some(Pool {
            address,
            token_address,
            pool_type,
            token_reserve: self.token_reserve,
            eth_reserve: self.denom_reserve,
            denom_currency,
            denom_address,
            trading_enabled: self.trading_enabled,
            trading_enabled_block: self.can_buy_block,
            trading_enabled_tx: None,
            fee_tier: None,
            pool_id: None,
            lp_token_address,
            position_manager_address,
            lp_total_supply: self.lp_total_supply,
            liquidity_positions: self
                .liquidity_positions
                .into_iter()
                .filter_map(|position| position.into_cache_position(self.lp_total_supply))
                .collect(),
            last_updated_block: self
                .latest_block_number
                .or(self.can_buy_block)
                .or(self.creation_block)
                .unwrap_or_default(),
            last_updated_time: 0.0,
            is_scam: self.is_scam,
            scam_label: self.scam_label,
            lp_tokens_approved_percentage: Some(self.lp_approved_percentage),
            lifecycle,
            control_addresses: Vec::new(),
            can_buy: self.can_buy,
            can_sell: self.can_sell,
            received_at: Instant::now(),
        })
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LiveLiquidityPositionView {
    position_id: String,
    token_id: Option<String>,
    owner: String,
    position_manager_address: Option<String>,
    liquidity: String,
    position_share_pct: Option<f64>,
    tick_lower: Option<i32>,
    tick_upper: Option<i32>,
    last_update_block: Option<u64>,
    last_update_tx: Option<String>,
}

impl LiveLiquidityPositionView {
    fn into_cache_position(
        self,
        pool_liquidity: Option<f64>,
    ) -> Option<ConcentratedLiquidityPosition> {
        if self.position_id.trim().is_empty() || self.owner.trim().is_empty() {
            return None;
        }
        let position_share_pct = self.position_share_pct.or_else(|| {
            let liquidity = self.liquidity.parse::<f64>().ok()?;
            let pool_liquidity = pool_liquidity?;
            (pool_liquidity > 0.0).then_some((liquidity / pool_liquidity) * 100.0)
        });
        Some(ConcentratedLiquidityPosition {
            position_id: self.position_id,
            token_id: self.token_id,
            owner: normalize_address(&self.owner)?,
            position_manager_address: normalize_optional_address(self.position_manager_address),
            liquidity: self.liquidity,
            position_share_pct,
            tick_lower: self.tick_lower,
            tick_upper: self.tick_upper,
            last_update_block: self.last_update_block,
            last_update_tx: self.last_update_tx,
        })
    }
}

fn parse_pool_type(value: &str) -> PoolType {
    let normalized = value
        .trim()
        .to_ascii_uppercase()
        .replace('_', "-")
        .replace(' ', "-");
    match normalized.as_str() {
        "UNISWAP-V2" | "V2" => PoolType::UniswapV2,
        "UNISWAP-V3" | "V3" => PoolType::UniswapV3,
        "UNISWAP-V4" | "V4" => PoolType::UniswapV4,
        "SUSHISWAP-V2" | "SUSHISWAP" | "SUSHI-V2" => PoolType::SushiSwapV2,
        "SUSHISWAP-V3" | "SUSHI-V3" => PoolType::SushiSwapV3,
        "PANCAKESWAP-V2" | "PANCAKESWAP" | "PANCAKE-V2" => PoolType::PancakeSwapV2,
        "PANCAKESWAP-V3" | "PANCAKE-V3" => PoolType::PancakeSwapV3,
        "SHIBASWAP-V2" | "SHIBASWAP" => PoolType::ShibaSwapV2,
        "FRAXSWAP-V2" | "FRAXSWAP" => PoolType::FraxswapV2,
        "CURVE" | "CURVE-V1" => PoolType::Curve,
        "BALANCER" | "BALANCER-V2" => PoolType::Balancer,
        _ => PoolType::Unknown,
    }
}

fn liquidity_ownership_token_for_pool(
    pool_type: &PoolType,
    pool_identifier: &str,
    live_lp_token_address: Option<String>,
) -> Option<String> {
    match pool_type {
        PoolType::UniswapV2
        | PoolType::SushiSwapV2
        | PoolType::PancakeSwapV2
        | PoolType::ShibaSwapV2
        | PoolType::FraxswapV2 => live_lp_token_address.or_else(|| evm_address(pool_identifier)),
        PoolType::Curve => live_lp_token_address.or_else(|| evm_address(pool_identifier)),
        PoolType::Balancer => live_lp_token_address
            .or_else(|| balancer_bpt_address_from_pool_identifier(pool_identifier))
            .or_else(|| evm_address(pool_identifier)),
        PoolType::UniswapV3
        | PoolType::SushiSwapV3
        | PoolType::PancakeSwapV3
        | PoolType::UniswapV4 => None,
        PoolType::Unknown => live_lp_token_address.or_else(|| evm_address(pool_identifier)),
    }
}

fn concentrated_position_manager_for_pool(
    pool_type: &PoolType,
    live_lp_token_address: Option<String>,
) -> Option<String> {
    match pool_type {
        PoolType::UniswapV3
        | PoolType::SushiSwapV3
        | PoolType::PancakeSwapV3
        | PoolType::UniswapV4 => live_lp_token_address,
        _ => None,
    }
}

fn balancer_bpt_address_from_pool_identifier(pool_identifier: &str) -> Option<String> {
    let (_, pool_id) = pool_identifier.split_once('#')?;
    let pool_id = pool_id.trim().strip_prefix("0x").unwrap_or(pool_id.trim());
    if pool_id.len() < 40
        || !pool_id
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    Some(format!("0x{}", &pool_id[..40]).to_ascii_lowercase())
}

fn evm_address(value: &str) -> Option<String> {
    let value = value.trim();
    let hex = value.strip_prefix("0x").unwrap_or(value);
    if hex.len() == 40 && hex.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        Some(format!("0x{hex}").to_ascii_lowercase())
    } else {
        None
    }
}

fn normalize_optional_address(value: Option<String>) -> Option<String> {
    value.and_then(|value| normalize_address(&value))
}

fn normalize_address(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_ascii_lowercase())
}

fn default_if_empty(value: String, default: &str) -> String {
    if value.trim().is_empty() {
        default.to_string()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{balancer_bpt_address_from_pool_identifier, endpoint, LivePoolView, LiveTokenView};
    use crate::token_tracking::types::{PoolLifecycle, PoolType};

    #[test]
    fn builds_token_server_endpoint() {
        assert_eq!(
            endpoint(
                "http://127.0.0.1:8765/",
                "/api/v1/eth/live-token-tracker/tokens"
            ),
            "http://127.0.0.1:8765/api/v1/eth/live-token-tracker/tokens"
        );
    }

    #[test]
    fn maps_live_token_server_views_to_cache_types() {
        let token = LiveTokenView {
            contract_address: "0xABC".to_string(),
            symbol: "ABC".to_string(),
            name: "Alpha".to_string(),
            decimals: 18,
            total_supply: "1000".to_string(),
            creation_block: Some(10),
            creator_address: Some("0xCREATOR".to_string()),
            ..Default::default()
        }
        .into_cache_token()
        .expect("token mapped");

        assert_eq!(token.address, "0xabc");
        assert_eq!(token.creator_address, "0xcreator");

        let pool = LivePoolView {
            token_address: "0xABC".to_string(),
            pool_address: "0xPOOL".to_string(),
            protocol: "UNISWAP-V2".to_string(),
            currency: "WETH".to_string(),
            denom_address: "0xWETH".to_string(),
            denom_reserve: 1.2,
            can_buy: true,
            ..Default::default()
        }
        .into_cache_pool()
        .expect("pool mapped");

        assert_eq!(pool.pool_type, PoolType::UniswapV2);
        assert_eq!(pool.lifecycle, PoolLifecycle::Active);
    }

    #[test]
    fn maps_multi_protocol_pool_labels_to_cache_types() {
        let cases = [
            ("SUSHISWAP-V2", PoolType::SushiSwapV2),
            ("SUSHISWAP-V3", PoolType::SushiSwapV3),
            ("PANCAKESWAP-V2", PoolType::PancakeSwapV2),
            ("PANCAKESWAP-V3", PoolType::PancakeSwapV3),
            ("SHIBASWAP-V2", PoolType::ShibaSwapV2),
            ("FRAXSWAP-V2", PoolType::FraxswapV2),
            ("CURVE", PoolType::Curve),
            ("BALANCER", PoolType::Balancer),
        ];

        for (protocol, expected) in cases {
            let pool = LivePoolView {
                token_address: "0xABC".to_string(),
                pool_address: "0xPOOL".to_string(),
                protocol: protocol.to_string(),
                currency: "USDC".to_string(),
                denom_address: "0xUSDC".to_string(),
                ..Default::default()
            }
            .into_cache_pool()
            .expect("pool mapped");

            assert_eq!(pool.pool_type, expected);
        }
    }

    #[test]
    fn derives_balancer_bpt_address_from_composite_pool_identifier() {
        let pool_identifier = "0xba12222222228d8ba445958a75a0704d566bf2c8#0x1234567890abcdef1234567890abcdef12345678000200000000000000000001";

        assert_eq!(
            balancer_bpt_address_from_pool_identifier(pool_identifier).as_deref(),
            Some("0x1234567890abcdef1234567890abcdef12345678")
        );
    }
}
