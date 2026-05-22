/// Trading Status Detector
///
/// Detects when trading becomes enabled on pools based on simulation results.
/// Triggers signals when: can_buy && can_sell && taxes are acceptable by eth_token::TaxBucket
/// and not already enabled.
use crate::simulator::SimulationResult;
use crate::token_tracking::TokenTrackingCache;
use eth_token::pools::TaxBucket;
use reth_chain_query::to_checksum_address;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct TradingStatusSignal {
    pub token_address: String,
    pub pool_address: String,
    pub pool_type: String, // Pool type (UniswapV2, UniswapV3, etc.)
    pub status_change: TradingStatusChange,
    pub executor: String,
    pub can_trade_after: bool,
    pub buy_tax: Option<f64>,
    pub sell_tax: Option<f64>,
    pub tx_hash: String,
    pub details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TradingStatusChange {
    /// Trading has been enabled
    TradingEnabled,
    /// Trading has been disabled
    TradingDisabled,
    /// Trading was paused
    TradingPaused,
    /// Trading status unchanged
    NoChange,
}

pub struct TradingStatusDetector {
    /// Token tracking cache to check existing trading status
    token_cache: Option<Arc<TokenTrackingCache>>,
    /// Tracks (token, pool) pairs we have already emitted signals for during this run
    emitted_trading_pairs: Arc<Mutex<HashSet<(String, String)>>>,
}

impl TradingStatusDetector {
    pub fn new() -> Self {
        Self {
            token_cache: None,
            emitted_trading_pairs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Set the token cache for checking existing trading status
    pub fn set_token_cache(&mut self, cache: Arc<TokenTrackingCache>) {
        self.token_cache = Some(cache);
    }

    /// Detect trading enabled signals from simulation results
    /// Triggers when: can_buy && can_sell && taxes are in an acceptable TaxBucket
    /// and not already enabled.
    /// Tax values are extracted from sim_result.buy_sell_result (calculated in simulation_manager)
    pub async fn detect(
        &self,
        sim_result: &SimulationResult,
        _buy_tax: Option<f64>,
        _sell_tax: Option<f64>,
    ) -> Option<TradingStatusSignal> {
        // Get buy/sell results from simulation
        let buy_sell = sim_result.buy_sell_result()?;

        // Extract transaction details
        let tx_hash = &sim_result.request.tx.hash;

        // Extract token/pool info from simulation result - MUST use the actual pool from simulation
        let token_address = if let Some(token_addr) = &sim_result.token_address {
            to_checksum_address(token_addr)
        } else {
            // Fallback to transaction category if not in sim result
            match &sim_result.request.category {
                crate::tx_router::TransactionCategory::CreatorTransaction {
                    target_token, ..
                } => target_token.as_ref()?.clone(),
                crate::tx_router::TransactionCategory::ContractCreation {
                    contract_address,
                    ..
                } => contract_address.clone(),
                _ => return None,
            }
        };

        // CRITICAL: Use pool address from simulation result, NOT from transaction category
        let pool_address = if let Some(pool_addr) = &sim_result.pool_address {
            to_checksum_address(pool_addr)
        } else {
            warn!(
                "No pool address in simulation result for token {}",
                token_address
            );
            return None;
        };

        // Get pool type from simulation result
        let pool_type = sim_result
            .pool_type
            .as_ref()
            .unwrap_or(&"Unknown".to_string())
            .clone();

        let already_enabled_in_cache = self
            .is_trading_already_enabled(&token_address, &pool_address)
            .await;
        if already_enabled_in_cache {
            info!(
                "ℹ️ TradingEnabled skipped for token {} pool {} (tx {}): pool is already tradable in cache",
                token_address, pool_address, tx_hash
            );
            return None;
        }

        // Get executor from transaction category
        let executor = match &sim_result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { creator, .. } => {
                creator.clone()
            }
            crate::tx_router::TransactionCategory::ContractCreation { deployer, .. } => {
                deployer.clone()
            }
            _ => return None,
        };

        // Extract tax values from simulation result (calculated in simulation_manager)
        let buy_tax = buy_sell.buy_tax;
        let sell_tax = buy_sell.sell_tax;

        // Check if trading works (both buy and sell)
        if !buy_sell.can_buy || !buy_sell.can_approve || !buy_sell.can_sell {
            info!(
                "⚠️ TradingEnabled skipped for token {} pool {} (tx {}): can_buy={} can_approve={} can_sell={}",
                token_address, pool_address, tx_hash, buy_sell.can_buy, buy_sell.can_approve, buy_sell.can_sell
            );
            return None;
        }

        // Check if taxes are in the accepted bucket range.
        // If taxes are too high, trading is not really "enabled" in a practical sense.
        // The accepted buckets live in eth_token so mempool signals use the same
        // labels and boundaries as token analytics.
        // IMPORTANT: If tax calculation fails (None), we cannot generate a TRADING_ENABLED signal
        let buy_tax_bucket = TaxBucket::from_percent(buy_tax);
        match buy_tax {
            Some(buy_t) if !buy_tax_bucket.is_acceptable_for_trading_enabled() => {
                info!(
                    "⚠️ TradingEnabled skipped for token {} pool {} (tx {}): buy tax {:.1}% classified as {}",
                    token_address,
                    pool_address,
                    tx_hash,
                    buy_t,
                    buy_tax_bucket.key()
                );
                return None;
            }
            None => {
                // Show the specific error if available
                let error_detail = buy_sell
                    .buy_tax_error
                    .as_ref()
                    .map(|e| format!(": {}", e))
                    .unwrap_or_default();
                warn!(
                    "INVALID_SIMULATION_RESULTS: Token {} pool {} (tx {}) - Cannot generate TRADING_ENABLED signal: buy tax calculation failed{}",
                    token_address, pool_address, tx_hash, error_detail
                );
                return None;
            }
            Some(buy_t) => {
                debug!(
                    "Token {} pool {} - Buy tax acceptable: {:.1}% ({})",
                    token_address,
                    pool_address,
                    buy_t,
                    buy_tax_bucket.key()
                );
            }
        }

        let sell_tax_bucket = TaxBucket::from_percent(sell_tax);
        match sell_tax {
            Some(sell_t) if !sell_tax_bucket.is_acceptable_for_trading_enabled() => {
                info!(
                    "⚠️ TradingEnabled skipped for token {} pool {} (tx {}): sell tax {:.1}% classified as {}",
                    token_address,
                    pool_address,
                    tx_hash,
                    sell_t,
                    sell_tax_bucket.key()
                );
                return None;
            }
            None => {
                // Show the specific error if available
                let error_detail = buy_sell
                    .sell_tax_error
                    .as_ref()
                    .map(|e| format!(": {}", e))
                    .unwrap_or_default();
                warn!(
                    "INVALID_SIMULATION_RESULTS: Token {} pool {} (tx {}) - Cannot generate TRADING_ENABLED signal: sell tax calculation failed{}",
                    token_address, pool_address, tx_hash, error_detail
                );
                return None;
            }
            Some(sell_t) => {
                debug!(
                    "Token {} pool {} - Sell tax acceptable: {:.1}% ({})",
                    token_address,
                    pool_address,
                    sell_t,
                    sell_tax_bucket.key()
                );
            }
        }

        // Prevent duplicate TradingEnabled signals during this run
        let pair_key = (token_address.clone(), pool_address.clone());
        {
            let mut emitted = self.emitted_trading_pairs.lock().await;
            if emitted.contains(&pair_key) {
                info!(
                    "ℹ️ TradingEnabled dedupe: token {} pool {} (tx {}) already emitted earlier in this run",
                    token_address, pool_address, tx_hash
                );
                return None;
            }
            emitted.insert(pair_key);
        }

        // All conditions met - generate trading enabled signal!
        info!(
            "🚀 TRADING ENABLED SIGNAL: Token {} Pool {} | Buy: {:.1}% | Sell: {:.1}% | TX: {}",
            token_address,
            pool_address,
            buy_tax.unwrap_or(0.0),
            sell_tax.unwrap_or(0.0),
            tx_hash
        );

        Some(TradingStatusSignal {
            token_address: token_address.to_string(),
            pool_address: pool_address.to_string(),
            pool_type: pool_type.clone(),
            status_change: TradingStatusChange::TradingEnabled,
            executor,
            can_trade_after: true,
            buy_tax,
            sell_tax,
            tx_hash: tx_hash.clone(),
            details: format!(
                "Trading enabled: buy_tax={:.1}%, sell_tax={:.1}%, cache_already_enabled={}",
                buy_tax.unwrap_or(0.0),
                sell_tax.unwrap_or(0.0),
                already_enabled_in_cache
            ),
        })
    }

    /// Check if trading is already enabled for this token/pool combination
    async fn is_trading_already_enabled(&self, token_address: &str, pool_address: &str) -> bool {
        if let Some(ref cache) = self.token_cache {
            // Get all pools for this token
            let pools = cache.get_pools_for_token(&token_address.to_string()).await;

            // Find the specific pool
            if let Some(pool_state) = pools
                .iter()
                .find(|p| p.address.eq_ignore_ascii_case(pool_address))
            {
                // Treat the pool as already enabled when either the explicit
                // flag is set or the current pool snapshot can both buy and
                // sell. Some live snapshots can have the tradable booleans
                // ahead of the legacy trading_enabled flag.
                if pool_state.trading_enabled || (pool_state.can_buy && pool_state.can_sell) {
                    debug!(
                        "Pool {} already has trading_enabled={} can_buy={} can_sell={} in cache",
                        pool_address,
                        pool_state.trading_enabled,
                        pool_state.can_buy,
                        pool_state.can_sell
                    );
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_tracking::{
        types::PoolLifecycle, Pool, PoolType, Token, TokenUpdate, TokenWithPools,
    };
    use std::collections::HashMap;

    const TOKEN: &str = "0x1000000000000000000000000000000000000001";
    const POOL: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CREATOR: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    async fn detector_with_pool(
        trading_enabled: bool,
        can_buy: bool,
        can_sell: bool,
    ) -> TradingStatusDetector {
        let cache = Arc::new(TokenTrackingCache::with_defaults());
        let token = Token {
            address: TOKEN.to_string(),
            symbol: "TEST".to_string(),
            name: "Test".to_string(),
            decimals: 18,
            total_supply: Some("1000000000000000000".to_string()),
            creator_address: CREATOR.to_string(),
            current_owner: CREATOR.to_string(),
            tax_setter_addresses: Vec::new(),
            ownership_renounced: false,
            renouncement_block: None,
            buy_tax: None,
            sell_tax: None,
            last_tax_change_block: None,
            tax_history: Vec::new(),
            creation_block: 1,
            creation_tx: "0xcreate".to_string(),
            creation_timestamp: None,
            latest_activity_block: 1,
            is_scam: false,
            scam_label: None,
            total_liquidity: 0.0,
        };
        let pool = Pool {
            address: POOL.to_string(),
            token_address: TOKEN.to_string(),
            pool_type: PoolType::UniswapV2,
            token_reserve: 1_000_000.0,
            eth_reserve: 1.0,
            denom_currency: "WETH".to_string(),
            denom_address: "0xC02aaa39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            trading_enabled,
            trading_enabled_block: trading_enabled.then_some(1),
            trading_enabled_tx: trading_enabled.then_some("0xenabled".to_string()),
            fee_tier: None,
            pool_id: None,
            lp_token_address: None,
            position_manager_address: None,
            lp_total_supply: None,
            liquidity_positions: Vec::new(),
            last_updated_block: 1,
            last_updated_time: 0.0,
            is_scam: false,
            scam_label: None,
            lp_tokens_approved_percentage: None,
            lifecycle: PoolLifecycle::Active,
            control_addresses: Vec::new(),
            can_buy,
            can_sell,
            received_at: std::time::Instant::now(),
        };
        let mut pools = HashMap::new();
        pools.insert(POOL.to_string(), pool);
        let mut data = HashMap::new();
        data.insert(TOKEN.to_string(), TokenWithPools { token, pools });
        cache
            .batch_update(TokenUpdate {
                message_type: "test".to_string(),
                token_count: 1,
                block_number: 1,
                timestamp: 0.0,
                data,
            })
            .await;

        let mut detector = TradingStatusDetector::new();
        detector.set_token_cache(cache);
        detector
    }

    #[tokio::test]
    async fn already_enabled_when_cache_can_buy_and_can_sell() {
        let detector = detector_with_pool(false, true, true).await;

        assert!(detector.is_trading_already_enabled(TOKEN, POOL).await);
    }

    #[tokio::test]
    async fn already_enabled_pool_match_is_case_insensitive() {
        let detector = detector_with_pool(true, false, false).await;

        assert!(
            detector
                .is_trading_already_enabled(TOKEN, "0xAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAaAa")
                .await
        );
    }

    #[tokio::test]
    async fn not_enabled_when_cache_only_can_buy() {
        let detector = detector_with_pool(false, true, false).await;

        assert!(!detector.is_trading_already_enabled(TOKEN, POOL).await);
    }
}
