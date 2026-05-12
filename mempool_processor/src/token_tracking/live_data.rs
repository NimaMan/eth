use crate::token_tracking::cache::{CacheApplyOutcome, CacheUpdateContext, TokenTrackingCache};
use crate::token_tracking::types::{
    Address, Pool, PoolLifecycle, PoolType, Token, TokenUpdate, TokenWithPools,
};
use eyre::{eyre, Result};
use redis::{AsyncCommands, AsyncIter};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, warn};

pub(crate) const DEFAULT_TOKEN_KEY_PREFIX: &str = "eth/live/token/snapshot/";
pub(crate) const LEGACY_TOKEN_KEY_PREFIX: &str = "token:snapshot:";

#[derive(Clone)]
pub struct LiveDataSnapshotFetcher {
    client: redis::Client,
    key_prefix: String,
    index_key: String,
}

impl LiveDataSnapshotFetcher {
    pub fn new(redis_url: &str, key_prefix: Option<&str>) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|err| eyre!("failed to parse redis url {}: {}", redis_url, err))?;
        let prefix = key_prefix
            .map(|s| s.to_string())
            .unwrap_or_else(|| DEFAULT_TOKEN_KEY_PREFIX.to_string());
        let index_key = token_index_key_for_prefix(&prefix);
        Ok(Self {
            client,
            key_prefix: prefix,
            index_key,
        })
    }

    pub async fn fetch_token_with_pools(
        &self,
        addresses: &[String],
    ) -> Result<HashMap<Address, TokenWithPools>> {
        if addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))?;

        let keys: Vec<String> = addresses
            .iter()
            .map(|addr| format!("{}{}", self.key_prefix, addr))
            .collect();

        let raw_snapshots: Vec<Option<String>> = redis::cmd("MGET")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .map_err(|err| eyre!("failed to fetch token snapshots via MGET: {}", err))?;

        let mut results = HashMap::new();
        for (address, maybe_payload) in addresses.iter().zip(raw_snapshots.into_iter()) {
            let payload = match maybe_payload {
                Some(p) => p,
                None => {
                    debug!(
                        "Token snapshot missing in redis for address {}; skipping",
                        address
                    );
                    continue;
                }
            };

            match serde_json::from_str::<RedisTokenSnapshot>(&payload) {
                Ok(snapshot) => match snapshot.into_token_with_pools() {
                    Ok(token_with_pools) => {
                        results.insert(address.clone(), token_with_pools);
                    }
                    Err(err) => {
                        warn!("Failed to convert redis snapshot for {}: {}", address, err);
                    }
                },
                Err(err) => {
                    warn!("Failed to parse redis snapshot for {}: {}", address, err);
                }
            }
        }

        Ok(results)
    }

    /// Return all token addresses from the explicit Redis token snapshot index.
    pub async fn fetch_indexed_addresses(&self) -> Result<Vec<String>> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))?;

        let mut addresses: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&self.index_key)
            .query_async(&mut conn)
            .await
            .map_err(|err| eyre!("failed to fetch token snapshot index: {}", err))?;
        addresses.sort();
        addresses.dedup();
        Ok(addresses)
    }

    /// Fallback: scan Redis for all token snapshot keys and return their addresses.
    pub async fn fetch_all_addresses(&self) -> Result<Vec<String>> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|err| eyre!("failed to connect to redis: {}", err))?;

        let pattern = format!("{}*", self.key_prefix);
        let iter: AsyncIter<String> = conn
            .scan_match(pattern)
            .await
            .map_err(|err| eyre!("failed to scan redis for token snapshots: {}", err))?;

        let mut addresses = Vec::new();
        tokio::pin!(iter);
        while let Some(key) = iter.next_item().await {
            let key =
                key.map_err(|err| eyre!("failed to scan redis token snapshot key: {}", err))?;
            if let Some(stripped) = key.strip_prefix(&self.key_prefix) {
                if stripped != "index" {
                    addresses.push(stripped.to_string());
                }
            }
        }

        Ok(addresses)
    }
}

fn token_index_key_for_prefix(prefix: &str) -> String {
    if let Some(stripped) = prefix.strip_suffix('/') {
        format!("{stripped}/index")
    } else {
        format!("{prefix}index")
    }
}

pub async fn apply_snapshot_map_to_cache(
    cache: &TokenTrackingCache,
    block_number: u64,
    timestamp: f64,
    message_type: &str,
    token_map: HashMap<Address, TokenWithPools>,
) -> CacheApplyOutcome {
    apply_snapshot_map_to_cache_with_context(
        cache,
        block_number,
        timestamp,
        message_type,
        None,
        false,
        token_map,
    )
    .await
}

pub async fn apply_snapshot_map_to_cache_with_context(
    cache: &TokenTrackingCache,
    block_number: u64,
    timestamp: f64,
    message_type: &str,
    status: Option<String>,
    require_live_status: bool,
    token_map: HashMap<Address, TokenWithPools>,
) -> CacheApplyOutcome {
    if token_map.is_empty() {
        return CacheApplyOutcome::SkippedEmpty;
    }

    let update = TokenUpdate {
        message_type: message_type.to_string(),
        token_count: token_map.len(),
        block_number,
        timestamp,
        data: token_map,
    };

    let outcome = cache
        .batch_update_with_context(
            update,
            CacheUpdateContext {
                source: message_type.to_string(),
                status,
                require_live_status,
            },
        )
        .await;
    match &outcome {
        CacheApplyOutcome::Applied(result) => {
            debug!(
                "Applied snapshot batch @block {} ({} tokens, {} pools updated)",
                block_number, result.tokens_updated, result.pools_updated
            );
        }
        CacheApplyOutcome::Rejected(rejection) => {
            warn!(
                "Rejected token cache snapshot source={} status={:?} block={} current_block={} reason={:?}",
                rejection.source,
                rejection.status,
                rejection.block_number,
                rejection.current_block,
                rejection.reason
            );
        }
        CacheApplyOutcome::SkippedEmpty => {}
    }
    outcome
}

#[derive(Debug, Deserialize)]
struct RedisTokenSnapshot {
    #[serde(default)]
    contract_address: String,
    #[serde(default)]
    metadata: SnapshotMetadata,
    #[serde(default)]
    creation: SnapshotCreation,
    #[serde(default)]
    latest_block: SnapshotBlockMeta,
    #[serde(default)]
    status: SnapshotStatus,
    #[serde(default)]
    control: SnapshotControl,
    #[serde(default)]
    pools: SnapshotPools,
}

impl RedisTokenSnapshot {
    fn into_token_with_pools(self) -> Result<TokenWithPools> {
        if self.contract_address.is_empty() {
            return Err(eyre!("snapshot missing contract address"));
        }

        let total_liquidity = self.pools.total_liquidity_by_denom.values().sum();

        let token = Token {
            address: self.contract_address.clone(),
            symbol: self
                .metadata
                .symbol
                .unwrap_or_else(|| "UNKNOWN".to_string()),
            name: self.metadata.name.unwrap_or_else(|| "Unknown".to_string()),
            decimals: self.metadata.decimals.unwrap_or(18),
            total_supply: value_to_string(self.metadata.total_supply),
            creator_address: self.creation.creator.unwrap_or_default(),
            current_owner: self.control.current_owner.unwrap_or_default(),
            tax_setter_addresses: self.control.tax_setter_addresses.unwrap_or_default(),
            ownership_renounced: self.status.ownership_renounced.unwrap_or(false),
            renouncement_block: self.control.ownership_renounced_block,
            buy_tax: None,
            sell_tax: None,
            last_tax_change_block: None,
            tax_history: Vec::new(),
            creation_block: self.creation.block.unwrap_or_default(),
            creation_tx: self.creation.tx.unwrap_or_default(),
            creation_timestamp: self.creation.timestamp,
            latest_activity_block: self
                .status
                .latest_activity_block
                .or(self.latest_block.number)
                .unwrap_or_default(),
            is_scam: self.status.is_scam.unwrap_or(false),
            scam_label: self.status.scam_label,
            total_liquidity,
        };

        let mut pools = HashMap::new();
        for (pool_address, pool_snapshot) in self.pools.info.into_iter() {
            if let Some(pool) = SnapshotPoolInfo::into_pool(
                pool_snapshot,
                &self.contract_address,
                &pool_address,
                &self.latest_block,
            ) {
                pools.insert(pool_address, pool);
            }
        }

        Ok(TokenWithPools { token, pools })
    }
}

#[derive(Debug, Deserialize, Default)]
struct SnapshotMetadata {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    decimals: Option<u8>,
    #[serde(default)]
    total_supply: Option<Value>,
}

#[derive(Debug, Deserialize, Default)]
struct SnapshotCreation {
    #[serde(default)]
    block: Option<u64>,
    #[serde(default)]
    timestamp: Option<f64>,
    #[serde(default)]
    tx: Option<String>,
    #[serde(default)]
    creator: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SnapshotBlockMeta {
    #[serde(default)]
    number: Option<u64>,
    #[serde(default)]
    timestamp: Option<f64>,
}

#[derive(Debug, Deserialize, Default)]
struct SnapshotStatus {
    #[serde(default)]
    #[allow(dead_code)]
    lifecycle: Option<String>,
    #[serde(default)]
    is_scam: Option<bool>,
    #[serde(default)]
    ownership_renounced: Option<bool>,
    #[serde(default)]
    latest_activity_block: Option<u64>,
    #[serde(default)]
    scam_label: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct SnapshotControl {
    #[serde(default)]
    current_owner: Option<String>,
    #[serde(default)]
    ownership_renounced_block: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    control_addresses: Option<Vec<String>>,
    #[serde(default)]
    tax_setter_addresses: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
struct SnapshotPools {
    #[serde(default)]
    #[allow(dead_code)]
    addresses: Vec<String>,
    #[serde(default)]
    info: HashMap<String, SnapshotPoolInfo>,
    #[serde(default)]
    total_liquidity_by_denom: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Default)]
struct SnapshotPoolInfo {
    #[serde(default)]
    pool_type: Option<String>,
    #[serde(default)]
    denom_address: Option<String>,
    #[serde(default)]
    denom_currency: Option<String>,
    #[serde(default)]
    token_reserve: Option<f64>,
    #[serde(default)]
    denom_reserve: Option<f64>,
    #[serde(default)]
    trading_enabled: Option<bool>,
    #[serde(default)]
    trading_enabled_block: Option<u64>,
    #[serde(default)]
    lifecycle: Option<String>,
    #[serde(default)]
    fee: Option<u32>,
    #[serde(default)]
    pool_id: Option<String>,
    #[serde(default)]
    control_addresses: Option<Vec<String>>,
    #[serde(default)]
    can_buy: Option<bool>,
    #[serde(default)]
    can_sell: Option<bool>,
    #[serde(default)]
    lp_tokens_approved_percentage: Option<f64>,
    #[serde(default)]
    scam_label: Option<String>,
    #[serde(default)]
    is_scam: Option<bool>,
}

impl SnapshotPoolInfo {
    fn into_pool(
        self,
        token_address: &str,
        pool_address: &str,
        latest_block: &SnapshotBlockMeta,
    ) -> Option<Pool> {
        if pool_address.is_empty() {
            return None;
        }

        Some(Pool {
            address: pool_address.to_string(),
            token_address: token_address.to_string(),
            pool_type: parse_pool_type(self.pool_type.as_deref()),
            token_reserve: self.token_reserve.unwrap_or(0.0),
            eth_reserve: self.denom_reserve.unwrap_or(0.0),
            denom_currency: self.denom_currency.unwrap_or_else(|| "UNKNOWN".to_string()),
            denom_address: self
                .denom_address
                .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string()),
            trading_enabled: self.trading_enabled.unwrap_or(false),
            trading_enabled_block: self.trading_enabled_block,
            trading_enabled_tx: None,
            fee_tier: self.fee,
            pool_id: self.pool_id,
            last_updated_block: latest_block.number.unwrap_or_default(),
            last_updated_time: latest_block.timestamp.unwrap_or_default(),
            is_scam: self.is_scam.unwrap_or_else(|| self.scam_label.is_some()),
            scam_label: self.scam_label,
            lp_tokens_approved_percentage: self.lp_tokens_approved_percentage,
            lifecycle: parse_pool_lifecycle(self.lifecycle.as_deref()),
            control_addresses: self.control_addresses.unwrap_or_default(),
            can_buy: self.can_buy.unwrap_or(true),
            can_sell: self.can_sell.unwrap_or(true),
            received_at: Instant::now(),
        })
    }
}

fn parse_pool_type(value: Option<&str>) -> PoolType {
    match value.unwrap_or("").to_uppercase().as_str() {
        "UNISWAP-V2" | "V2" => PoolType::UniswapV2,
        "UNISWAP-V3" | "V3" => PoolType::UniswapV3,
        "UNISWAP-V4" | "V4" => PoolType::UniswapV4,
        _ => PoolType::Unknown,
    }
}

fn parse_pool_lifecycle(value: Option<&str>) -> PoolLifecycle {
    match value.unwrap_or("").to_uppercase().as_str() {
        "SEEDED" | "LIQUIDITY_DEPOSITED" => PoolLifecycle::LiquidityDeposited,
        "ACTIVE" => PoolLifecycle::Active,
        "SCAM" => PoolLifecycle::Scam,
        "EVICTED" => PoolLifecycle::Evicted,
        "DISCOVERED" => PoolLifecycle::Discovered,
        _ => PoolLifecycle::Unknown,
    }
}

fn value_to_string(value: Option<Value>) -> Option<String> {
    value.and_then(|v| match v {
        Value::String(s) => Some(s),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some((b as u8).to_string()),
        _ => None,
    })
}
