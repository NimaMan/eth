// token_tracking/address_tracking_cache.rs
//
// High-performance cache for token tracking and signal detection
// Optimized for mempool transaction analysis and pool state monitoring

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Represents the role of an address in relation to a token
#[derive(Debug, Clone, PartialEq)]
pub enum AddressRole {
    Creator,
    Owner,
    Both,
}

/// Function call record for tracking creator/owner activity
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub tx_hash: String,
    pub function_selector: [u8; 4],
    pub function_name: String,
    pub block_number: Option<u64>,
    pub timestamp: u64,
    pub gas_price: Option<u64>,
    pub success: Option<bool>,
}

/// Activity tracking for a specific address
#[derive(Debug, Clone)]
pub struct AddressActivity {
    pub address: String,
    pub tokens: HashMap<String, AddressRole>, // token_address -> role
    pub function_history: VecDeque<FunctionCall>, // Limited history
    pub total_functions_called: u64,
    pub last_activity_timestamp: u64,
    pub is_high_risk: bool, // Flag for known scammers
}

impl AddressActivity {
    pub fn new(address: String) -> Self {
        Self {
            address,
            tokens: HashMap::new(),
            function_history: VecDeque::with_capacity(100), // Keep last 100 calls
            total_functions_called: 0,
            last_activity_timestamp: 0,
            is_high_risk: false,
        }
    }
    
    pub fn add_function_call(&mut self, call: FunctionCall) {
        self.last_activity_timestamp = call.timestamp;
        self.total_functions_called += 1;
        
        // Maintain history limit
        if self.function_history.len() >= 100 {
            self.function_history.pop_front();
        }
        self.function_history.push_back(call);
    }
}

/// Pool information for monitoring
#[derive(Debug, Clone)]
pub struct PoolMonitoringInfo {
    pub pool_address: String,
    pub token_address: String,
    pub pool_type: String, // V2, V3, V4
    pub current_eth_reserve: f64,
    pub current_token_reserve: f64,
    pub last_update_block: u64,
    pub creation_block: u64,
    pub is_primary_pool: bool, // Main liquidity pool for this token
}

/// Complete token information including creators, owners, and pools
#[derive(Debug, Clone)]
pub struct TokenTrackingInfo {
    pub token_address: String,
    pub creator_address: String,
    pub current_owner: String,
    pub previous_owners: Vec<String>, // Ownership history
    pub pools: HashMap<String, PoolMonitoringInfo>, // pool_address -> info
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<u64>,
    pub creation_block: u64,
    pub is_scam: bool,
    pub scam_reason: Option<String>,
}

/// Main token tracking cache optimized for signal detection
#[derive(Clone)]
pub struct AddressTrackingCache {
    /// Primary index: address -> activity tracking
    /// This is the main lookup for mempool transaction matching
    address_activity: Arc<RwLock<HashMap<String, AddressActivity>>>,
    
    /// Token information: token_address -> full token data
    token_info: Arc<RwLock<HashMap<String, TokenTrackingInfo>>>,
    
    /// Pool lookup: pool_address -> token_address (for quick pool identification)
    pool_to_token: Arc<RwLock<HashMap<String, String>>>,
    
    /// High-risk addresses for priority monitoring
    high_risk_addresses: Arc<RwLock<HashMap<String, String>>>, // address -> reason
    
    /// Configuration
    max_addresses: usize,
    max_tokens: usize,
}

impl AddressTrackingCache {
    pub fn new() -> Self {
        Self {
            address_activity: Arc::new(RwLock::new(HashMap::new())),
            token_info: Arc::new(RwLock::new(HashMap::new())),
            pool_to_token: Arc::new(RwLock::new(HashMap::new())),
            high_risk_addresses: Arc::new(RwLock::new(HashMap::new())),
            max_addresses: 200_000, // Track up to 200K addresses
            max_tokens: 100_000,    // Track up to 100K tokens
        }
    }
    
    /// Update cache with new token data from Python
    pub async fn update_token_data(
        &self,
        token_address: &str,
        creator: &str,
        owner: &str,
        pools: Vec<(String, f64, f64)>, // (pool_address, eth_reserve, token_reserve)
        trading_enabled: bool,
        is_scam: bool,
    ) {
        // Update token info
        let mut token_info_guard = self.token_info.write().await;
        let token_info = token_info_guard.entry(token_address.to_string())
            .or_insert_with(|| TokenTrackingInfo {
                token_address: token_address.to_string(),
                creator_address: creator.to_string(),
                current_owner: owner.to_string(),
                previous_owners: Vec::new(),
                pools: HashMap::new(),
                trading_enabled,
                trading_enabled_block: None,
                creation_block: 0,
                is_scam,
                scam_reason: None,
            });
        
        // Update owner if changed
        if token_info.current_owner != owner {
            token_info.previous_owners.push(token_info.current_owner.clone());
            token_info.current_owner = owner.to_string();
        }
        
        token_info.trading_enabled = trading_enabled;
        token_info.is_scam = is_scam;
        
        // Update pools
        for (pool_addr, eth_reserve, token_reserve) in pools.iter() {
            let pool_info = PoolMonitoringInfo {
                pool_address: pool_addr.clone(),
                token_address: token_address.to_string(),
                pool_type: "V2".to_string(), // TODO: Get from data
                current_eth_reserve: *eth_reserve,
                current_token_reserve: *token_reserve,
                last_update_block: 0, // TODO: Get from data
                creation_block: 0,
                is_primary_pool: true, // TODO: Determine primary
            };
            token_info.pools.insert(pool_addr.clone(), pool_info);
        }
        
        drop(token_info_guard);
        
        // Update address activity for creator
        let mut address_activity_guard = self.address_activity.write().await;
        let creator_activity = address_activity_guard.entry(creator.to_string())
            .or_insert_with(|| AddressActivity::new(creator.to_string()));
        creator_activity.tokens.insert(token_address.to_string(), AddressRole::Creator);
        
        // Update address activity for owner (might be same as creator)
        if creator != owner {
            let owner_activity = address_activity_guard.entry(owner.to_string())
                .or_insert_with(|| AddressActivity::new(owner.to_string()));
            owner_activity.tokens.insert(token_address.to_string(), AddressRole::Owner);
        } else {
            // Same address is both creator and owner
            creator_activity.tokens.insert(token_address.to_string(), AddressRole::Both);
        }
        
        drop(address_activity_guard);
        
        // Update pool lookup
        let mut pool_lookup_guard = self.pool_to_token.write().await;
        for (pool_addr, _, _) in pools.iter() {
            pool_lookup_guard.insert(pool_addr.clone(), token_address.to_string());
        }
        
        debug!("Updated token data for {} with {} pools", token_address, pool_lookup_guard.len());
    }
    
    /// Check if an address is a creator or owner (main entry point for mempool analysis)
    pub async fn get_address_info(&self, address: &str) -> Option<AddressActivity> {
        let guard = self.address_activity.read().await;
        guard.get(address).cloned()
    }
    
    /// Record a function call from a tracked address
    pub async fn record_function_call(
        &self,
        address: &str,
        function_selector: [u8; 4],
        function_name: String,
        tx_hash: String,
        timestamp: u64,
    ) {
        let mut guard = self.address_activity.write().await;
        if let Some(activity) = guard.get_mut(address) {
            let call = FunctionCall {
                tx_hash,
                function_selector,
                function_name: function_name.clone(),
                block_number: None,
                timestamp,
                gas_price: None,
                success: None,
            };
            activity.add_function_call(call);
            
            info!("Recorded function call {} from tracked address {}", function_name, address);
        }
    }
    
    /// Get pool info for post-simulation analysis
    pub async fn get_pool_info(&self, pool_address: &str) -> Option<(String, PoolMonitoringInfo)> {
        let pool_lookup = self.pool_to_token.read().await;
        if let Some(token_address) = pool_lookup.get(pool_address) {
            let token_info_guard = self.token_info.read().await;
            if let Some(token_info) = token_info_guard.get(token_address) {
                if let Some(pool_info) = token_info.pools.get(pool_address) {
                    return Some((token_address.clone(), pool_info.clone()));
                }
            }
        }
        None
    }
    
    /// Check if address is high risk
    pub async fn is_high_risk_address(&self, address: &str) -> bool {
        let guard = self.high_risk_addresses.read().await;
        guard.contains_key(address)
    }
    
    /// Mark an address as high risk
    pub async fn mark_address_high_risk(&self, address: &str, reason: &str) {
        let mut guard = self.high_risk_addresses.write().await;
        guard.insert(address.to_string(), reason.to_string());
        
        // Also update in address activity
        let mut activity_guard = self.address_activity.write().await;
        if let Some(activity) = activity_guard.get_mut(address) {
            activity.is_high_risk = true;
        }
        
        warn!("Marked address {} as high risk: {}", address, reason);
    }
    
    /// Get all tokens created or owned by an address
    pub async fn get_address_tokens(&self, address: &str) -> Vec<(String, AddressRole)> {
        let guard = self.address_activity.read().await;
        if let Some(activity) = guard.get(address) {
            activity.tokens.iter()
                .map(|(addr, role)| (addr.clone(), role.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Get statistics about the cache
    pub async fn get_stats(&self) -> CacheStats {
        let addresses = self.address_activity.read().await.len();
        let tokens = self.token_info.read().await.len();
        let pools = self.pool_to_token.read().await.len();
        let high_risk = self.high_risk_addresses.read().await.len();
        
        CacheStats {
            tracked_addresses: addresses,
            tracked_tokens: tokens,
            tracked_pools: pools,
            high_risk_addresses: high_risk,
        }
    }
    
    /// Clean up stale entries (addresses with no activity for 24 hours)
    pub async fn cleanup_stale_entries(&self, max_age_seconds: u64) -> usize {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut guard = self.address_activity.write().await;
        let initial_count = guard.len();
        
        guard.retain(|_, activity| {
            current_time - activity.last_activity_timestamp < max_age_seconds
        });
        
        let removed = initial_count - guard.len();
        if removed > 0 {
            info!("Cleaned up {} stale address entries", removed);
        }
        
        removed
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub tracked_addresses: usize,
    pub tracked_tokens: usize,
    pub tracked_pools: usize,
    pub high_risk_addresses: usize,
}

impl Default for AddressTrackingCache {
    fn default() -> Self {
        Self::new()
    }
}