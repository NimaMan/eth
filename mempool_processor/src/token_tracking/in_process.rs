use std::collections::HashMap;
use std::sync::Arc;

use eth_live_feed::{LiveTokenEvent, LiveTokenPoolSnapshot, LiveTokenReader, LiveTokenSnapshot};
use eyre::Result;
use tokio::sync::broadcast;
use tracing::{debug, warn};

use super::cache::TokenTrackingCache;
use super::types::{Address, Pool, PoolLifecycle, PoolType, Token, TokenUpdate, TokenWithPools};

pub async fn hydrate_cache_from_live_reader<R>(
    cache: &TokenTrackingCache,
    reader: &R,
) -> Result<usize>
where
    R: LiveTokenReader + ?Sized,
{
    let progress = reader.progress().await;
    let snapshots = reader.token_snapshots().await;
    let count = snapshots.len();
    apply_live_token_snapshots_to_cache(
        cache,
        progress.current_block.unwrap_or_default(),
        "live_reader_hydrate",
        snapshots,
    )
    .await;
    Ok(count)
}

pub async fn start_live_token_reader_cache_sync<R>(
    cache: Arc<TokenTrackingCache>,
    reader: Arc<R>,
) -> Result<()>
where
    R: LiveTokenReader + 'static,
{
    hydrate_cache_from_live_reader(cache.as_ref(), reader.as_ref()).await?;
    let mut updates = reader.subscribe();

    loop {
        match updates.recv().await {
            Ok(LiveTokenEvent::BlockApplied { block_number, .. }) => {
                let snapshots = reader.token_snapshots().await;
                apply_live_token_snapshots_to_cache(
                    cache.as_ref(),
                    block_number,
                    "live_reader_block_update",
                    snapshots,
                )
                .await;
            }
            Ok(_) => {}
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                warn!(
                    skipped,
                    "live token reader cache sync lagged; refreshing full token cache"
                );
                let progress = reader.progress().await;
                let snapshots = reader.token_snapshots().await;
                apply_live_token_snapshots_to_cache(
                    cache.as_ref(),
                    progress.current_block.unwrap_or_default(),
                    "live_reader_lag_refresh",
                    snapshots,
                )
                .await;
            }
            Err(broadcast::error::RecvError::Closed) => return Ok(()),
        }
    }
}

pub async fn apply_live_token_snapshots_to_cache(
    cache: &TokenTrackingCache,
    block_number: u64,
    message_type: &str,
    snapshots: Vec<LiveTokenSnapshot>,
) {
    let token_map = snapshots
        .into_iter()
        .map(|snapshot| {
            let address = snapshot.contract_address.clone();
            (address, live_snapshot_to_token_with_pools(snapshot))
        })
        .collect::<HashMap<_, _>>();

    if token_map.is_empty() {
        return;
    }

    let update = TokenUpdate {
        message_type: message_type.to_string(),
        token_count: token_map.len(),
        block_number,
        timestamp: 0.0,
        data: token_map,
    };

    let result = cache.batch_update(update).await;
    debug!(
        "Applied in-process live token update @block {} ({} tokens, {} pools)",
        block_number, result.tokens_updated, result.pools_updated
    );
}

fn live_snapshot_to_token_with_pools(snapshot: LiveTokenSnapshot) -> TokenWithPools {
    let total_liquidity = snapshot
        .pools
        .iter()
        .filter(|pool| !pool.is_scam)
        .map(|pool| pool.denom_reserve)
        .sum();

    let token = Token {
        address: snapshot.contract_address.clone(),
        symbol: snapshot.symbol,
        name: snapshot.name,
        decimals: snapshot.decimals,
        total_supply: Some(snapshot.total_supply),
        creator_address: snapshot.creator_address.clone().unwrap_or_default(),
        current_owner: snapshot
            .current_owner
            .or(snapshot.creator_address)
            .unwrap_or_default(),
        tax_setter_addresses: snapshot.tax_setter_addresses,
        ownership_renounced: snapshot.ownership_renounced,
        renouncement_block: None,
        buy_tax: snapshot.buy_tax,
        sell_tax: snapshot.sell_tax,
        last_tax_change_block: None,
        tax_history: Vec::new(),
        creation_block: snapshot.creation_block.unwrap_or_default(),
        creation_tx: snapshot.creation_tx.unwrap_or_default(),
        creation_timestamp: snapshot.creation_timestamp.map(|value| value as f64),
        latest_activity_block: snapshot.latest_activity_block.unwrap_or_default(),
        is_scam: snapshot.is_scam,
        scam_label: snapshot.scam_label,
        total_liquidity,
    };

    let pools = snapshot
        .pools
        .into_iter()
        .map(|pool| (pool.pool_address.clone(), live_pool_to_cache_pool(pool)))
        .collect::<HashMap<Address, Pool>>();

    TokenWithPools { token, pools }
}

fn live_pool_to_cache_pool(pool: LiveTokenPoolSnapshot) -> Pool {
    Pool {
        address: pool.pool_address,
        token_address: pool.token_address,
        pool_type: parse_pool_type(&pool.protocol),
        token_reserve: pool.token_reserve,
        eth_reserve: pool.denom_reserve,
        denom_currency: pool.denom_symbol,
        denom_address: pool.denom_address,
        trading_enabled: pool.trading_enabled,
        trading_enabled_block: pool.trading_enabled_block,
        trading_enabled_tx: pool.trading_enabled_tx,
        fee_tier: None,
        pool_id: None,
        last_updated_block: pool.latest_block_number.unwrap_or_default(),
        last_updated_time: 0.0,
        is_scam: pool.is_scam,
        scam_label: pool.scam_label,
        lp_tokens_approved_percentage: pool.lp_tokens_approved_percentage,
        lifecycle: parse_pool_lifecycle(&pool.lifecycle),
        control_addresses: pool.control_addresses,
        can_buy: pool.can_buy,
        can_sell: pool.can_sell,
        received_at: std::time::Instant::now(),
    }
}

fn parse_pool_type(value: &str) -> PoolType {
    match value.to_ascii_uppercase().as_str() {
        "UNISWAP-V2" | "V2" => PoolType::UniswapV2,
        "UNISWAP-V3" | "V3" => PoolType::UniswapV3,
        "UNISWAP-V4" | "V4" => PoolType::UniswapV4,
        _ => PoolType::Unknown,
    }
}

fn parse_pool_lifecycle(value: &str) -> PoolLifecycle {
    match value.to_ascii_uppercase().as_str() {
        "SEEDED" | "LIQUIDITY_DEPOSITED" => PoolLifecycle::LiquidityDeposited,
        "ACTIVE" => PoolLifecycle::Active,
        "SCAM" => PoolLifecycle::Scam,
        "EVICTED" => PoolLifecycle::Evicted,
        "DISCOVERED" => PoolLifecycle::Discovered,
        _ => PoolLifecycle::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use eth_live_feed::{LiveTokenPoolSnapshot, LiveTokenSnapshot};

    use super::super::cache::TokenTrackingCache;
    use super::apply_live_token_snapshots_to_cache;

    #[tokio::test]
    async fn applies_live_token_snapshot_to_cache() {
        let cache = TokenTrackingCache::with_defaults();
        let token_address = "0x0000000000000000000000000000000000000001".to_string();
        let pool_address = "0x0000000000000000000000000000000000000002".to_string();

        let snapshot = LiveTokenSnapshot {
            contract_address: token_address.clone(),
            name: "Token".to_string(),
            symbol: "TKN".to_string(),
            decimals: 18,
            total_supply: "1000000000000000000".to_string(),
            creation_block: Some(100),
            creation_timestamp: Some(1_700),
            creation_tx: Some("0xcreation".to_string()),
            creator_address: Some("0x0000000000000000000000000000000000000003".to_string()),
            current_owner: None,
            tax_setter_addresses: Vec::new(),
            ownership_renounced: false,
            latest_activity_block: Some(120),
            latest_activity_timestamp: Some(1_720),
            is_scam: false,
            scam_label: None,
            buy_tax: Some(1.0),
            sell_tax: Some(2.0),
            pools: vec![LiveTokenPoolSnapshot {
                pool_address: pool_address.clone(),
                token_address: token_address.clone(),
                protocol: "UNISWAP-V2".to_string(),
                denom_address: "0x0000000000000000000000000000000000000004".to_string(),
                denom_symbol: "WETH".to_string(),
                token_reserve: 10.0,
                denom_reserve: 1.0,
                price: 0.1,
                total_liquidity: 1.0,
                can_buy: true,
                can_sell: true,
                trading_enabled: true,
                trading_enabled_block: Some(120),
                trading_enabled_tx: Some("0xtrade".to_string()),
                buy_tax: Some(1.0),
                sell_tax: Some(2.0),
                is_scam: false,
                scam_label: None,
                creation_block: Some(110),
                latest_block_number: Some(120),
                lifecycle: "ACTIVE".to_string(),
                control_addresses: Vec::new(),
                lp_tokens_approved_percentage: Some(0.0),
            }],
        };

        apply_live_token_snapshots_to_cache(&cache, 120, "test", vec![snapshot]).await;

        let stats = cache.stats().await;
        assert_eq!(stats.total_tokens, 1);
        assert_eq!(stats.total_pools, 1);
        assert!(cache.get_token(&token_address).await.is_some());
        assert!(cache.get_pool(&pool_address).await.is_some());
    }
}
