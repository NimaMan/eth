use std::collections::BTreeSet;
use std::time::Duration;

use alloy_primitives::Address;
use eyre::{eyre, Result};
use reth_chain_query::provider::BlockHeader;
use serde::{Deserialize, Serialize};
use tx_processor::{LivePoolBuySellSimulator, PoolBuySellSimulator, ProcessedTransaction};

use crate::chain_metadata::{
    UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};
use crate::erc20::ERC20Token;
use crate::pools::uniswap::{UniswapV2TradingSimulationConfig, UniswapV2TxContext};
use crate::pools::BasePoolConfig;
use crate::pools::UniswapV2Pool;

use super::replay_context::triggers::tx_is_token_control_replay_candidate;
use super::{
    address_string, hash_string, normalize_address, normalize_address_string, parse_address_lossy,
    same_address_str, TokenRegistry, TokenStateUpdateReport, TrackedTokenIndex,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessedTokenUpdateRouter {
    pub history_limit: usize,
    pub known_routers: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum V2TradingSimulation<'a> {
    Historical(&'a PoolBuySellSimulator),
    Live(&'a LivePoolBuySellSimulator),
    #[cfg(test)]
    Noop,
}

impl V2TradingSimulation<'_> {
    fn is_live(self) -> bool {
        matches!(self, Self::Live(_))
    }
}

const LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS: u64 = 2_500;
const LIVE_POOL_SIMULATION_TIMEOUT_MS: u64 = 2_500;

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
                    simulated_uniswap_v2_pools: Vec::new(),
                });
            }
        }

        Ok(reports)
    }

    pub async fn update_registry_from_processed_transaction_with_pool_simulator(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        pool_simulator: &PoolBuySellSimulator,
        prior_txs: &[ProcessedTransaction],
    ) -> Result<Vec<TokenStateUpdateReport>> {
        self.update_registry_from_processed_transaction_with_trading_simulation(
            registry,
            token_index,
            tx,
            V2TradingSimulation::Historical(pool_simulator),
            prior_txs,
            None,
        )
        .await
    }

    pub async fn update_registry_from_processed_transaction_with_live_pool_simulator(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        pool_simulator: &LivePoolBuySellSimulator,
        prior_txs: &[ProcessedTransaction],
    ) -> Result<Vec<TokenStateUpdateReport>> {
        self.update_registry_from_processed_transaction_with_trading_simulation(
            registry,
            token_index,
            tx,
            V2TradingSimulation::Live(pool_simulator),
            prior_txs,
            None,
        )
        .await
    }

    pub(crate) async fn update_registry_from_processed_transaction_with_trading_simulation(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        trading_simulation: V2TradingSimulation<'_>,
        prior_txs: &[ProcessedTransaction],
        block_header: Option<&BlockHeader>,
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
            let token_control_replay =
                token_state_updated && tx_is_token_control_replay_candidate(token, tx);
            let simulation_pool_addresses =
                simulation_pool_addresses(token, &updated, token_control_replay);
            let current_block_pool_addresses = current_block_simulation_pool_addresses(
                &simulation_pool_addresses,
                &updated,
                &discovered,
                token_control_replay,
            );
            let simulation_prior_txs = simulation_prior_txs(prior_txs, tx, token_control_replay);
            let simulated = simulate_updated_v2_pools(
                token,
                tx,
                &simulation_pool_addresses,
                &current_block_pool_addresses,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;

            if token_state_updated || !discovered.is_empty() || !updated.is_empty() {
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered,
                    updated_uniswap_v2_pools: updated,
                    simulated_uniswap_v2_pools: simulated,
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
            None,
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
                    None,
                )
                .await?;
            discovered.extend(
                self.discover_uniswap_v2_pools_from_swaps_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                    None,
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
                    simulated_uniswap_v2_pools: Vec::new(),
                });
            }
        }

        Ok(reports)
    }

    pub async fn update_registry_from_processed_transaction_with_discovery_and_pool_simulator<P>(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
        pool_simulator: &PoolBuySellSimulator,
        prior_txs: &[ProcessedTransaction],
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        self.update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            V2TradingSimulation::Historical(pool_simulator),
            prior_txs,
            None,
        )
        .await
    }

    pub async fn update_registry_from_processed_transaction_with_discovery_and_live_pool_simulator<
        P,
    >(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
        pool_simulator: &LivePoolBuySellSimulator,
        prior_txs: &[ProcessedTransaction],
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        self.update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            V2TradingSimulation::Live(pool_simulator),
            prior_txs,
            None,
        )
        .await
    }

    pub(crate) async fn update_registry_from_processed_transaction_with_discovery_and_trading_simulation<
        P,
    >(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        pool_metadata_provider: &P,
        trading_simulation: V2TradingSimulation<'_>,
        prior_txs: &[ProcessedTransaction],
        block_header: Option<&BlockHeader>,
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        let pool_metadata_timeout = if trading_simulation.is_live() {
            Some(Duration::from_millis(LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS))
        } else {
            None
        };
        let token_addresses = candidate_token_addresses_with_pool_discovery(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            pool_metadata_timeout,
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
                    pool_metadata_timeout,
                )
                .await?;
            discovered.extend(
                self.discover_uniswap_v2_pools_from_swaps_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                    pool_metadata_timeout,
                )
                .await?,
            );

            let updated = update_touched_v2_pools(token, tx)?;
            let token_control_replay =
                token_state_updated && tx_is_token_control_replay_candidate(token, tx);
            let simulation_pool_addresses =
                simulation_pool_addresses(token, &updated, token_control_replay);
            let current_block_pool_addresses = current_block_simulation_pool_addresses(
                &simulation_pool_addresses,
                &updated,
                &discovered,
                token_control_replay,
            );
            let simulation_prior_txs = simulation_prior_txs(prior_txs, tx, token_control_replay);
            let simulated = simulate_updated_v2_pools(
                token,
                tx,
                &simulation_pool_addresses,
                &current_block_pool_addresses,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;

            if token_state_updated || !discovered.is_empty() || !updated.is_empty() {
                discovered.sort();
                discovered.dedup();
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered,
                    updated_uniswap_v2_pools: updated,
                    simulated_uniswap_v2_pools: simulated,
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
        metadata_timeout: Option<Duration>,
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
                metadata_timeout,
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
        metadata_timeout: Option<Duration>,
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
    metadata_timeout: Option<Duration>,
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
            metadata_timeout,
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
    metadata_timeout: Option<Duration>,
) -> Result<Option<UniswapV2PoolMetadata>>
where
    P: UniswapV2PoolMetadataProvider,
{
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
        Ok(metadata) => Ok(metadata),
        Err(error) if is_not_uniswap_v2_pool_metadata_miss(&error.to_string()) => Ok(None),
        Err(error) => Err(error),
    }
}

fn is_not_uniswap_v2_pool_metadata_miss(message: &str) -> bool {
    message.contains("token0() view call failed")
        || message.contains("token1() view call failed")
        || message.contains("missing live block header")
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

async fn simulate_updated_v2_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    pool_addresses: &[String],
    current_block_pool_addresses: &[String],
    prior_txs: &[ProcessedTransaction],
    trading_simulation: V2TradingSimulation<'_>,
    force_simulation: bool,
    block_header: Option<&BlockHeader>,
) -> Result<Vec<String>> {
    if pool_addresses.is_empty() {
        return Ok(Vec::new());
    }

    let token_address = token.contract_address.clone();
    let tx_context = UniswapV2TxContext {
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_hash: hash_string(&tx.hash),
        from_address: Some(address_string(&tx.from_address)),
    };
    let config = UniswapV2TradingSimulationConfig {
        prior_txs: prior_txs.to_vec(),
        ..Default::default()
    };

    let mut simulated = Vec::new();
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v2_pool(pool_address)
            .map(|pool| {
                !pool.base.is_scam() && (force_simulation || should_simulate_v2_trading(pool, tx))
            })
            .unwrap_or(false);
        if !should_simulate {
            continue;
        }

        let Some(pool) = token.uniswap_v2_pool_mut(pool_address) else {
            continue;
        };

        let pool_config = if should_simulate_at_current_block(
            pool_address,
            current_block_pool_addresses,
            trading_simulation,
        ) {
            UniswapV2TradingSimulationConfig {
                block_number: Some(tx.block_number),
                block_header: block_header.cloned(),
                prior_txs: Vec::new(),
                ..config.clone()
            }
        } else {
            config.clone()
        };
        let prior_tx_count = pool_config.prior_txs.len();
        let tx_hash = hash_string(&tx.hash);
        tracing::debug!(
            target: "pool_buy_sell_sim",
            block_number = tx.block_number,
            token_address = %token_address,
            pool_address = %pool_address,
            tx_hash = %tx_hash,
            prior_tx_count,
            force_simulation,
            action = "evaluate_v2_trading",
            result = "started",
            "starting v2 pool trading simulation"
        );

        let simulation_result = match trading_simulation {
            V2TradingSimulation::Historical(pool_simulator) => pool
                .evaluate_trading_status_v2_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|_| ()),
            V2TradingSimulation::Live(pool_simulator) => {
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v2(pool_simulator, &tx_context, pool_config),
                )
                .await
                {
                    Ok(result) => result.map(|_| ()),
                    Err(_) => Err(eyre!(
                        "live v2 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} prior_tx_count={} force_simulation={}",
                        timeout.as_millis(),
                        tx.block_number,
                        tx.tx_index,
                        tx_hash,
                        token_address,
                        pool_address,
                        prior_tx_count,
                        force_simulation
                    )),
                }
            }
            #[cfg(test)]
            V2TradingSimulation::Noop => Ok(()),
        };

        match simulation_result {
            Ok(()) => {
                tracing::info!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    can_buy = pool.base.state.can_buy,
                    can_sell = pool.base.state.can_sell,
                    buy_tax = ?pool.base.buy_tax,
                    sell_tax = ?pool.base.sell_tax,
                    action = "evaluate_v2_trading",
                    result = "ok",
                    "completed v2 pool trading simulation"
                );
            }
            Err(error) => {
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_address = %pool_address,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    action = "evaluate_v2_trading",
                    result = "error",
                    reason = %error,
                    "failed v2 pool trading simulation"
                );
                return Err(error);
            }
        }
        simulated.push(pool_address.clone());
    }

    Ok(simulated)
}

fn should_simulate_at_current_block(
    pool_address: &str,
    current_block_pool_addresses: &[String],
    _trading_simulation: V2TradingSimulation<'_>,
) -> bool {
    current_block_pool_addresses
        .iter()
        .any(|current_pool| current_pool == pool_address)
}

fn simulation_pool_addresses(
    token: &ERC20Token,
    updated: &[String],
    include_all_token_pools: bool,
) -> Vec<String> {
    let mut addresses = updated.to_vec();
    if include_all_token_pools {
        addresses.extend(token.pool_addresses());
    }
    addresses.sort();
    addresses.dedup();
    addresses
}

fn current_block_simulation_pool_addresses(
    simulation_pool_addresses: &[String],
    updated: &[String],
    discovered: &[String],
    force_simulation: bool,
) -> Vec<String> {
    let mut addresses = if force_simulation {
        simulation_pool_addresses.to_vec()
    } else {
        Vec::new()
    };
    addresses.extend(updated.iter().cloned());
    addresses.extend(discovered.iter().cloned());
    addresses.sort();
    addresses.dedup();
    addresses
}

fn simulation_prior_txs(
    prior_txs: &[ProcessedTransaction],
    current_tx: &ProcessedTransaction,
    include_current_tx: bool,
) -> Vec<ProcessedTransaction> {
    let mut simulation_prior_txs = prior_txs.to_vec();
    if include_current_tx
        && !simulation_prior_txs
            .iter()
            .any(|prior_tx| prior_tx.hash == current_tx.hash)
    {
        simulation_prior_txs.push(current_tx.clone());
    }
    simulation_prior_txs.sort_by_key(|tx| tx.tx_index);
    simulation_prior_txs.dedup_by_key(|tx| tx.hash);
    simulation_prior_txs
}

fn should_simulate_v2_trading(pool: &UniswapV2Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.is_scam() {
        return false;
    }

    let pool_address = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v2_trading_simulation_trigger(tx, pool_address))
}

fn has_v2_trading_simulation_trigger(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v2_swaps
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
}

fn tx_control_addresses(tx: &ProcessedTransaction) -> Vec<String> {
    let mut addresses: Vec<String> = tx.unique_addresses.iter().map(address_string).collect();
    addresses.push(address_string(&tx.from_address));
    if let Some(to_address) = tx.to_address {
        addresses.push(address_string(&to_address));
    }
    if let Some(contract_address) = tx.contract_address {
        addresses.push(address_string(&contract_address));
    }
    addresses
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_current_block_pool_uses_current_block_state() {
        let pool = "0xb80b6c2453b82996950a81c42db446e95a3fa2d5";

        assert!(should_simulate_at_current_block(
            pool,
            &[pool.to_string()],
            V2TradingSimulation::Noop
        ));
    }

    #[test]
    fn existing_pool_without_current_discovery_uses_parent_replay() {
        assert!(!should_simulate_at_current_block(
            "0xb80b6c2453b82996950a81c42db446e95a3fa2d5",
            &[],
            V2TradingSimulation::Noop
        ));
    }

    #[test]
    fn updated_pool_uses_current_block_state_without_setup_replay() {
        let updated = vec!["0x0000000000000000000000000000000000000001".to_string()];
        let current = current_block_simulation_pool_addresses(&[], &updated, &[], false);

        assert_eq!(current, updated);
    }

    #[test]
    fn forced_token_control_simulates_all_pools_at_current_block() {
        let pools = vec![
            "0x0000000000000000000000000000000000000002".to_string(),
            "0x0000000000000000000000000000000000000001".to_string(),
        ];
        let current = current_block_simulation_pool_addresses(&pools, &[], &[], true);

        assert_eq!(
            current,
            vec![
                "0x0000000000000000000000000000000000000001".to_string(),
                "0x0000000000000000000000000000000000000002".to_string()
            ]
        );
    }

    #[test]
    fn live_header_gap_is_optional_pool_metadata_miss() {
        assert!(is_not_uniswap_v2_pool_metadata_miss(
            "missing live block header for 25050934"
        ));
    }
}
