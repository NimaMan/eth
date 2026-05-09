/// Verify Redis Loading & Cache Integrity
///
/// This example connects directly to Redis, scans for all token snapshots,
/// parses them (mimicking the internal logic), loads them into the TokenTrackingCache,
/// and verifies the data integrity.
///
/// It effectively replaces the need for the Python publisher during verification/testing.
///
/// Usage: cargo run --example export_all_tokens_from_cache_to_csv
use mempool_processor::config::DEFAULT_REDIS_TOKEN_PREFIX;
use mempool_processor::token_tracking::types::PoolLifecycle;
use mempool_processor::token_tracking::{
    CacheConfig, Pool, PoolType, Token, TokenTrackingCache, TokenUpdate, TokenWithPools,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("📊 Token Cache Verification & Redis Loader");
    info!("========================================");

    // 1. Setup Redis connection
    let redis_url = std::env::var("TOKEN_SNAPSHOT_REDIS_URL")
        .unwrap_or_else(|_| mempool_processor::config::live_data_redis_url_from_env());
    let redis_prefix = std::env::var("TOKEN_SNAPSHOT_REDIS_PREFIX")
        .unwrap_or_else(|_| DEFAULT_REDIS_TOKEN_PREFIX.to_string());

    info!("Connecting to Redis at {}...", redis_url);
    let client = redis::Client::open(redis_url.clone())?;
    let mut conn = client.get_multiplexed_tokio_connection().await?;

    // 2. Scan for keys
    info!("Scanning Redis for keys matching '{}*'...", redis_prefix);
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("{}*", redis_prefix))
        .query_async(&mut conn)
        .await?;

    info!("Found {} token keys in Redis.", keys.len());

    if keys.is_empty() {
        warn!("No tokens found! Aborting verification.");
        return Ok(());
    }

    // 3. Fetch values in batches to avoid blocking Redis
    let batch_size = 1000;
    let mut loaded_tokens = HashMap::new();
    let mut failed_parse = 0;

    info!("Fetching and parsing snapshots...");
    let start_time = Instant::now();

    for chunk in keys.chunks(batch_size) {
        let values: Vec<Option<String>> =
            redis::cmd("MGET").arg(chunk).query_async(&mut conn).await?;

        for (key, val_opt) in chunk.iter().zip(values.into_iter()) {
            if let Some(val) = val_opt {
                match serde_json::from_str::<RedisTokenSnapshot>(&val) {
                    Ok(snapshot) => {
                        // Extract address from key if missing in snapshot (fallback)
                        let address_from_key = key.replace(&redis_prefix, "");

                        match snapshot.into_token_with_pools(&address_from_key) {
                            Ok(token_with_pools) => {
                                loaded_tokens.insert(
                                    token_with_pools.token.address.clone(),
                                    token_with_pools,
                                );
                            }
                            Err(_e) => {
                                // warn!("Failed to convert snapshot for {}: {}", address_from_key, e);
                                failed_parse += 1;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("JSON parse error for {}: {}", key, e);
                        failed_parse += 1;
                    }
                }
            }
        }
    }

    let fetch_duration = start_time.elapsed();
    info!(
        "Fetched & parsed {} tokens in {:.2?} (Failed: {})",
        loaded_tokens.len(),
        fetch_duration,
        failed_parse
    );

    // 4. Initialize Cache
    info!("Initializing TokenTrackingCache...");
    let config = CacheConfig {
        max_tokens: 100_000, // Ensure big enough for all
        max_pools: 200_000,
        eth_threshold: 0.0, // Load everything for verification
        evict_scam_first: true,
    };
    let cache = Arc::new(TokenTrackingCache::new(config));

    // 5. Populate Cache
    info!("Populating cache with loaded tokens...");

    // Split into batches for cache update
    let all_tokens: Vec<TokenWithPools> = loaded_tokens.values().cloned().collect();
    let update_batch_size = 5000;

    for chunk in all_tokens.chunks(update_batch_size) {
        let mut data_map = HashMap::new();
        for t in chunk {
            data_map.insert(t.token.address.clone(), t.clone());
        }

        let update = TokenUpdate {
            message_type: "redis_verification".to_string(),
            token_count: data_map.len(),
            block_number: 0, // Unknown from just list
            timestamp: 0.0,
            data: data_map,
        };

        cache.batch_update(update).await;
    }

    // 6. Verify Cache Stats
    let stats = cache.stats().await;
    info!("✅ Cache Populated Successfully!");
    info!("   Total Tokens: {}", stats.total_tokens);
    info!("   Total Pools:  {}", stats.total_pools);
    info!("   Total Creators: {}", stats.total_creators);

    // 7. Sanity Check (from check_if_creator.rs)
    let test_creator = "0xcb1534B135450ac5433BE4c5f5eC61a781b05c35";
    info!("\n=== Sanity Check: {} ===", test_creator);
    if cache.is_creator(&test_creator.to_string()).await {
        info!("✓ Address IS correctly identified as a creator");
        let tokens = cache.get_tokens_by_creator(&test_creator.to_string()).await;
        info!("  Created {} tokens:", tokens.len());
        for t in tokens {
            info!("  - {} ({})", t.symbol, t.address);
        }
    } else {
        warn!("✗ Address was NOT found as a creator (might be expected if not in Redis data)");
    }

    Ok(())
}

// ==================================================================================
// Private structs copy-pasted from live_data.rs to support parsing
// ==================================================================================

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
    fn into_token_with_pools(self, fallback_address: &str) -> Result<TokenWithPools, String> {
        let address = if self.contract_address.is_empty() {
            fallback_address.to_string()
        } else {
            self.contract_address.clone()
        };

        if address.is_empty() {
            return Err("Missing address".to_string());
        }

        let total_liquidity = self.pools.total_liquidity_by_denom.values().sum();

        let token = Token {
            address: address.clone(),
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
                &address,
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
