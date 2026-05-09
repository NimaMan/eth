use std::collections::BTreeSet;
use std::time::Duration;

use eyre::Result;

use crate::chain_metadata::{
    UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};
use crate::erc20::ERC20Token;
use crate::pools::{BasePoolConfig, SUSHISWAP_V2_FACTORY};
use tx_processor::ProcessedTransaction;

use super::super::known_tokens::known_decimals_for_address;
use super::super::metadata::optional_uniswap_v2_pool_metadata;
use super::super::{TokenTransactionApplier, UNISWAP_V2_FACTORY};
use crate::tracking::{
    address_string, hash_string, normalize_address, parse_address_lossy, same_address_str,
};

impl TokenTransactionApplier {
    pub(crate) fn discover_uniswap_v2_pools_for_token(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
    ) -> Vec<String> {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v2_pair_created_events {
            if !is_uniswap_v2_pair_created_event(event) {
                continue;
            }
            let token_is_token0 = same_address_str(event.token0, &token_address);
            let token_is_token1 = same_address_str(event.token1, &token_address);
            if !token_is_token0 && !token_is_token1 {
                continue;
            }

            let pool_address = address_string(&event.pair_address);
            if token.uniswap_v2_pool(&pool_address).is_some() {
                continue;
            }

            let denom_address = if token_is_token0 {
                event.token1
            } else {
                event.token0
            };
            let pool = token.create_uniswap_v2_pool(
                pool_address.clone(),
                address_string(&denom_address),
                BasePoolConfig {
                    token_decimals: token.decimals,
                    denom_decimals: None,
                    token1_is_denom: Some(token_is_token0),
                    history_limit: self.history_limit,
                    denom_threshold: 0.0,
                    threshold_unit: None,
                    test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
                },
                &self.known_routers,
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            discovered.push(pool_address);
        }

        discovered
    }

    pub(crate) fn discover_sushiswap_v2_pools_for_token(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
    ) -> Vec<String> {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v2_pair_created_events {
            if !is_sushiswap_v2_pair_created_event(event) {
                continue;
            }
            let token_is_token0 = same_address_str(event.token0, &token_address);
            let token_is_token1 = same_address_str(event.token1, &token_address);
            if !token_is_token0 && !token_is_token1 {
                continue;
            }

            let pool_address = address_string(&event.pair_address);
            if token.uniswap_v2_pool(&pool_address).is_some() {
                continue;
            }

            let denom_address = if token_is_token0 {
                event.token1
            } else {
                event.token0
            };
            let pool = token.create_sushiswap_v2_pool(
                pool_address.clone(),
                address_string(&denom_address),
                BasePoolConfig {
                    token_decimals: token.decimals,
                    denom_decimals: None,
                    token1_is_denom: Some(token_is_token0),
                    history_limit: self.history_limit,
                    denom_threshold: 0.0,
                    threshold_unit: None,
                    test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
                },
                &self.known_routers,
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            discovered.push(pool_address);
        }

        discovered
    }

    pub(crate) async fn discover_uniswap_v2_pools_for_token_with_metadata<P>(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
        _pool_metadata_provider: &P,
        _metadata_timeout: Option<Duration>,
    ) -> Result<Vec<String>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v2_pair_created_events {
            if !is_uniswap_v2_pair_created_event(event) {
                continue;
            }
            let token_is_token0 = same_address_str(event.token0, &token_address);
            let token_is_token1 = same_address_str(event.token1, &token_address);
            if !token_is_token0 && !token_is_token1 {
                continue;
            }

            let pool_address = address_string(&event.pair_address);
            if token.uniswap_v2_pool(&pool_address).is_some() {
                continue;
            }

            let denom_address = if token_is_token0 {
                event.token1
            } else {
                event.token0
            };
            let denom_address = address_string(&denom_address);
            let config = BasePoolConfig {
                token_decimals: token.decimals,
                denom_decimals: known_decimals_for_address(if token_is_token0 {
                    event.token1
                } else {
                    event.token0
                }),
                token1_is_denom: Some(token_is_token0),
                history_limit: self.history_limit,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
            };

            let pool = token.create_uniswap_v2_pool(
                pool_address.clone(),
                denom_address,
                config,
                &self.known_routers,
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            discovered.push(pool_address);
        }

        Ok(discovered)
    }

    pub(crate) async fn discover_uniswap_v2_pools_from_events_for_token<P>(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
        metadata_timeout: Option<Duration>,
    ) -> Result<Vec<String>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for pool_address in v2_pool_addresses_from_events(tx) {
            if token.uniswap_v2_pool(&pool_address).is_some()
                || discovered.iter().any(|known| known == &pool_address)
            {
                continue;
            }

            let Some(metadata) = optional_uniswap_v2_pool_metadata(
                pool_metadata_provider,
                UniswapV2PoolMetadataLookup {
                    tracked_token_address: Some(parse_address_lossy(&token_address)),
                    pool_address: parse_address_lossy(&pool_address),
                    block_number: tx.block_number,
                    transaction_hash: tx.hash,
                    tx_index: tx.tx_index,
                },
                metadata_timeout,
            )
            .await?
            else {
                continue;
            };

            let Some((denom_address, config)) = self.v2_pool_config_from_metadata(token, &metadata)
            else {
                continue;
            };

            let pool = token.create_uniswap_v2_pool(
                pool_address.clone(),
                denom_address,
                config,
                &self.known_routers,
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            discovered.push(pool_address);
        }

        Ok(discovered)
    }

    fn v2_pool_config_from_metadata(
        &self,
        token: &ERC20Token,
        metadata: &UniswapV2PoolMetadata,
    ) -> Option<(String, BasePoolConfig)> {
        let token_address = normalize_address(&token.contract_address);
        let token_is_token0 = normalize_address(&metadata.token0) == token_address;
        let token_is_token1 = normalize_address(&metadata.token1) == token_address;
        if !token_is_token0 && !token_is_token1 {
            return None;
        }

        let (denom_address, denom_decimals, token1_is_denom) = if token_is_token0 {
            (metadata.token1.clone(), metadata.token1_decimals, true)
        } else {
            (metadata.token0.clone(), metadata.token0_decimals, false)
        };

        Some((
            denom_address,
            BasePoolConfig {
                token_decimals: token.decimals,
                denom_decimals: Some(denom_decimals),
                token1_is_denom: Some(token1_is_denom),
                history_limit: self.history_limit,
                denom_threshold: 0.0,
                threshold_unit: None,
                test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
            },
        ))
    }
}

fn is_uniswap_v2_pair_created_event(
    event: &tx_processor::tx_processor::data_models::UniswapV2PairCreatedEvent,
) -> bool {
    event.factory_address.is_zero() || same_address_str(event.factory_address, UNISWAP_V2_FACTORY)
}

fn is_sushiswap_v2_pair_created_event(
    event: &tx_processor::tx_processor::data_models::UniswapV2PairCreatedEvent,
) -> bool {
    same_address_str(event.factory_address, SUSHISWAP_V2_FACTORY)
}

fn v2_pool_addresses_from_events(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut addresses = BTreeSet::new();
    for event in &tx.uniswap_v2_syncs {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_swaps {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_mints {
        addresses.insert(address_string(&event.pair_address));
    }
    for event in &tx.uniswap_v2_burns {
        addresses.insert(address_string(&event.pair_address));
    }
    addresses
}
