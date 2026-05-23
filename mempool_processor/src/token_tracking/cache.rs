// cache_v2.rs - High-performance token tracking cache with zero-copy access
//
// Key improvements:
// 1. LRU eviction for bounded memory
// 2. Arc-wrapped data for zero-copy reads
// 3. Indexed lookups for O(1) access
// 4. Batch updates with single lock acquisition
use alloy_primitives::U256;
use chrono::Utc;
use lru::LruCache;
use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::liquidity_ownership::liquidity_ownership_token_addresses;
use super::pool_filters::{pool_is_viable, pool_liquidity_threshold};
use super::position_index::{
    add_position_indexes_for_pool, normalize_pool_positions, position_owner_manager_key,
    position_token_key, remove_position_indexes_for_pool,
};
pub use super::types::{
    Address, CacheConfig, ConcentratedPositionApprovalContext, Pool, PoolLifecycle, Token,
    TokenUpdate, TokenWithPools,
};

#[derive(Debug, Clone, Default)]
struct TokenCacheContext {
    last_accepted_block: u64,
    last_accepted_status: Option<String>,
    last_accepted_source: Option<String>,
    accepted_updates: u64,
    rejected_stale_snapshots: u64,
    rejected_non_live_snapshots: u64,
}

#[derive(Debug, Clone, Default)]
pub struct TokenCacheContextSnapshot {
    pub last_accepted_block: u64,
    pub last_accepted_status: Option<String>,
    pub last_accepted_source: Option<String>,
    pub accepted_updates: u64,
    pub rejected_stale_snapshots: u64,
    pub rejected_non_live_snapshots: u64,
}

#[derive(Debug, Clone)]
pub struct CacheUpdateContext {
    pub source: String,
    pub status: Option<String>,
    pub status_policy: CacheStatusPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatusPolicy {
    Any,
    LiveOnly,
    LiveOrWarmingUntilLive,
}

#[derive(Debug, Clone)]
pub enum CacheApplyOutcome {
    Applied(UpdateResult),
    Rejected(CacheUpdateRejection),
    SkippedEmpty,
}

impl CacheApplyOutcome {
    pub fn applied(&self) -> bool {
        matches!(self, Self::Applied(_))
    }
}

#[derive(Debug, Clone)]
pub struct CacheUpdateRejection {
    pub source: String,
    pub status: Option<String>,
    pub block_number: u64,
    pub current_block: u64,
    pub reason: CacheUpdateRejectionReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheUpdateRejectionReason {
    NonLiveStatus,
    StaleBlock,
}

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
    ownership_token_to_pool: Arc<RwLock<HashMap<Address, Address>>>,
    position_manager_to_pools: Arc<RwLock<HashMap<Address, HashSet<Address>>>>,
    position_token_to_context: Arc<RwLock<HashMap<String, ConcentratedPositionApprovalContext>>>,
    position_owner_manager_to_context:
        Arc<RwLock<HashMap<String, Vec<ConcentratedPositionApprovalContext>>>>,

    // Pre-computed sets for fast filtering
    active_creators: Arc<RwLock<HashSet<Address>>>,
    active_pools: Arc<RwLock<HashSet<Address>>>,
    high_liquidity_pools: Arc<RwLock<HashSet<Address>>>,
    scam_pools: Arc<RwLock<HashSet<Address>>>,

    // Configuration
    config: CacheConfig,
    log_path: Arc<RwLock<Option<PathBuf>>>,
    context: Arc<RwLock<TokenCacheContext>>,
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
            ownership_token_to_pool: Arc::new(RwLock::new(HashMap::new())),
            position_manager_to_pools: Arc::new(RwLock::new(HashMap::new())),
            position_token_to_context: Arc::new(RwLock::new(HashMap::new())),
            position_owner_manager_to_context: Arc::new(RwLock::new(HashMap::new())),
            active_creators: Arc::new(RwLock::new(HashSet::new())),
            active_pools: Arc::new(RwLock::new(HashSet::new())),
            high_liquidity_pools: Arc::new(RwLock::new(HashSet::new())),
            scam_pools: Arc::new(RwLock::new(HashSet::new())),
            config,
            log_path: Arc::new(RwLock::new(None)),
            context: Arc::new(RwLock::new(TokenCacheContext::default())),
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

    /// Check if an address is a tracked token without updating LRU state.
    pub async fn is_token(&self, address: &Address) -> bool {
        let address = normalize_address(address);
        let tokens = self.tokens.read().await;
        tokens.contains(&address)
    }

    pub async fn is_liquidity_ownership_token(&self, address: &Address) -> bool {
        let address = normalize_address(address);
        let ownership_tokens = self.ownership_token_to_pool.read().await;
        ownership_tokens.contains_key(&address)
    }

    pub async fn get_pool_by_liquidity_ownership_token(
        &self,
        address: &Address,
    ) -> Option<Arc<Pool>> {
        let address = normalize_address(address);
        let pool_address = {
            let ownership_tokens = self.ownership_token_to_pool.read().await;
            ownership_tokens.get(&address).cloned()
        };

        match pool_address {
            Some(pool_address) => self.get_pool(&pool_address).await,
            None => self.get_pool(&address).await,
        }
    }

    pub async fn is_position_manager(&self, address: &Address) -> bool {
        let address = normalize_address(address);
        let managers = self.position_manager_to_pools.read().await;
        managers.contains_key(&address)
    }

    pub async fn position_context_by_token_id(
        &self,
        position_manager: &Address,
        token_id: U256,
    ) -> Option<ConcentratedPositionApprovalContext> {
        let key = position_token_key(position_manager, token_id);
        let contexts = self.position_token_to_context.read().await;
        contexts.get(&key).cloned()
    }

    pub async fn position_contexts_by_owner(
        &self,
        position_manager: &Address,
        owner: &Address,
    ) -> Vec<ConcentratedPositionApprovalContext> {
        let key = position_owner_manager_key(position_manager, owner);
        let contexts = self.position_owner_manager_to_context.read().await;
        contexts.get(&key).cloned().unwrap_or_default()
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

    pub async fn context_snapshot(&self) -> TokenCacheContextSnapshot {
        let context = self.context.read().await;
        TokenCacheContextSnapshot {
            last_accepted_block: context.last_accepted_block,
            last_accepted_status: context.last_accepted_status.clone(),
            last_accepted_source: context.last_accepted_source.clone(),
            accepted_updates: context.accepted_updates,
            rejected_stale_snapshots: context.rejected_stale_snapshots,
            rejected_non_live_snapshots: context.rejected_non_live_snapshots,
        }
    }

    // ===== Batch Update Operations =====

    pub async fn batch_update_with_context(
        &self,
        update: TokenUpdate,
        update_context: CacheUpdateContext,
    ) -> CacheApplyOutcome {
        if update.data.is_empty() {
            return CacheApplyOutcome::SkippedEmpty;
        }

        let normalized_status = update_context
            .status
            .as_ref()
            .map(|status| status.trim().to_ascii_lowercase());
        let block_number = update.block_number;

        let mut context = self.context.write().await;
        let current_status = context
            .last_accepted_status
            .as_ref()
            .map(|status| status.trim().to_ascii_lowercase());
        let status_allowed = match update_context.status_policy {
            CacheStatusPolicy::Any => true,
            CacheStatusPolicy::LiveOnly => normalized_status.as_deref() == Some("live"),
            CacheStatusPolicy::LiveOrWarmingUntilLive => {
                normalized_status.as_deref() == Some("live")
                    || (normalized_status.as_deref() == Some("warming")
                        && current_status.as_deref() != Some("live"))
            }
        };

        if !status_allowed {
            context.rejected_non_live_snapshots += 1;
            let rejection = CacheUpdateRejection {
                source: update_context.source,
                status: update_context.status,
                block_number,
                current_block: context.last_accepted_block,
                reason: CacheUpdateRejectionReason::NonLiveStatus,
            };
            drop(context);
            self.log_context_rejection_to_file(&rejection).await;
            return CacheApplyOutcome::Rejected(rejection);
        }

        if context.last_accepted_block > 0
            && block_number > 0
            && block_number < context.last_accepted_block
        {
            context.rejected_stale_snapshots += 1;
            let rejection = CacheUpdateRejection {
                source: update_context.source,
                status: update_context.status,
                block_number,
                current_block: context.last_accepted_block,
                reason: CacheUpdateRejectionReason::StaleBlock,
            };
            drop(context);
            self.log_context_rejection_to_file(&rejection).await;
            return CacheApplyOutcome::Rejected(rejection);
        }

        let source = update_context.source;
        let status = update_context.status;
        let result = self.batch_update(update).await;
        context.last_accepted_block = context.last_accepted_block.max(block_number);
        context.last_accepted_status = status;
        context.last_accepted_source = Some(source);
        context.accepted_updates += 1;

        CacheApplyOutcome::Applied(result)
    }

    /// Update cache with token-server token/pool context.
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
        let mut ownership_token_to_pool = self.ownership_token_to_pool.write().await;
        let mut position_manager_to_pools = self.position_manager_to_pools.write().await;
        let mut position_token_to_context = self.position_token_to_context.write().await;
        let mut position_owner_manager_to_context =
            self.position_owner_manager_to_context.write().await;
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
                    ownership_token_to_pool.retain(|_, mapped_pool| mapped_pool != old_pool);
                    remove_position_indexes_for_pool(
                        old_pool,
                        &mut position_manager_to_pools,
                        &mut position_token_to_context,
                        &mut position_owner_manager_to_context,
                    );
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
                pool.position_manager_address = pool
                    .position_manager_address
                    .map(|address| normalize_address(&address));
                pool.lp_token_address = pool
                    .lp_token_address
                    .map(|address| normalize_address(&address));
                pool.control_addresses = pool
                    .control_addresses
                    .into_iter()
                    .map(|address| normalize_address(&address))
                    .collect();
                normalize_pool_positions(&mut pool);
                // Ensure token_address is set
                pool.token_address = token_addr.clone();

                if pool.is_scam {
                    pool_to_token.remove(&pool_addr);
                    remove_position_indexes_for_pool(
                        &pool_addr,
                        &mut position_manager_to_pools,
                        &mut position_token_to_context,
                        &mut position_owner_manager_to_context,
                    );
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
                for ownership_token in liquidity_ownership_token_addresses(&pool) {
                    ownership_token_to_pool.insert(ownership_token, pool_addr.clone());
                }
                add_position_indexes_for_pool(
                    &pool,
                    &mut position_manager_to_pools,
                    &mut position_token_to_context,
                    &mut position_owner_manager_to_context,
                );

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

    async fn log_context_rejection_to_file(&self, rejection: &CacheUpdateRejection) {
        let log_path = { self.log_path.read().await.clone() };
        if let Some(path) = log_path {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f");
                let _ = writeln!(
                    file,
                    "[{}] rejected token cache update source={} status={:?} block={} current_block={} reason={:?}",
                    timestamp,
                    rejection.source,
                    rejection.status,
                    rejection.block_number,
                    rejection.current_block,
                    rejection.reason
                );
            }
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
            lp_token_address: None,
            position_manager_address: None,
            lp_total_supply: None,
            liquidity_positions: Vec::new(),
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

    #[tokio::test]
    async fn rejects_non_live_context_when_live_required() {
        let cache = TokenTrackingCache::with_defaults();
        let update = test_update(100);

        let outcome = cache
            .batch_update_with_context(
                update,
                CacheUpdateContext {
                    source: "live_token_server_sync".to_string(),
                    status: Some("warming".to_string()),
                    status_policy: CacheStatusPolicy::LiveOnly,
                },
            )
            .await;

        match outcome {
            CacheApplyOutcome::Rejected(rejection) => {
                assert_eq!(rejection.reason, CacheUpdateRejectionReason::NonLiveStatus);
            }
            other => panic!("expected rejection, got {other:?}"),
        }
        let context = cache.context_snapshot().await;
        assert_eq!(context.accepted_updates, 0);
        assert_eq!(context.rejected_non_live_snapshots, 1);
    }

    #[tokio::test]
    async fn accepts_warming_context_until_live_context_exists() {
        let cache = TokenTrackingCache::with_defaults();

        let outcome = cache
            .batch_update_with_context(
                test_update(100),
                CacheUpdateContext {
                    source: "live_token_server_sync".to_string(),
                    status: Some("warming".to_string()),
                    status_policy: CacheStatusPolicy::LiveOrWarmingUntilLive,
                },
            )
            .await;

        assert!(outcome.applied());
        let context = cache.context_snapshot().await;
        assert_eq!(context.last_accepted_block, 100);
        assert_eq!(context.last_accepted_status.as_deref(), Some("warming"));
    }

    #[tokio::test]
    async fn rejects_warming_context_after_live_context_exists() {
        let cache = TokenTrackingCache::with_defaults();

        let live = cache
            .batch_update_with_context(
                test_update(200),
                CacheUpdateContext {
                    source: "live_token_server_sync".to_string(),
                    status: Some("live".to_string()),
                    status_policy: CacheStatusPolicy::LiveOrWarmingUntilLive,
                },
            )
            .await;
        assert!(live.applied());

        let warming = cache
            .batch_update_with_context(
                test_update(201),
                CacheUpdateContext {
                    source: "live_token_server_sync".to_string(),
                    status: Some("warming".to_string()),
                    status_policy: CacheStatusPolicy::LiveOrWarmingUntilLive,
                },
            )
            .await;

        match warming {
            CacheApplyOutcome::Rejected(rejection) => {
                assert_eq!(rejection.reason, CacheUpdateRejectionReason::NonLiveStatus);
                assert_eq!(rejection.current_block, 200);
            }
            other => panic!("expected rejection, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn rejects_lower_block_after_live_context() {
        let cache = TokenTrackingCache::with_defaults();

        let applied = cache
            .batch_update_with_context(
                test_update(200),
                CacheUpdateContext {
                    source: "live_token_server_sync".to_string(),
                    status: Some("live".to_string()),
                    status_policy: CacheStatusPolicy::LiveOnly,
                },
            )
            .await;
        assert!(applied.applied());

        let stale = cache
            .batch_update_with_context(
                test_update(199),
                CacheUpdateContext {
                    source: "live_token_server_sync".to_string(),
                    status: Some("live".to_string()),
                    status_policy: CacheStatusPolicy::LiveOnly,
                },
            )
            .await;

        match stale {
            CacheApplyOutcome::Rejected(rejection) => {
                assert_eq!(rejection.reason, CacheUpdateRejectionReason::StaleBlock);
                assert_eq!(rejection.current_block, 200);
            }
            other => panic!("expected stale rejection, got {other:?}"),
        }
        let context = cache.context_snapshot().await;
        assert_eq!(context.last_accepted_block, 200);
        assert_eq!(context.rejected_stale_snapshots, 1);
    }

    fn test_update(block_number: u64) -> TokenUpdate {
        let token_address = "0x1000000000000000000000000000000000000001".to_string();
        let pool_address = "0x2000000000000000000000000000000000000002".to_string();
        let creator_address = "0x3000000000000000000000000000000000000003".to_string();
        let token = Token {
            address: token_address.clone(),
            symbol: "TEST".to_string(),
            name: "Test".to_string(),
            decimals: 18,
            total_supply: Some("1000".to_string()),
            creator_address: creator_address.clone(),
            current_owner: creator_address,
            tax_setter_addresses: Vec::new(),
            ownership_renounced: false,
            renouncement_block: None,
            buy_tax: None,
            sell_tax: None,
            last_tax_change_block: None,
            tax_history: Vec::new(),
            creation_block: block_number,
            creation_tx: "0xHASH".to_string(),
            creation_timestamp: None,
            latest_activity_block: block_number,
            is_scam: false,
            scam_label: None,
            total_liquidity: 0.0,
        };
        let pool = Pool {
            address: pool_address.clone(),
            token_address: token_address.clone(),
            pool_type: super::super::types::PoolType::UniswapV2,
            token_reserve: 1000000.0,
            eth_reserve: 10.0,
            denom_currency: "ETH".to_string(),
            denom_address: "0xWETH".to_string(),
            trading_enabled: true,
            trading_enabled_block: Some(block_number),
            trading_enabled_tx: None,
            fee_tier: None,
            pool_id: None,
            lp_token_address: None,
            position_manager_address: None,
            lp_total_supply: None,
            liquidity_positions: Vec::new(),
            last_updated_block: block_number,
            last_updated_time: 0.0,
            is_scam: false,
            scam_label: None,
            lp_tokens_approved_percentage: None,
            lifecycle: PoolLifecycle::Active,
            control_addresses: Vec::new(),
            can_buy: true,
            can_sell: true,
            received_at: std::time::Instant::now(),
        };
        let mut pools = HashMap::new();
        pools.insert(pool_address, pool);
        let mut data = HashMap::new();
        data.insert(token_address, TokenWithPools { token, pools });
        TokenUpdate {
            message_type: "test".to_string(),
            token_count: 1,
            block_number,
            timestamp: 0.0,
            data,
        }
    }
}
