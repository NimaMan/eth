/// Trading Status Updater
/// 
/// Updates token trading status based on actual simulation results
/// This fixes the issue where Python doesn't detect all trading enable methods

use super::TokenTrackingCache;
use crate::simulator::SimulationResult;
use std::sync::Arc;
use tracing::{info, debug};

pub struct TradingStatusUpdater {
    token_cache: Arc<TokenTrackingCache>,
}

impl TradingStatusUpdater {
    pub fn new(token_cache: Arc<TokenTrackingCache>) -> Self {
        Self { token_cache }
    }
    
    /// Update trading status based on simulation results
    /// If a token can be bought and sold with reasonable taxes, trading is enabled
    pub async fn update_from_simulation(&self, result: &SimulationResult) {
        // Only process if we have buy/sell results
        let buy_sell = match &result.buy_sell_result {
            Some(bs) => bs,
            None => return,
        };
        
        // Extract token address
        let token_address = match &result.request.category {
            crate::tx_router::TransactionCategory::CreatorTransaction { target_token, .. } => {
                match target_token {
                    Some(token) => token.clone(),
                    None => return,
                }
            }
            crate::tx_router::TransactionCategory::ContractCreation { contract_address, .. } => {
                contract_address.clone()
            }
            _ => return,
        };
        
        // If both buy and sell work, trading is effectively enabled
        if buy_sell.can_buy && buy_sell.can_sell {
            // Get current token info
            if let Some(mut token_info) = self.token_cache.get_token(&token_address).await {
                // Check if we need to update trading status
                if !token_info.trading_enabled {
                    info!("🔄 Updating trading status for {} based on simulation (was: false, now: true)", 
                          token_address);
                    
                    // Update the trading enabled flag
                    token_info.trading_enabled = true;
                    
                    // If we don't have a trading enabled tx, use the current tx
                    if token_info.trading_enabled_txn.is_none() {
                        token_info.trading_enabled_txn = Some(result.request.tx.hash.clone());
                        debug!("  Set trading_enabled_txn to current tx: {}", result.request.tx.hash);
                    }
                    
                    // Update the token in cache
                    self.token_cache.update_token(token_info).await;
                    
                    info!("✅ Trading status updated for {} - trading is ENABLED", token_address);
                } else {
                    debug!("Token {} already has trading_enabled=true", token_address);
                }
            } else {
                debug!("Token {} not found in cache, cannot update trading status", token_address);
            }
        } else if !buy_sell.can_buy || !buy_sell.can_sell {
            // Trading might be disabled or restricted
            if let Some(mut token_info) = self.token_cache.get_token(&token_address).await {
                if token_info.trading_enabled {
                    info!("⚠️ Token {} has trading_enabled=true but simulation shows can_buy={}, can_sell={}", 
                          token_address, buy_sell.can_buy, buy_sell.can_sell);
                    // Don't automatically disable - might be temporary issue
                }
            }
        }
    }
    
    /// Check if trading should be considered enabled based on pool state
    /// If a pool has significant liquidity and no excessive taxes, trading is likely enabled
    pub async fn infer_from_pool_state(&self, token_address: &str) -> bool {
        // Get pools for this token
        let pools = self.token_cache.get_pools_for_token(token_address).await;
        
        // If there's a pool with > 0.1 ETH liquidity, trading is likely enabled
        for (_pool_addr, pool) in pools {
            if pool.eth_reserve > 0.1 {
                debug!("Token {} has pool with {:.4} ETH - inferring trading enabled", 
                       token_address, pool.eth_reserve);
                return true;
            }
        }
        
        false
    }
}