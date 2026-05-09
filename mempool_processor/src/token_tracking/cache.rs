// cache_v2.rs - High-performance token tracking cache with zero-copy access
//
// Key improvements:
// 1. LRU eviction for bounded memory
// 2. Arc-wrapped data for zero-copy reads
// 3. Indexed lookups for O(1) access
// 4. Batch updates with single lock acquisition

use chrono::Utc;
use lru::LruCache;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::thresholds::threshold_for_symbol;
pub use super::types::{
    Address, CacheConfig, Pool, PoolLifecycle, Token, TokenUpdate, TokenWithPools,
};

/// High-performance token tracking cache with bounded memory
#[derive(Clone)]
pub struct TokenTrackingCache {
    // Primary storage - bounded with LRU eviction
    tokens: Arc<RwLock<LruCache<Address, Arc<Token>>>>,
    pools: Arc<RwLock<LruCache<Address, Arc<Pool>>>>,

    // Indexed lookups for O(1) access
    creator_to_tokens: Arc<RwLock<HashMap<Address, HashSet<Address>>>>,
    token_to_pools: Arc<RwLock<HashMap<Address, HashSet<Address>>>>,
    pool_to_token: Arc<RwLock<HashMap<Address, Address>>>,

    // Pre-computed sets for fast filtering
    active_creators: Arc<RwLock<HashSet<Address>>>,
    active_pools: Arc<RwLock<HashSet<Address>>>,
    high_liquidity_pools: Arc<RwLock<HashSet<Address>>>,
    scam_pools: Arc<RwLock<HashSet<Address>>>,

    // Configuration
    config: CacheConfig,
    log_path: Arc<RwLock<Option<PathBuf>>>,
}

impl TokenTrackingCache {
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        let max_tokens = NonZeroUsize::new(config.max_tokens).unwrap();
        let max_pools = NonZeroUsize::new(config.max_pools).unwrap();

        Self {
            tokens: Arc::new(RwLock::new(LruCache::new(max_tokens))),
            pools: Arc::new(RwLock::new(LruCache::new(max_pools))),
            creator_to_tokens: Arc::new(RwLock::new(HashMap::new())),
            token_to_pools: Arc::new(RwLock::new(HashMap::new())),
            pool_to_token: Arc::new(RwLock::new(HashMap::new())),
            active_creators: Arc::new(RwLock::new(HashSet::new())),
            active_pools: Arc::new(RwLock::new(HashSet::new())),
            high_liquidity_pools: Arc::new(RwLock::new(HashSet::new())),
            scam_pools: Arc::new(RwLock::new(HashSet::new())),
            config,
            log_path: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    // ===== Zero-Copy Read Operations =====

    /// Get a token by address (zero-copy via Arc)
    pub async fn get_token(&self, address: &Address) -> Option<Arc<Token>> {
        let address = normalize_address(address);
        let mut tokens = self.tokens.write().await; // LRU needs mut for access tracking
        tokens.get(&address).cloned()
    }

    /// Get a pool by address (zero-copy via Arc)
    pub async fn get_pool(&self, address: &Address) -> Option<Arc<Pool>> {
        let address = normalize_address(address);
        let mut pools = self.pools.write().await; // LRU needs mut for access tracking
        pools.get(&address).cloned()
    }

    /// Check if an address is a creator/owner/tax setter - O(1)
    pub async fn is_creator(&self, address: &Address) -> bool {
        let address = normalize_address(address);
        let creators = self.active_creators.read().await;
        creators.contains(&address)
    }

    /// Check if an address is a pool - O(1)
    pub async fn is_pool(&self, address: &Address) -> bool {
        let address = normalize_address(address);
        let pools = self.active_pools.read().await;
        pools.contains(&address)
    }

    /// Get the first token created by an address - O(1) index lookup
    pub async fn get_token_for_creator(&self, creator: &Address) -> Option<Arc<Token>> {
        let creator = normalize_address(creator);
        let creator_map = self.creator_to_tokens.read().await;
        if let Some(token_addresses) = creator_map.get(&creator) {
            if let Some(first_token) = token_addresses.iter().next() {
                return self.get_token(first_token).await;
            }
        }
        None
    }

    /// Get all tokens created by an address - O(1) index lookup
    pub async fn get_tokens_by_creator(&self, creator: &Address) -> Vec<Arc<Token>> {
        let creator = normalize_address(creator);
        let creator_map = self.creator_to_tokens.read().await;
        let mut result = Vec::new();

        if let Some(token_addresses) = creator_map.get(&creator) {
            // Get all tokens in parallel
            for token_addr in token_addresses {
                if let Some(token) = self.get_token(token_addr).await {
                    result.push(token);
                }
            }
        }

        result
    }

    /// Get all pools for a token - O(1) index lookup
    pub async fn get_pools_for_token(&self, token: &Address) -> Vec<Arc<Pool>> {
        let token = normalize_address(token);
        let token_map = self.token_to_pools.read().await;
        let mut result = Vec::new();

        if let Some(pool_addresses) = token_map.get(&token) {
            for pool_addr in pool_addresses {
                if let Some(pool) = self.get_pool(pool_addr).await {
                    result.push(pool);
                }
            }
        }

        result
    }

    /// Take a snapshot of all pools currently cached.
    pub async fn pools_snapshot(&self) -> Vec<(Address, Arc<Pool>)> {
        let pools = self.pools.read().await;
        pools
            .iter()
            .map(|(addr, pool)| (addr.clone(), Arc::clone(pool)))
            .collect()
    }

    /// Get all creator addresses (reference, no clone)
    pub async fn creator_addresses(&self) -> HashSet<Address> {
        let creators = self.active_creators.read().await;
        creators.clone() // Only clone the HashSet structure, not the data
    }

    /// Get all pool addresses above liquidity threshold
    pub async fn high_liquidity_pools(&self) -> HashSet<Address> {
        let pools = self.high_liquidity_pools.read().await;
        pools.clone()
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let tokens = self.tokens.read().await;
        let pools = self.pools.read().await;
        let creators = self.active_creators.read().await;

        CacheStats {
            total_tokens: tokens.len(),
            total_pools: pools.len(),
            total_creators: creators.len(),
            max_tokens: self.config.max_tokens,
            max_pools: self.config.max_pools,
        }
    }

    // ===== Batch Update Operations =====

    /// Update cache with new token data from Python (batch operation)
    pub async fn batch_update(&self, update: TokenUpdate) -> UpdateResult {
        let mut tokens_updated = 0;
        let mut pools_updated = 0;
        let mut creators_added = 0;

        let TokenUpdate {
            block_number, data, ..
        } = update;

        // Acquire all write locks at once to avoid deadlock
        let mut tokens_cache = self.tokens.write().await;
        let mut pools_cache = self.pools.write().await;
        let mut creator_to_tokens = self.creator_to_tokens.write().await;
        let mut token_to_pools = self.token_to_pools.write().await;
        let mut pool_to_token = self.pool_to_token.write().await;
        let mut active_creators = self.active_creators.write().await;
        let mut active_pools = self.active_pools.write().await;
        let mut high_liquidity_pools = self.high_liquidity_pools.write().await;
        let mut scam_pools = self.scam_pools.write().await;

        // Process each token and its pools
        for (token_addr, token_with_pools) in data {
            let token_addr = normalize_address(&token_addr);
            let mut token = token_with_pools.token;
            let pools = token_with_pools.pools;
            token.address = normalize_address(&token.address);
            token.creator_address = normalize_address(&token.creator_address);
            token.current_owner = normalize_address(&token.current_owner);
            token.tax_setter_addresses = token
                .tax_setter_addresses
                .into_iter()
                .map(|address| normalize_address(&address))
                .collect();

            // Calculate cached values
            token.total_liquidity = pools
                .values()
                .filter(|pool| !pool.is_scam)
                .filter(|pool| pool_is_viable(pool, self.config.eth_threshold))
                .map(|p| p.eth_reserve)
                .sum();

            // Update token
            let token_arc = Arc::new(token.clone());
            tokens_cache.put(token_addr.clone(), token_arc.clone());
            tokens_updated += 1;

            // Update creator index
            active_creators.insert(token.creator_address.clone());
            creator_to_tokens
                .entry(token.creator_address.clone())
                .or_insert_with(HashSet::new)
                .insert(token_addr.clone());

            // Add tax setters as creators
            for tax_setter in &token.tax_setter_addresses {
                active_creators.insert(tax_setter.clone());
                creator_to_tokens
                    .entry(tax_setter.clone())
                    .or_insert_with(HashSet::new)
                    .insert(token_addr.clone());
            }

            // Add current owner as creator
            active_creators.insert(token.current_owner.clone());
            creator_to_tokens
                .entry(token.current_owner.clone())
                .or_insert_with(HashSet::new)
                .insert(token_addr.clone());
            creators_added = active_creators.len();

            // Clear old pool mappings for this token
            if let Some(old_pools) = token_to_pools.get(&token_addr) {
                for old_pool in old_pools {
                    pool_to_token.remove(old_pool);
                    active_pools.remove(old_pool);
                    high_liquidity_pools.remove(old_pool);
                    scam_pools.remove(old_pool);
                }
            }

            // Update pools
            let mut pool_addrs = HashSet::new();
            for (pool_addr, mut pool) in pools {
                let pool_addr = normalize_address(&pool_addr);
                pool.address = normalize_address(&pool.address);
                pool.denom_address = normalize_address(&pool.denom_address);
                pool.control_addresses = pool
                    .control_addresses
                    .into_iter()
                    .map(|address| normalize_address(&address))
                    .collect();
                // Ensure token_address is set
                pool.token_address = token_addr.clone();

                if pool.is_scam {
                    pool_to_token.remove(&pool_addr);
                    active_pools.remove(&pool_addr);
                    high_liquidity_pools.remove(&pool_addr);
                    scam_pools.insert(pool_addr.clone());
                    continue;
                } else {
                    scam_pools.remove(&pool_addr);
                }

                let pool_arc = Arc::new(pool.clone());
                pools_cache.put(pool_addr.clone(), pool_arc);
                pools_updated += 1;

                // Update indexes
                pool_addrs.insert(pool_addr.clone());
                pool_to_token.insert(pool_addr.clone(), token_addr.clone());
                active_pools.insert(pool_addr.clone());

                for control_addr in &pool.control_addresses {
                    if control_addr.is_empty() {
                        continue;
                    }
                    active_creators.insert(control_addr.clone());
                    creator_to_tokens
                        .entry(control_addr.clone())
                        .or_insert_with(HashSet::new)
                        .insert(token_addr.clone());
                }

                // Track high liquidity pools
                let threshold = pool_liquidity_threshold(&pool, self.config.eth_threshold);
                if pool_is_viable(&pool, self.config.eth_threshold) && pool.eth_reserve >= threshold
                {
                    high_liquidity_pools.insert(pool_addr.clone());
                } else {
                    high_liquidity_pools.remove(&pool_addr);
                }
            }

            // Update token->pools mapping
            token_to_pools.insert(token_addr, pool_addrs);
        }

        self.log_update_to_file(block_number, tokens_updated, pools_updated, creators_added)
            .await;

        UpdateResult {
            tokens_updated,
            pools_updated,
            creators_added,
        }
    }

    pub async fn set_log_path<P: Into<PathBuf>>(&self, path: P) {
        let mut guard = self.log_path.write().await;
        *guard = Some(path.into());
    }

    async fn log_update_to_file(
        &self,
        block_number: u64,
        tokens_updated: usize,
        pools_updated: usize,
        creators_added: usize,
    ) {
        let log_path = { self.log_path.read().await.clone() };
        if let Some(path) = log_path {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(
                    file,
                    "[{}] @block {}: tokens={}, pools={}, creators={}",
                    timestamp, block_number, tokens_updated, pools_updated, creators_added
                );
            }
        }
    }

    /// Get pool by address
    pub async fn get_pool_by_address(&self, address: &str) -> Option<Arc<Pool>> {
        self.get_pool(&address.to_string()).await
    }

    /// Get token info
    pub async fn get_token_info(&self, address: &str) -> Option<Token> {
        self.get_token(&address.to_string())
            .await
            .map(|arc| (*arc).clone())
    }

    /// Get creator count
    pub async fn get_creator_count(&self) -> usize {
        let creators = self.active_creators.read().await;
        creators.len()
    }

    /// Get pool count
    pub async fn get_pool_count(&self) -> usize {
        let pools = self.pools.read().await;
        pools.len()
    }
}

/// Statistics about the cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_tokens: usize,
    pub total_pools: usize,
    pub total_creators: usize,
    pub max_tokens: usize,
    pub max_pools: usize,
}

/// Result of a batch update operation
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub tokens_updated: usize,
    pub pools_updated: usize,
    pub creators_added: usize,
}

fn pool_is_viable(pool: &Pool, fallback_eth_threshold: f64) -> bool {
    match pool.lifecycle {
        PoolLifecycle::LiquidityDeposited | PoolLifecycle::Active => true,
        PoolLifecycle::Scam | PoolLifecycle::Evicted => false,
        PoolLifecycle::Discovered | PoolLifecycle::Unknown => {
            pool.eth_reserve >= pool_liquidity_threshold(pool, fallback_eth_threshold)
        }
    }
}

fn pool_liquidity_threshold(pool: &Pool, fallback_eth_threshold: f64) -> f64 {
    let symbol = pool.denom_currency.trim();
    if !symbol.is_empty() {
        if let Some(threshold) = threshold_for_symbol(symbol) {
            return threshold;
        }

        if symbol.eq_ignore_ascii_case("ETH") {
            return fallback_eth_threshold;
        }
    }

    // Treat the zero address as native ETH in V4 pools
    if pool
        .denom_address
        .eq_ignore_ascii_case("0x0000000000000000000000000000000000000000")
    {
        return fallback_eth_threshold;
    }

    fallback_eth_threshold
}

fn normalize_address(address: &str) -> String {
    address.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_operations() {
        let cache = TokenTrackingCache::with_defaults();

        // Create test token
        let token = Token {
            address: "0xTOKEN".to_string(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            decimals: 18,
            total_supply: Some("1000000".to_string()),
            creator_address: "0xCREATOR".to_string(),
            current_owner: "0xOWNER".to_string(),
            tax_setter_addresses: vec!["0xTAXSETTER".to_string()],
            ownership_renounced: false,
            renouncement_block: None,
            buy_tax: Some(5.0),
            sell_tax: Some(5.0),
            last_tax_change_block: None,
            tax_history: vec![],
            creation_block: 1000,
            creation_tx: "0xHASH".to_string(),
            creation_timestamp: Some(1234567890.0),
            latest_activity_block: 2000,
            is_scam: false,
            scam_label: None,
            total_liquidity: 0.0,
        };

        // Create test pool
        let pool = Pool {
            address: "0xPOOL".to_string(),
            token_address: "0xTOKEN".to_string(),
            pool_type: super::super::types::PoolType::UniswapV2,
            token_reserve: 1000000.0,
            eth_reserve: 10.0,
            denom_currency: "ETH".to_string(),
            denom_address: "0xWETH".to_string(),
            trading_enabled: true,
            trading_enabled_block: Some(1500),
            trading_enabled_tx: Some("0xTRADING".to_string()),
            fee_tier: None,
            pool_id: None,
            last_updated_block: 2000,
            last_updated_time: 1234567900.0,
            is_scam: false,
            scam_label: None,
            lp_tokens_approved_percentage: None,
            lifecycle: PoolLifecycle::Active,
            control_addresses: Vec::new(),
            can_buy: true,
            can_sell: true,
            received_at: std::time::Instant::now(),
        };

        // Create update
        let mut token_data = HashMap::new();
        let mut pools_map = HashMap::new();
        pools_map.insert("0xPOOL".to_string(), pool);

        token_data.insert(
            "0xTOKEN".to_string(),
            TokenWithPools {
                token,
                pools: pools_map,
            },
        );

        let update = TokenUpdate {
            message_type: "test".to_string(),
            token_count: 1,
            block_number: 2000,
            timestamp: 1234567900.0,
            data: token_data,
        };

        // Perform update
        let result = cache.batch_update(update).await;
        assert_eq!(result.tokens_updated, 1);
        assert_eq!(result.pools_updated, 1);
        assert_eq!(result.creators_added, 3); // creator, owner, tax setter

        // Test reads
        assert!(cache.is_creator(&"0xCREATOR".to_string()).await);
        assert!(cache.is_creator(&"0xOWNER".to_string()).await);
        assert!(cache.is_creator(&"0xTAXSETTER".to_string()).await);
        assert!(cache.is_pool(&"0xPOOL".to_string()).await);

        let token = cache.get_token(&"0xTOKEN".to_string()).await.unwrap();
        assert_eq!(token.symbol, "TEST");

        let pool = cache.get_pool(&"0xPOOL".to_string()).await.unwrap();
        assert_eq!(pool.eth_reserve, 10.0);

        let creator_token = cache
            .get_token_for_creator(&"0xCREATOR".to_string())
            .await
            .unwrap();
        assert_eq!(creator_token.address, "0xtoken");

        let pools = cache.get_pools_for_token(&"0xTOKEN".to_string()).await;
        assert_eq!(pools.len(), 1);
        assert_eq!(pools[0].address, "0xpool");
    }
}
