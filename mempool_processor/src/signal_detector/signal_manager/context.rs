use std::sync::Arc;

use crate::token_tracking::TokenTrackingCache;

use super::formatting::known_denom_decimals;

#[derive(Clone)]
pub(super) struct SignalPoolContext {
    pub(super) denom_address: String,
    pub(super) denom_currency: String,
    pub(super) denom_decimals: Option<u8>,
    pub(super) token_decimals: Option<u8>,
    pub(super) denom_reserve: f64,
    pub(super) token_reserve: f64,
    pub(super) pool_creation_block: Option<u64>,
    pub(super) latest_block: u64,
    pub(super) can_buy: bool,
    pub(super) can_sell: bool,
    pub(super) is_scam: bool,
}

pub(super) async fn pool_context_for_address(
    token_cache: &Option<Arc<TokenTrackingCache>>,
    pool_identifier: Option<&str>,
) -> Option<SignalPoolContext> {
    let cache = token_cache.as_ref()?;
    let pool_identifier = pool_identifier?;
    let pool = cache.get_pool_by_address(pool_identifier).await?;
    let token_decimals = cache
        .get_token(&pool.token_address.to_string())
        .await
        .map(|token| token.decimals);
    Some(SignalPoolContext {
        denom_address: pool.denom_address.clone(),
        denom_currency: pool.denom_currency.clone(),
        denom_decimals: known_denom_decimals(&pool.denom_currency, &pool.denom_address),
        token_decimals,
        denom_reserve: pool.eth_reserve,
        token_reserve: pool.token_reserve,
        pool_creation_block: pool.trading_enabled_block,
        latest_block: pool.last_updated_block,
        can_buy: pool.can_buy,
        can_sell: pool.can_sell,
        is_scam: pool.is_scam,
    })
}
