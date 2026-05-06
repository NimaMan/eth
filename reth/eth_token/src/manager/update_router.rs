use std::collections::BTreeSet;

use alloy_primitives::Address;
use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::ProcessedTransaction;

use crate::erc20::ERC20Token;
use crate::pools::BasePoolConfig;

use super::{
    address_string, hash_string, normalize_address, normalize_address_string, parse_address_lossy,
    same_address_str, TokenRegistry, TokenStateUpdateReport, TrackedTokenIndex,
    UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessedTokenUpdateRouter {
    pub history_limit: usize,
    pub known_routers: Vec<String>,
}

impl ProcessedTokenUpdateRouter {
    pub fn new(history_limit: usize) -> Self {
        Self {
            history_limit,
            known_routers: Vec::new(),
        }
    }

    pub fn with_known_routers(
        history_limit: usize,
        routers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            history_limit,
            known_routers: routers
                .into_iter()
                .map(|router| normalize_address_string(router))
                .collect(),
        }
    }

    pub fn update_registry_from_processed_transaction(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
    ) -> Result<Vec<TokenStateUpdateReport>> {
        let token_addresses = candidate_token_addresses(registry, token_index, tx);
        let mut reports = Vec::new();

        for token_address in token_addresses {
            let Some(token) = registry.token_mut(&token_address) else {
                continue;
            };

            let token_state_updated = touches_token_state(tx, &token_address);
            if token_state_updated {
                token.update_token_state_from_processed_transaction(tx)?;
            }

            let discovered = self.discover_uniswap_v2_pools_for_token(token, tx);
            let updated = update_touched_v2_pools(token, tx)?;

            if token_state_updated || !discovered.is_empty() || !updated.is_empty() {
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered,
                    updated_uniswap_v2_pools: updated,
                });
            }
        }

        Ok(reports)
    }

    pub async fn update_registry_from_processed_transaction_with_discovery<P>(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        let token_addresses = candidate_token_addresses_with_pool_discovery(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
        )
        .await?;
        let mut reports = Vec::new();

        for token_address in token_addresses {
            let Some(token) = registry.token_mut(&token_address) else {
                continue;
            };

            let token_state_updated = touches_token_state(tx, &token_address);
            if token_state_updated {
                token.update_token_state_from_processed_transaction(tx)?;
            }

            let mut discovered = self
                .discover_uniswap_v2_pools_for_token_with_metadata(
                    token,
                    tx,
                    pool_metadata_provider,
                )
                .await?;
            discovered.extend(
                self.discover_uniswap_v2_pools_from_swaps_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                )
                .await?,
            );

            let updated = update_touched_v2_pools(token, tx)?;

            if token_state_updated || !discovered.is_empty() || !updated.is_empty() {
                discovered.sort();
                discovered.dedup();
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered,
                    updated_uniswap_v2_pools: updated,
                });
            }
        }

        Ok(reports)
    }

    fn discover_uniswap_v2_pools_for_token(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
    ) -> Vec<String> {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v2_pair_created_events {
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

    async fn discover_uniswap_v2_pools_for_token_with_metadata<P>(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
    ) -> Result<Vec<String>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v2_pair_created_events {
            let token_is_token0 = same_address_str(event.token0, &token_address);
            let token_is_token1 = same_address_str(event.token1, &token_address);
            if !token_is_token0 && !token_is_token1 {
                continue;
            }

            let pool_address = address_string(&event.pair_address);
            if token.uniswap_v2_pool(&pool_address).is_some() {
                continue;
            }

            let metadata = optional_uniswap_v2_pool_metadata(
                pool_metadata_provider,
                UniswapV2PoolMetadataLookup {
                    tracked_token_address: Some(parse_address_lossy(&token_address)),
                    pool_address: event.pair_address,
                    block_number: tx.block_number,
                    transaction_hash: tx.hash,
                    tx_index: tx.tx_index,
                },
            )
            .await?;

            let (denom_address, config) = metadata
                .as_ref()
                .and_then(|metadata| self.v2_pool_config_from_metadata(token, metadata))
                .unwrap_or_else(|| {
                    let denom_address = if token_is_token0 {
                        event.token1
                    } else {
                        event.token0
                    };
                    (
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
                    )
                });

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

    async fn discover_uniswap_v2_pools_from_swaps_for_token<P>(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
    ) -> Result<Vec<String>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v2_swaps {
            let pool_address = address_string(&event.pair_address);
            if token.uniswap_v2_pool(&pool_address).is_some()
                || discovered.iter().any(|known| known == &pool_address)
            {
                continue;
            }

            let Some(metadata) = optional_uniswap_v2_pool_metadata(
                pool_metadata_provider,
                UniswapV2PoolMetadataLookup {
                    tracked_token_address: Some(parse_address_lossy(&token_address)),
                    pool_address: event.pair_address,
                    block_number: tx.block_number,
                    transaction_hash: tx.hash,
                    tx_index: tx.tx_index,
                },
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

fn candidate_token_addresses(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
) -> Vec<String> {
    let mut candidates = BTreeSet::new();
    for address in routing_addresses(tx) {
        insert_resolved_token_address(registry, token_index, &mut candidates, address);
    }
    candidates.into_iter().collect()
}

async fn candidate_token_addresses_with_pool_discovery<P>(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    tx: &ProcessedTransaction,
    pool_metadata_provider: &P,
) -> Result<Vec<String>>
where
    P: UniswapV2PoolMetadataProvider,
{
    let mut candidates = BTreeSet::new();
    for address in routing_addresses(tx) {
        insert_resolved_token_address(registry, token_index, &mut candidates, address);
    }

    for pool_address in v2_pool_event_addresses(tx) {
        let pool_address_string = address_string(&pool_address);
        if token_index
            .resolve_token_address(&pool_address_string)
            .is_some()
        {
            continue;
        }

        let Some(metadata) = optional_uniswap_v2_pool_metadata(
            pool_metadata_provider,
            UniswapV2PoolMetadataLookup {
                tracked_token_address: None,
                pool_address,
                block_number: tx.block_number,
                transaction_hash: tx.hash,
                tx_index: tx.tx_index,
            },
        )
        .await?
        else {
            continue;
        };

        insert_resolved_token_address_str(registry, token_index, &mut candidates, &metadata.token0);
        insert_resolved_token_address_str(registry, token_index, &mut candidates, &metadata.token1);
    }

    Ok(candidates.into_iter().collect())
}

async fn optional_uniswap_v2_pool_metadata<P>(
    pool_metadata_provider: &P,
    lookup: UniswapV2PoolMetadataLookup,
) -> Result<Option<UniswapV2PoolMetadata>>
where
    P: UniswapV2PoolMetadataProvider,
{
    match pool_metadata_provider
        .uniswap_v2_pool_metadata(&lookup)
        .await
    {
        Ok(metadata) => Ok(metadata),
        Err(error) if is_not_uniswap_v2_pool_metadata_miss(&error.to_string()) => Ok(None),
        Err(error) => Err(error),
    }
}

fn is_not_uniswap_v2_pool_metadata_miss(message: &str) -> bool {
    message.contains("token0() view call failed") || message.contains("token1() view call failed")
}

fn routing_addresses(tx: &ProcessedTransaction) -> BTreeSet<Address> {
    let mut addresses = BTreeSet::new();

    addresses.extend(tx.unique_addresses.iter().copied());
    addresses.extend(tx.erc20_contracts.iter().copied());
    if let Some(address) = tx.contract_address {
        addresses.insert(address);
    }

    for transfer in &tx.erc20_transfers {
        addresses.insert(transfer.token_address);
    }
    for approval in &tx.erc20_approval_events {
        addresses.insert(approval.token_address);
    }
    for event in &tx.trading_enabled_events {
        addresses.insert(event.token_address);
    }
    for event in &tx.trading_disabled_events {
        addresses.insert(event.token_address);
    }
    for event in &tx.contract_creation_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.ownership_transferred_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.ownership_transfer_started_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.access_control_role_granted_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.access_control_role_revoked_events {
        addresses.insert(event.contract_address);
    }
    for event in &tx.proxy_admin_changed_events {
        addresses.insert(event.contract_address);
    }

    for event in &tx.uniswap_v2_pair_created_events {
        addresses.insert(event.pair_address);
        addresses.insert(event.token0);
        addresses.insert(event.token1);
    }
    addresses.extend(v2_pool_event_addresses(tx));

    addresses
}

fn v2_pool_event_addresses(tx: &ProcessedTransaction) -> BTreeSet<Address> {
    let mut addresses = BTreeSet::new();
    for event in &tx.uniswap_v2_syncs {
        addresses.insert(event.pair_address);
    }
    for event in &tx.uniswap_v2_swaps {
        addresses.insert(event.pair_address);
    }
    for event in &tx.uniswap_v2_mints {
        addresses.insert(event.pair_address);
    }
    for event in &tx.uniswap_v2_burns {
        addresses.insert(event.pair_address);
    }
    addresses
}

fn insert_resolved_token_address(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    candidates: &mut BTreeSet<String>,
    address: Address,
) {
    insert_resolved_token_address_str(registry, token_index, candidates, &address_string(&address));
}

fn insert_resolved_token_address_str(
    registry: &TokenRegistry,
    token_index: &TrackedTokenIndex,
    candidates: &mut BTreeSet<String>,
    address: &str,
) {
    if let Some(token_address) = token_index.resolve_token_address(address) {
        if registry.token(token_address).is_some() {
            candidates.insert(token_address.to_string());
        }
        return;
    }

    let address = normalize_address(address);
    if registry.token(&address).is_some() {
        candidates.insert(address);
    }
}

fn update_touched_v2_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
) -> Result<Vec<String>> {
    let pool_addresses = token.pool_addresses();
    let mut updated = Vec::new();
    for pool_address in pool_addresses {
        if touches_v2_pool(tx, &pool_address) {
            token.update_uniswap_v2_pool_from_processed_transaction(&pool_address, tx)?;
            updated.push(pool_address);
        }
    }
    Ok(updated)
}

fn touches_token_state(tx: &ProcessedTransaction, token_address: &str) -> bool {
    tx.erc20_contracts
        .iter()
        .any(|address| same_address_str(*address, token_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .ownership_transferred_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .ownership_transfer_started_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .access_control_role_granted_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .access_control_role_revoked_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .proxy_admin_changed_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
        || tx
            .trading_enabled_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .trading_disabled_events
            .iter()
            .any(|event| same_address_str(event.token_address, token_address))
        || tx
            .contract_address
            .is_some_and(|address| same_address_str(address, token_address))
        || tx
            .contract_creation_events
            .iter()
            .any(|event| same_address_str(event.contract_address, token_address))
}

fn touches_v2_pool(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v2_syncs
        .iter()
        .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_swaps
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_mints
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .uniswap_v2_burns
            .iter()
            .any(|event| same_address_str(event.pair_address, pool_address))
        || tx
            .erc20_transfers
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
        || tx
            .erc20_approval_events
            .iter()
            .any(|event| same_address_str(event.token_address, pool_address))
}
