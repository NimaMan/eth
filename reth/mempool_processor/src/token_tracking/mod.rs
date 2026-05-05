// rust/mempool_processor/src/token_tracking/mod.rs
//
// Module for subscribing to pool and token creator updates published by the Python component.

pub mod address_tracking_cache;
pub mod cache;
mod live_data;
mod thresholds;
pub mod token_parameter_extraction;
pub mod types;

// Re-export commonly used types
pub use address_tracking_cache::{AddressRole, AddressTrackingCache};
pub use cache::{CacheStats, TokenTrackingCache, UpdateResult};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
pub use types::CacheConfig;
pub use types::{Address, Pool, PoolType, Token, TokenUpdate, TokenUpdatePayload, TokenWithPools};
use zmq;

use self::live_data::{apply_snapshot_map_to_cache, LiveDataSnapshotFetcher};
use self::types::{
    PoolUpdatesMessage, TokenCreatorMessage, TokenCreatorsMessage, TokenQueryResponse,
    TokenUpdatesMessage,
};

// Default ZMQ endpoints
const DEFAULT_ZMQ_PUB_ENDPOINT: &str = "tcp://localhost:5557";
const DEFAULT_ZMQ_REP_ENDPOINT: &str = "tcp://localhost:5558";
const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379/0";
const DEFAULT_REDIS_TOKEN_PREFIX: &str = "token:snapshot:";

pub struct TokenTrackingSubscriber {
    cache: Arc<TokenTrackingCache>,
    zmq_pub_endpoint: String,
    zmq_rep_endpoint: String,
    redis_fetcher: LiveDataSnapshotFetcher,
}

impl TokenTrackingSubscriber {
    pub fn new(eth_threshold: f64) -> Self {
        Self::with_sources(
            eth_threshold,
            DEFAULT_ZMQ_PUB_ENDPOINT,
            DEFAULT_ZMQ_REP_ENDPOINT,
            DEFAULT_REDIS_URL,
            DEFAULT_REDIS_TOKEN_PREFIX,
        )
    }

    pub fn with_endpoints(eth_threshold: f64, pub_endpoint: &str, rep_endpoint: &str) -> Self {
        Self::with_sources(
            eth_threshold,
            pub_endpoint,
            rep_endpoint,
            DEFAULT_REDIS_URL,
            DEFAULT_REDIS_TOKEN_PREFIX,
        )
    }

    pub fn with_sources(
        eth_threshold: f64,
        pub_endpoint: &str,
        rep_endpoint: &str,
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
        Self {
            cache,
            zmq_pub_endpoint: pub_endpoint.to_string(),
            zmq_rep_endpoint: rep_endpoint.to_string(),
            redis_fetcher,
        }
    }

    /// Get a clone of the combined cache for use by other components
    pub fn get_cache(&self) -> Arc<TokenTrackingCache> {
        self.cache.clone()
    }

    /// Get a clone of the combined cache for backward compatibility
    pub fn get_pool_cache(&self) -> Arc<TokenTrackingCache> {
        self.cache.clone()
    }

    /// Request initial pool state from Python service via REQ/REP socket
    async fn request_initial_pool_state(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Requesting pool data from Python service...");

        // Create REQ socket to request pool data
        let context = zmq::Context::new();
        let requester = context.socket(zmq::REQ)?;
        requester.set_rcvtimeo(2_000)?;
        requester.set_sndtimeo(2_000)?;

        // Connect to REP endpoint
        requester.connect(&self.zmq_rep_endpoint)?;
        info!(
            "Connected to Python REP socket at {}",
            self.zmq_rep_endpoint
        );

        // Create request for all tokens
        let request = serde_json::json!({
            "type": "get_all_tokens"
        });

        // Send request
        requester.send(&request.to_string(), 0)?;
        debug!("Sent get_all_tokens request");

        // Receive response (with timeout)
        let response_str = match requester.recv_string(0) {
            Ok(Ok(s)) => Some(s),
            Ok(Err(e)) => {
                warn!(
                    "ZMQ string conversion error: {:?} (continuing with Redis fallback)",
                    e
                );
                None
            }
            Err(e) => {
                warn!(
                    "ZMQ recv error: {:?} (likely publisher offline); continuing with Redis fallback",
                    e
                );
                None
            }
        };
        if let Some(response_str) = response_str {
            debug!("Received response from Python service");

            // Parse response
            let response: TokenQueryResponse = serde_json::from_str(&response_str)?;

            if response.status != "success" {
                warn!(
                    "Failed to get token data: {}",
                    response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            } else {
                let token_count = response.count.unwrap_or(0);
                info!(
                    "Received {} token addresses from Python service",
                    token_count
                );

                if let Some(data_value) = response.data {
                    if let Some(addresses) = data_value.as_array() {
                        let addr_list: Vec<String> = addresses
                            .iter()
                            .filter_map(|val| val.as_str().map(|s| s.to_string()))
                            .collect();
                        self.fetch_and_apply_snapshots(&addr_list, 0, "initial_load", 0.0)
                            .await;
                    } else if data_value.is_object() {
                        match serde_json::from_value::<HashMap<Address, TokenWithPools>>(data_value)
                        {
                            Ok(token_data) => {
                                let update = types::TokenUpdate {
                                    message_type: "initial_load".to_string(),
                                    token_count,
                                    block_number: 0,
                                    timestamp: 0.0,
                                    data: token_data,
                                };

                                let result = self.cache.batch_update(update).await;
                                info!(
                                    "✅ Initialized cache with {} tokens, {} pools, {} creators",
                                    result.tokens_updated,
                                    result.pools_updated,
                                    result.creators_added
                                );
                            }
                            Err(err) => {
                                error!("Failed to parse legacy token snapshot payload: {}", err);
                            }
                        }
                    } else {
                        warn!("Initial response payload not understood; skipping cache warmup");
                    }
                }
            }
        } else {
            warn!("No ZMQ response received; skipping ZMQ warmup and falling back to Redis");
        }

        let stats = self.cache.stats().await;
        info!(
            "Cache stats after initialization: {} tokens, {} pools, {} creators",
            stats.total_tokens, stats.total_pools, stats.total_creators
        );

        // Fallback: if ZMQ path didn't hydrate anything, scan Redis directly.
        if stats.total_tokens == 0 {
            info!("Token cache empty after ZMQ init; scanning Redis for snapshots...");
            match self.redis_fetcher.fetch_all_addresses().await {
                Ok(addresses) if !addresses.is_empty() => {
                    info!(
                        "Found {} token snapshots via Redis scan; hydrating cache...",
                        addresses.len()
                    );
                    self.fetch_and_apply_snapshots(&addresses, 0, "redis_scan", 0.0)
                        .await;
                    let stats = self.cache.stats().await;
                    info!(
                        "Cache stats after Redis scan: {} tokens, {} pools, {} creators",
                        stats.total_tokens, stats.total_pools, stats.total_creators
                    );
                }
                Ok(_) => {
                    warn!("Redis scan returned no token snapshots; cache remains empty");
                }
                Err(err) => {
                    warn!("Redis scan failed for token snapshots: {}", err);
                }
            }
        }

        Ok(())
    }

    /// Request initial token creator data from Python service
    /// NOTE: This is now handled within request_initial_pool_data since creators are embedded in token data
    async fn request_initial_creator_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Creator data is now loaded as part of token data in request_initial_pool_data
        debug!("Creator data already loaded from token data");
        Ok(())
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

        // Request initial data from Python
        if let Err(e) = self.request_initial_pool_state().await {
            warn!("Failed to get initial pool state: {}", e);
        }

        if let Err(e) = self.request_initial_creator_data().await {
            warn!("Failed to get initial creator data: {}", e);
        }

        info!("📊 Real-time token tracking updates active");
        loop {
            match subscriber.recv_string(0) {
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
                self.fetch_and_apply_snapshots(&addresses, block_number, &message_type, timestamp)
                    .await;
            }
            TokenUpdatePayload::Empty => {
                debug!("Token update payload empty; nothing to hydrate");
            }
        }
    }

    async fn fetch_and_apply_snapshots(
        &self,
        addresses: &[String],
        block_number: u64,
        reason: &str,
        timestamp: f64,
    ) {
        const BATCH_SIZE: usize = 64;
        for chunk in addresses.chunks(BATCH_SIZE) {
            match self.redis_fetcher.fetch_token_with_pools(chunk).await {
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
        let rep_endpoint = "tcp://127.0.0.1:5558";
        let subscriber = TokenTrackingSubscriber::with_endpoints(0.1, pub_endpoint, rep_endpoint);
        assert_eq!(subscriber.zmq_pub_endpoint, pub_endpoint);
        assert_eq!(subscriber.zmq_rep_endpoint, rep_endpoint);
    }

    #[tokio::test]
    async fn handle_token_message_updates_cache() {
        let subscriber = TokenTrackingSubscriber::with_sources(
            0.1,
            "tcp://127.0.0.1:6007",
            "tcp://127.0.0.1:6008",
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
