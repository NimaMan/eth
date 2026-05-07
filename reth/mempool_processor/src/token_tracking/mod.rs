// rust/mempool_processor/src/token_tracking/mod.rs
//
// Module for subscribing to pool and token creator updates published by the Python component.

pub mod address_tracking_cache;
pub mod cache;
pub mod in_process;
mod live_data;
pub mod live_server;
mod thresholds;
pub mod token_parameter_extraction;
pub mod types;

// Re-export commonly used types
pub use address_tracking_cache::{AddressRole, AddressTrackingCache};
pub use cache::{CacheStats, TokenTrackingCache, UpdateResult};
pub use in_process::{
    apply_live_token_snapshots_to_cache, hydrate_cache_from_live_reader,
    start_live_token_reader_cache_sync,
};
pub use live_server::{
    hydrate_cache_from_live_token_server, start_live_token_server_cache_sync,
    LiveTokenServerHydrationReport,
};
use serde_json;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
pub use types::CacheConfig;
pub use types::{Address, Pool, PoolType, Token, TokenUpdate, TokenUpdatePayload, TokenWithPools};
use zmq;

use self::live_data::{
    apply_snapshot_map_to_cache, LiveDataSnapshotFetcher, LEGACY_TOKEN_KEY_PREFIX,
};
use self::types::{
    PoolUpdatesMessage, TokenCreatorMessage, TokenCreatorsMessage, TokenUpdatesMessage,
};

// Default ZMQ endpoints
const DEFAULT_ZMQ_PUB_ENDPOINT: &str = "tcp://localhost:5557";
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/0";
const DEFAULT_REDIS_TOKEN_PREFIX: &str = "eth/live/token/snapshot/";

pub struct TokenTrackingSubscriber {
    cache: Arc<TokenTrackingCache>,
    zmq_pub_endpoint: String,
    redis_fetcher: LiveDataSnapshotFetcher,
    legacy_redis_fetcher: Option<LiveDataSnapshotFetcher>,
    skip_initial_redis_warmup: bool,
}

impl TokenTrackingSubscriber {
    pub fn new(eth_threshold: f64) -> Self {
        Self::with_sources(
            eth_threshold,
            DEFAULT_ZMQ_PUB_ENDPOINT,
            DEFAULT_REDIS_URL,
            DEFAULT_REDIS_TOKEN_PREFIX,
        )
    }

    pub fn with_endpoints(eth_threshold: f64, pub_endpoint: &str) -> Self {
        Self::with_sources(
            eth_threshold,
            pub_endpoint,
            DEFAULT_REDIS_URL,
            DEFAULT_REDIS_TOKEN_PREFIX,
        )
    }

    pub fn with_sources(
        eth_threshold: f64,
        pub_endpoint: &str,
        redis_url: &str,
        redis_token_prefix: &str,
    ) -> Self {
        let config = CacheConfig {
            eth_threshold,
            ..Default::default()
        };
        let cache = Arc::new(TokenTrackingCache::new(config));
        let redis_fetcher = LiveDataSnapshotFetcher::new(redis_url, Some(redis_token_prefix))
            .expect("failed to initialize redis snapshot fetcher");
        let legacy_redis_fetcher = if redis_token_prefix == LEGACY_TOKEN_KEY_PREFIX {
            None
        } else {
            Some(
                LiveDataSnapshotFetcher::new(redis_url, Some(LEGACY_TOKEN_KEY_PREFIX))
                    .expect("failed to initialize legacy redis snapshot fetcher"),
            )
        };
        Self {
            cache,
            zmq_pub_endpoint: pub_endpoint.to_string(),
            redis_fetcher,
            legacy_redis_fetcher,
            skip_initial_redis_warmup: false,
        }
    }

    pub fn skip_initial_redis_warmup(&mut self, skip: bool) {
        self.skip_initial_redis_warmup = skip;
    }

    /// Get a clone of the combined cache for use by other components
    pub fn get_cache(&self) -> Arc<TokenTrackingCache> {
        self.cache.clone()
    }

    /// Get a clone of the combined cache for backward compatibility
    pub fn get_pool_cache(&self) -> Arc<TokenTrackingCache> {
        self.cache.clone()
    }

    /// Hydrate initial token state from Redis snapshots.
    ///
    /// ZMQ is only the live invalidation feed. Startup/restart recovery comes
    /// from the Redis token index, with a scan fallback for older publishers.
    async fn hydrate_initial_token_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Hydrating initial token cache from Redis token snapshot index...");

        match self.redis_fetcher.fetch_indexed_addresses().await {
            Ok(addresses) if !addresses.is_empty() => {
                info!(
                    "Found {} token snapshots via Redis index; hydrating cache...",
                    addresses.len()
                );
                self.fetch_and_apply_snapshots(
                    &self.redis_fetcher,
                    &addresses,
                    0,
                    "redis_index",
                    0.0,
                )
                .await;
            }
            Ok(_) => {
                info!("Redis token snapshot index is empty; scanning snapshot keys...");
                self.hydrate_initial_token_state_from_scan().await;
            }
            Err(err) => {
                warn!(
                    "Redis token snapshot index unavailable ({}); scanning snapshot keys...",
                    err
                );
                self.hydrate_initial_token_state_from_scan().await;
            }
        }

        let stats = self.cache.stats().await;
        if stats.total_tokens == 0 {
            self.hydrate_initial_token_state_from_legacy().await;
        }

        let stats = self.cache.stats().await;
        info!(
            "Cache stats after Redis warmup: {} tokens, {} pools, {} creators",
            stats.total_tokens, stats.total_pools, stats.total_creators
        );

        Ok(())
    }

    async fn hydrate_initial_token_state_from_scan(&self) {
        match self.redis_fetcher.fetch_all_addresses().await {
            Ok(addresses) if !addresses.is_empty() => {
                info!(
                    "Found {} token snapshots via Redis scan; hydrating cache...",
                    addresses.len()
                );
                self.fetch_and_apply_snapshots(
                    &self.redis_fetcher,
                    &addresses,
                    0,
                    "redis_scan",
                    0.0,
                )
                .await;
            }
            Ok(_) => {
                warn!("Redis scan returned no token snapshots; cache remains empty");
            }
            Err(err) => {
                warn!("Redis scan failed for token snapshots: {}", err);
            }
        }
    }

    async fn hydrate_initial_token_state_from_legacy(&self) {
        let Some(legacy_fetcher) = &self.legacy_redis_fetcher else {
            return;
        };

        info!("Canonical token snapshots are empty; checking legacy token:snapshot:* keys...");
        match legacy_fetcher.fetch_all_addresses().await {
            Ok(addresses) if !addresses.is_empty() => {
                info!(
                    "Found {} legacy token snapshots; hydrating cache for migration...",
                    addresses.len()
                );
                self.fetch_and_apply_snapshots(
                    legacy_fetcher,
                    &addresses,
                    0,
                    "legacy_redis_scan",
                    0.0,
                )
                .await;
            }
            Ok(_) => {
                warn!("Legacy Redis scan returned no token snapshots");
            }
            Err(err) => {
                warn!("Legacy Redis scan failed for token snapshots: {}", err);
            }
        }
    }

    pub async fn start_listening(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Starting token tracking subscriber");

        let context = zmq::Context::new();
        let subscriber = context.socket(zmq::SUB)?;

        info!(
            "Connecting ZMQ SUB socket to Python publisher at {}",
            self.zmq_pub_endpoint
        );
        subscriber.connect(&self.zmq_pub_endpoint)?;

        // Subscribe to all messages (empty subscription prefix)
        subscriber.set_subscribe(b"")?;
        debug!("Subscribed to all messages from publisher.");

        // Subscribe first, then warm from Redis so restart/startup recovery does
        // not depend on an in-memory Python query socket.
        if self.skip_initial_redis_warmup {
            info!(
                "Skipping Redis token snapshot warmup because live token tracker hydrate succeeded"
            );
        } else if let Err(e) = self.hydrate_initial_token_state().await {
            warn!("Failed to hydrate initial token state: {}", e);
        }

        info!("📊 Real-time token tracking updates active");
        loop {
            match subscriber.recv_string(zmq::DONTWAIT) {
                Ok(Ok(msg_str)) => {
                    debug!("Received update message from Python publisher");

                    // Try to parse as new token-centric message format first
                    if let Ok(token_message) = serde_json::from_str::<TokenUpdatesMessage>(&msg_str)
                    {
                        self.handle_token_message(token_message).await;
                    } else if let Ok(pool_message) =
                        serde_json::from_str::<PoolUpdatesMessage>(&msg_str)
                    {
                        // Handle legacy pool updates (backward compatibility)
                        debug!(
                            "Received legacy pool update: {} pools",
                            pool_message.data.len()
                        );

                        // Note: Legacy pool updates don't contain full token info
                        // For now, we'll skip them as the new cache requires full token data
                        warn!("Legacy pool updates not supported with new cache. Skipping.");
                    } else if serde_json::from_str::<TokenCreatorMessage>(&msg_str).is_ok() {
                        // Handle single creator update
                        debug!("Received token creator update");

                        // Note: Creator-only updates don't contain full token info
                        // For now, we'll skip them as the new cache requires full token data
                        warn!("Creator-only updates not supported with new cache. Skipping.");
                    } else if let Ok(creators_message) =
                        serde_json::from_str::<TokenCreatorsMessage>(&msg_str)
                    {
                        // Handle bulk creator updates
                        debug!(
                            "Received bulk token creators update: {} creators",
                            creators_message.data.len()
                        );

                        // Note: Creator-only updates don't contain full token info
                        // For now, we'll skip them as the new cache requires full token data
                        warn!("Creator-only updates not supported with new cache. Skipping.");
                    } else {
                        warn!("Failed to parse update message as any known type");
                        debug!("Raw message: {}", msg_str);
                    }
                }
                Ok(Err(zmq_err)) => {
                    // ZMQ string conversion error
                    error!("Error converting ZMQ message to string: {:?}", zmq_err);
                }
                Err(zmq::Error::EAGAIN) => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => {
                    // ZMQ recv error
                    error!("Error receiving ZMQ message: {:?}", e);
                    // Consider adding a small delay or break/reconnect logic here
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
            // Yield to allow other tasks to run, especially if in a tight loop without actual blocking IO
            tokio::task::yield_now().await;
        }
        // Unreachable in the current loop, but good practice for future changes
        // Ok(())
    }

    async fn handle_token_message(&self, token_message: TokenUpdatesMessage) {
        let message_type = token_message.message_type.clone();
        let block_number = token_message.block_number;
        let timestamp = token_message.timestamp;

        match message_type.as_str() {
            "full_update" => {
                info!(
                    "Received full update with {} tokens",
                    token_message.token_count
                );
            }
            "block_update" => {
                debug!(
                    "Received block update with {} tokens",
                    token_message.token_count
                );
            }
            other => {
                warn!("Unknown token message type: {}", other);
            }
        }

        match token_message.into_snapshot_map() {
            TokenUpdatePayload::TokenMap(map) => {
                apply_snapshot_map_to_cache(
                    &self.cache,
                    block_number,
                    timestamp,
                    &message_type,
                    map,
                )
                .await;
            }
            TokenUpdatePayload::Addresses(addresses) => {
                self.fetch_and_apply_snapshots(
                    &self.redis_fetcher,
                    &addresses,
                    block_number,
                    &message_type,
                    timestamp,
                )
                .await;
            }
            TokenUpdatePayload::Empty => {
                debug!("Token update payload empty; nothing to hydrate");
            }
        }
    }

    async fn fetch_and_apply_snapshots(
        &self,
        fetcher: &LiveDataSnapshotFetcher,
        addresses: &[String],
        block_number: u64,
        reason: &str,
        timestamp: f64,
    ) {
        const BATCH_SIZE: usize = 64;
        for chunk in addresses.chunks(BATCH_SIZE) {
            match fetcher.fetch_token_with_pools(chunk).await {
                Ok(map) => {
                    if map.is_empty() {
                        continue;
                    }
                    apply_snapshot_map_to_cache(&self.cache, block_number, timestamp, reason, map)
                        .await;
                }
                Err(err) => {
                    warn!(
                        "Failed to fetch {} token snapshots from redis: {}",
                        chunk.len(),
                        err
                    );
                }
            }
        }
    }
}

// Basic test function to ensure the module structure is sound
#[cfg(test)]
mod basic_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn can_create_subscriber() {
        let subscriber = TokenTrackingSubscriber::new(0.05);
        let cache = subscriber.get_cache();
        // Cache exists and can be accessed
        assert!(Arc::strong_count(&cache) > 0);
    }

    #[test]
    fn can_create_subscriber_with_custom_endpoints() {
        let pub_endpoint = "tcp://127.0.0.1:5557";
        let subscriber = TokenTrackingSubscriber::with_endpoints(0.1, pub_endpoint);
        assert_eq!(subscriber.zmq_pub_endpoint, pub_endpoint);
    }

    #[tokio::test]
    async fn handle_token_message_updates_cache() {
        let subscriber = TokenTrackingSubscriber::with_sources(
            0.1,
            "tcp://127.0.0.1:6007",
            "redis://127.0.0.1:6379/0",
            DEFAULT_REDIS_TOKEN_PREFIX,
        );

        let token_address = "0x0000000000000000000000000000000000000001";
        let pool_address = "0x0000000000000000000000000000000000000002";
        let creator_address = "0x0000000000000000000000000000000000000003";

        let pool_entry = json!({
            "pool_address": pool_address,
            "token_address": token_address,
            "pool_type": "UNISWAP-V2",
            "token_reserve": 1.0,
            "denom_reserve": 1.5,
            "denom_currency": "ETH",
            "denom_address": "0x000000000000000000000000000000000000000E",
            "trading_enabled": true,
            "trading_enabled_block": 150u64,
            "latest_block_number": 150u64,
            "last_update_time": 1690000500.0,
            "is_scam": false,
            "control_addresses": [],
            "lp_tokens_approved_percentage": null,
            "can_buy": true,
            "can_sell": true
        });

        let mut pools_map = serde_json::Map::new();
        pools_map.insert(pool_address.to_string(), pool_entry);

        let mut token_entry = json!({
            "token_address": token_address,
            "token_symbol": "TKN",
            "token_name": "Test Token",
            "token_decimals": 18,
            "total_supply": "1000000000000000000",
            "creator_address": creator_address,
            "current_owner": creator_address,
            "tax_setter_addresses": [],
            "ownership_renounced": false,
            "renouncement_block": null,
            "current_buy_tax": null,
            "current_sell_tax": null,
            "creation_block": 100u64,
            "creation_tx": "0xcreationtx",
            "creation_timestamp": 1690000000.0,
            "latest_activity_block": 150u64,
            "is_scam": false,
            "scam_label": null,
            "pools": {}
        });

        if let serde_json::Value::Object(ref mut obj) = token_entry {
            obj.insert("pools".to_string(), serde_json::Value::Object(pools_map));
        }

        let mut root_map = serde_json::Map::new();
        root_map.insert(token_address.to_string(), token_entry);
        let token_payload = serde_json::Value::Object(root_map);

        let message = TokenUpdatesMessage {
            message_type: "block_update".to_string(),
            token_count: 1,
            block_number: 150,
            timestamp: 1690000500.0,
            data: token_payload,
        };

        subscriber.handle_token_message(message).await;

        let cache = subscriber.get_cache();
        let stats = cache.stats().await;
        assert_eq!(stats.total_tokens, 1);
        assert_eq!(stats.total_pools, 1);

        let creator = creator_address.to_string();
        assert!(cache.is_creator(&creator).await);

        let token = cache
            .get_token(&token_address.to_string())
            .await
            .expect("token cached");
        assert_eq!(token.address, token_address);
        assert!(cache.get_pool(&pool_address.to_string()).await.is_some());
    }
}
