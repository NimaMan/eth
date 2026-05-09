use std::time::{Duration, Instant};

use eyre::Result;

use crate::chain_metadata::{
    UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};
use crate::tracking::{address_string, hash_string};

use super::LIVE_TOKEN_TRACKER_LOG_TARGET;

pub(super) async fn optional_uniswap_v2_pool_metadata<P>(
    pool_metadata_provider: &P,
    lookup: UniswapV2PoolMetadataLookup,
    metadata_timeout: Option<Duration>,
) -> Result<Option<UniswapV2PoolMetadata>>
where
    P: UniswapV2PoolMetadataProvider,
{
    let started = Instant::now();
    tracing::debug!(
        target: LIVE_TOKEN_TRACKER_LOG_TARGET,
        block_number = lookup.block_number,
        tx_index = lookup.tx_index,
        tx_hash = %hash_string(&lookup.transaction_hash),
        pool_address = %address_string(&lookup.pool_address),
        action = "uniswap_v2_pool_metadata_lookup",
        result = "started",
        "live uniswap v2 pool metadata lookup started"
    );
    let metadata_result = if let Some(timeout) = metadata_timeout {
        match tokio::time::timeout(
            timeout,
            pool_metadata_provider.uniswap_v2_pool_metadata(&lookup),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(
                    target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                    block_number = lookup.block_number,
                    tx_index = lookup.tx_index,
                    tx_hash = %hash_string(&lookup.transaction_hash),
                    pool_address = %address_string(&lookup.pool_address),
                    timeout_ms = timeout.as_millis(),
                    action = "uniswap_v2_pool_metadata_lookup",
                    result = "timeout",
                    "live uniswap v2 pool metadata lookup timed out"
                );
                return Ok(None);
            }
        }
    } else {
        pool_metadata_provider
            .uniswap_v2_pool_metadata(&lookup)
            .await
    };

    match metadata_result {
        Ok(metadata) => {
            tracing::debug!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                block_number = lookup.block_number,
                tx_index = lookup.tx_index,
                tx_hash = %hash_string(&lookup.transaction_hash),
                pool_address = %address_string(&lookup.pool_address),
                elapsed_ms = started.elapsed().as_millis(),
                found = metadata.is_some(),
                action = "uniswap_v2_pool_metadata_lookup",
                result = "ok",
                "live uniswap v2 pool metadata lookup completed"
            );
            Ok(metadata)
        }
        Err(error) if is_not_uniswap_v2_pool_metadata_miss(&error.to_string()) => {
            tracing::debug!(
                target: LIVE_TOKEN_TRACKER_LOG_TARGET,
                block_number = lookup.block_number,
                tx_index = lookup.tx_index,
                tx_hash = %hash_string(&lookup.transaction_hash),
                pool_address = %address_string(&lookup.pool_address),
                elapsed_ms = started.elapsed().as_millis(),
                action = "uniswap_v2_pool_metadata_lookup",
                result = "miss",
                reason = %error,
                "live uniswap v2 pool metadata lookup missed"
            );
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub(super) fn is_not_uniswap_v2_pool_metadata_miss(message: &str) -> bool {
    message.contains("token0() view call failed")
        || message.contains("token1() view call failed")
        || message.contains("Failed to get token decimals")
        || message.contains("Token decimals call")
        || message.contains("missing live block header")
}
