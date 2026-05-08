use std::collections::BTreeSet;
use std::time::Duration;

use alloy_primitives::Address;
use eyre::{eyre, Result};
use reth_chain_query::common_addresses::{get_token_decimals, get_token_symbol};
use reth_chain_query::provider::BlockHeader;
use serde::{Deserialize, Serialize};
use tx_processor::{LivePoolBuySellSimulator, PoolBuySellSimulator, ProcessedTransaction};

use crate::chain_metadata::{
    UniswapV2PoolMetadata, UniswapV2PoolMetadataLookup, UniswapV2PoolMetadataProvider,
};
use crate::erc20::ERC20Token;
use crate::pools::uniswap::{
    display_denom_for_v4_currency, v4_event_display_key, PoolTradingSimulationConfig,
    UniswapV2TxContext,
};
use crate::pools::{BasePoolConfig, UniswapV2Pool, UniswapV3Pool, UniswapV4Pool};

use super::replay_context::triggers::tx_is_token_control_replay_candidate;
use super::trading_failure::classify_v2_trading_failure;
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

            let discovered_v2 = self.discover_uniswap_v2_pools_for_token(token, tx);
            let discovered_v3 = self.discover_uniswap_v3_pools_for_token(token, tx);
            let discovered_v4 = self.discover_uniswap_v4_pools_for_token(token, tx);
            let updated_v2 = update_touched_v2_pools(token, tx)?;
            let updated_v3 = update_touched_v3_pools(token, tx)?;
            let updated_v4 = update_touched_v4_pools(token, tx)?;

            if token_state_updated
                || has_protocol_pool_changes(&[
                    &discovered_v2,
                    &updated_v2,
                    &discovered_v3,
                    &updated_v3,
                    &discovered_v4,
                    &updated_v4,
                ])
            {
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered_v2,
                    updated_uniswap_v2_pools: updated_v2,
                    discovered_uniswap_v3_pools: discovered_v3,
                    updated_uniswap_v3_pools: updated_v3,
                    discovered_uniswap_v4_pools: discovered_v4,
                    updated_uniswap_v4_pools: updated_v4,
                    ..Default::default()
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

            let discovered_v2 = self.discover_uniswap_v2_pools_for_token(token, tx);
            let discovered_v3 = self.discover_uniswap_v3_pools_for_token(token, tx);
            let discovered_v4 = self.discover_uniswap_v4_pools_for_token(token, tx);
            let updated_v2 = update_touched_v2_pools(token, tx)?;
            let updated_v3 = update_touched_v3_pools(token, tx)?;
            let updated_v4 = update_touched_v4_pools(token, tx)?;
            let token_control_replay =
                token_state_updated && tx_is_token_control_replay_candidate(token, tx);
            let simulation_prior_txs = simulation_prior_txs(prior_txs, tx, token_control_replay);
            let simulation_v2_pool_addresses = simulation_pool_addresses(
                token.uniswap_v2_pool_addresses(),
                &updated_v2,
                token_control_replay,
            );
            let current_block_v2_pool_addresses = current_block_simulation_pool_addresses(
                &simulation_v2_pool_addresses,
                &updated_v2,
                &discovered_v2,
                token_control_replay,
            );
            let simulated_v2 = simulate_updated_v2_pools(
                token,
                tx,
                &simulation_v2_pool_addresses,
                &current_block_v2_pool_addresses,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;
            let simulation_v3_pool_addresses = simulation_pool_addresses(
                token.uniswap_v3_pool_addresses(),
                &updated_v3,
                token_control_replay,
            );
            let current_block_v3_pool_addresses = current_block_simulation_pool_addresses(
                &simulation_v3_pool_addresses,
                &updated_v3,
                &discovered_v3,
                token_control_replay,
            );
            let simulated_v3 = simulate_updated_v3_pools(
                token,
                tx,
                &simulation_v3_pool_addresses,
                &current_block_v3_pool_addresses,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;
            let simulation_v4_pool_keys = simulation_pool_addresses(
                token.uniswap_v4_pool_keys(),
                &updated_v4,
                token_control_replay,
            );
            let current_block_v4_pool_keys = current_block_simulation_pool_addresses(
                &simulation_v4_pool_keys,
                &updated_v4,
                &discovered_v4,
                token_control_replay,
            );
            let simulated_v4 = simulate_updated_v4_pools(
                token,
                tx,
                &simulation_v4_pool_keys,
                &current_block_v4_pool_keys,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;

            if token_state_updated
                || has_protocol_pool_changes(&[
                    &discovered_v2,
                    &updated_v2,
                    &simulated_v2,
                    &discovered_v3,
                    &updated_v3,
                    &simulated_v3,
                    &discovered_v4,
                    &updated_v4,
                    &simulated_v4,
                ])
            {
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered_v2,
                    updated_uniswap_v2_pools: updated_v2,
                    simulated_uniswap_v2_pools: simulated_v2,
                    discovered_uniswap_v3_pools: discovered_v3,
                    updated_uniswap_v3_pools: updated_v3,
                    simulated_uniswap_v3_pools: simulated_v3,
                    discovered_uniswap_v4_pools: discovered_v4,
                    updated_uniswap_v4_pools: updated_v4,
                    simulated_uniswap_v4_pools: simulated_v4,
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

            let mut discovered_v2 = self
                .discover_uniswap_v2_pools_for_token_with_metadata(
                    token,
                    tx,
                    pool_metadata_provider,
                    None,
                )
                .await?;
            discovered_v2.extend(
                self.discover_uniswap_v2_pools_from_swaps_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                    None,
                )
                .await?,
            );
            let discovered_v3 = self.discover_uniswap_v3_pools_for_token(token, tx);
            let discovered_v4 = self.discover_uniswap_v4_pools_for_token(token, tx);

            let updated_v2 = update_touched_v2_pools(token, tx)?;
            let updated_v3 = update_touched_v3_pools(token, tx)?;
            let updated_v4 = update_touched_v4_pools(token, tx)?;

            if token_state_updated
                || has_protocol_pool_changes(&[
                    &discovered_v2,
                    &updated_v2,
                    &discovered_v3,
                    &updated_v3,
                    &discovered_v4,
                    &updated_v4,
                ])
            {
                discovered_v2.sort();
                discovered_v2.dedup();
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered_v2,
                    updated_uniswap_v2_pools: updated_v2,
                    discovered_uniswap_v3_pools: discovered_v3,
                    updated_uniswap_v3_pools: updated_v3,
                    discovered_uniswap_v4_pools: discovered_v4,
                    updated_uniswap_v4_pools: updated_v4,
                    ..Default::default()
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

            let mut discovered_v2 = self
                .discover_uniswap_v2_pools_for_token_with_metadata(
                    token,
                    tx,
                    pool_metadata_provider,
                    pool_metadata_timeout,
                )
                .await?;
            discovered_v2.extend(
                self.discover_uniswap_v2_pools_from_swaps_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                    pool_metadata_timeout,
                )
                .await?,
            );
            let discovered_v3 = self.discover_uniswap_v3_pools_for_token(token, tx);
            let discovered_v4 = self.discover_uniswap_v4_pools_for_token(token, tx);

            let updated_v2 = update_touched_v2_pools(token, tx)?;
            let updated_v3 = update_touched_v3_pools(token, tx)?;
            let updated_v4 = update_touched_v4_pools(token, tx)?;
            let token_control_replay =
                token_state_updated && tx_is_token_control_replay_candidate(token, tx);
            let simulation_prior_txs = simulation_prior_txs(prior_txs, tx, token_control_replay);
            let simulation_v2_pool_addresses = simulation_pool_addresses(
                token.uniswap_v2_pool_addresses(),
                &updated_v2,
                token_control_replay,
            );
            let current_block_v2_pool_addresses = current_block_simulation_pool_addresses(
                &simulation_v2_pool_addresses,
                &updated_v2,
                &discovered_v2,
                token_control_replay,
            );
            let simulated_v2 = simulate_updated_v2_pools(
                token,
                tx,
                &simulation_v2_pool_addresses,
                &current_block_v2_pool_addresses,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;
            let simulation_v3_pool_addresses = simulation_pool_addresses(
                token.uniswap_v3_pool_addresses(),
                &updated_v3,
                token_control_replay,
            );
            let current_block_v3_pool_addresses = current_block_simulation_pool_addresses(
                &simulation_v3_pool_addresses,
                &updated_v3,
                &discovered_v3,
                token_control_replay,
            );
            let simulated_v3 = simulate_updated_v3_pools(
                token,
                tx,
                &simulation_v3_pool_addresses,
                &current_block_v3_pool_addresses,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;
            let simulation_v4_pool_keys = simulation_pool_addresses(
                token.uniswap_v4_pool_keys(),
                &updated_v4,
                token_control_replay,
            );
            let current_block_v4_pool_keys = current_block_simulation_pool_addresses(
                &simulation_v4_pool_keys,
                &updated_v4,
                &discovered_v4,
                token_control_replay,
            );
            let simulated_v4 = simulate_updated_v4_pools(
                token,
                tx,
                &simulation_v4_pool_keys,
                &current_block_v4_pool_keys,
                &simulation_prior_txs,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;

            if token_state_updated
                || has_protocol_pool_changes(&[
                    &discovered_v2,
                    &updated_v2,
                    &simulated_v2,
                    &discovered_v3,
                    &updated_v3,
                    &simulated_v3,
                    &discovered_v4,
                    &updated_v4,
                    &simulated_v4,
                ])
            {
                discovered_v2.sort();
                discovered_v2.dedup();
                reports.push(TokenStateUpdateReport {
                    token_address,
                    token_state_updated,
                    discovered_uniswap_v2_pools: discovered_v2,
                    updated_uniswap_v2_pools: updated_v2,
                    simulated_uniswap_v2_pools: simulated_v2,
                    discovered_uniswap_v3_pools: discovered_v3,
                    updated_uniswap_v3_pools: updated_v3,
                    simulated_uniswap_v3_pools: simulated_v3,
                    discovered_uniswap_v4_pools: discovered_v4,
                    updated_uniswap_v4_pools: updated_v4,
                    simulated_uniswap_v4_pools: simulated_v4,
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

    fn discover_uniswap_v3_pools_for_token(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
    ) -> Vec<String> {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v3_pools {
            let token_is_token0 = same_address_str(event.token0, &token_address);
            let token_is_token1 = same_address_str(event.token1, &token_address);
            if !token_is_token0 && !token_is_token1 {
                continue;
            }

            let pool_address = address_string(&event.pool);
            if token.uniswap_v3_pool(&pool_address).is_some() {
                continue;
            }

            let denom_address = if token_is_token0 {
                event.token1
            } else {
                event.token0
            };
            let pool = token.create_uniswap_v3_pool(
                pool_address.clone(),
                address_string(&denom_address),
                address_string(&event.token0),
                address_string(&event.token1),
                event.fee,
                event.tick_spacing,
                BasePoolConfig {
                    token_decimals: token.decimals,
                    denom_decimals: known_decimals_for_address(denom_address),
                    token1_is_denom: Some(token_is_token0),
                    history_limit: self.history_limit,
                    denom_threshold: 0.0,
                    threshold_unit: None,
                    test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
                },
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            discovered.push(pool_address);
        }

        discovered
    }

    fn discover_uniswap_v4_pools_for_token(
        &self,
        token: &mut ERC20Token,
        tx: &ProcessedTransaction,
    ) -> Vec<String> {
        let token_address = token.contract_address.clone();
        let mut discovered = Vec::new();

        for event in &tx.uniswap_v4_initializes {
            let token_is_currency0 = same_address_str(event.currency0, &token_address);
            let token_is_currency1 = same_address_str(event.currency1, &token_address);
            if !token_is_currency0 && !token_is_currency1 {
                continue;
            }

            let pool_key = v4_event_display_key(event.pool_manager_address, event.event_id);
            if token.uniswap_v4_pool(&pool_key).is_some() {
                continue;
            }

            let denom_currency = if token_is_currency0 {
                event.currency1
            } else {
                event.currency0
            };
            let mut pool = UniswapV4Pool::from_initialize_event(
                event,
                token_address.clone(),
                display_denom_for_v4_currency(denom_currency),
                BasePoolConfig {
                    token_decimals: token.decimals,
                    denom_decimals: known_decimals_for_address_or_native(denom_currency),
                    token1_is_denom: Some(token_is_currency0),
                    history_limit: self.history_limit,
                    denom_threshold: 0.0,
                    threshold_unit: None,
                    test_buy_amount_eth: crate::pools::base::DEFAULT_TEST_BUY_ETH,
                },
            );
            pool.base.creation_block = Some(tx.block_number);
            pool.base.creation_tx = Some(hash_string(&tx.hash));
            pool.base.creation_timestamp = Some(tx.block_timestamp);
            token.add_uniswap_v4_pool(pool);
            discovered.push(pool_key);
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
    for pool_key in v4_pool_event_keys(tx) {
        insert_resolved_token_address_str(registry, token_index, &mut candidates, &pool_key);
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
    for pool_key in v4_pool_event_keys(tx) {
        insert_resolved_token_address_str(registry, token_index, &mut candidates, &pool_key);
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
        || message.contains("Failed to get token decimals")
        || message.contains("Token decimals call")
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
    for event in &tx.uniswap_v3_pools {
        addresses.insert(event.pool);
        addresses.insert(event.token0);
        addresses.insert(event.token1);
    }
    addresses.extend(v3_pool_event_addresses(tx));
    for event in &tx.uniswap_v4_initializes {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.currency0);
        addresses.insert(event.currency1);
        addresses.insert(event.hooks);
    }
    for event in &tx.uniswap_v4_modifies {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.sender);
    }
    for event in &tx.uniswap_v4_swaps {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.sender);
    }
    for event in &tx.uniswap_v4_donates {
        addresses.insert(event.pool_manager_address);
        addresses.insert(event.sender);
    }

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

fn v3_pool_event_addresses(tx: &ProcessedTransaction) -> BTreeSet<Address> {
    let mut addresses = BTreeSet::new();
    for event in &tx.uniswap_v3_initializations {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_swaps {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_mints {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_burns {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_positions {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_increases {
        addresses.insert(event.pool_address);
    }
    for event in &tx.uniswap_v3_decreases {
        addresses.insert(event.pool_address);
    }
    addresses
}

fn v4_pool_event_keys(tx: &ProcessedTransaction) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for event in &tx.uniswap_v4_initializes {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_modifies {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_swaps {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_donates {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_protocol_fee_updates {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    for event in &tx.uniswap_v4_dynamic_lp_fee_updates {
        keys.insert(v4_event_display_key(
            event.pool_manager_address,
            event.event_id,
        ));
    }
    keys
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
    let pool_addresses = token.uniswap_v2_pool_addresses();
    let mut updated = Vec::new();
    for pool_address in pool_addresses {
        if touches_v2_pool(tx, &pool_address) {
            token.update_uniswap_v2_pool_from_processed_transaction(&pool_address, tx)?;
            updated.push(pool_address);
        }
    }
    Ok(updated)
}

fn update_touched_v3_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
) -> Result<Vec<String>> {
    let pool_addresses = token.uniswap_v3_pool_addresses();
    let mut updated = Vec::new();
    for pool_address in pool_addresses {
        if touches_v3_pool(tx, &pool_address) {
            token.update_uniswap_v3_pool_from_processed_transaction(&pool_address, tx)?;
            updated.push(pool_address);
        }
    }
    Ok(updated)
}

fn update_touched_v4_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
) -> Result<Vec<String>> {
    let pool_keys = token.uniswap_v4_pool_keys();
    let mut updated = Vec::new();
    for pool_key in pool_keys {
        if touches_v4_pool(tx, &pool_key) {
            token.update_uniswap_v4_pool_from_processed_transaction(&pool_key, tx)?;
            updated.push(pool_key);
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
    let config = PoolTradingSimulationConfig {
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
            PoolTradingSimulationConfig {
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
                .map(|result| result.failure_reason.clone()),
            V2TradingSimulation::Live(pool_simulator) => {
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v2(pool_simulator, &tx_context, pool_config),
                )
                .await
                {
                    Ok(result) => result.map(|result| result.failure_reason.clone()),
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
            V2TradingSimulation::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                let failure_class = failure_reason
                    .as_ref()
                    .and_then(|reason| classify_v2_trading_failure(pool, tx, reason));
                let failure_class_code = failure_class.map(|class| class.code());
                let failure_class_description = failure_class.map(|class| class.description());
                pool.base.set_trading_failure_context(
                    failure_reason.clone(),
                    failure_class_code.map(|code| code.to_string()),
                );
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
                    reason = ?failure_reason,
                    failure_class = ?failure_class_code,
                    failure_class_description = ?failure_class_description,
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

async fn simulate_updated_v3_pools(
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
    let config = PoolTradingSimulationConfig {
        prior_txs: prior_txs.to_vec(),
        ..Default::default()
    };

    let mut simulated = Vec::new();
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v3_pool(pool_address)
            .map(|pool| {
                !pool.base.is_scam() && (force_simulation || should_simulate_v3_trading(pool, tx))
            })
            .unwrap_or(false);
        if !should_simulate {
            continue;
        }

        let Some(pool) = token.uniswap_v3_pool_mut(pool_address) else {
            continue;
        };

        let pool_config = if should_simulate_at_current_block(
            pool_address,
            current_block_pool_addresses,
            trading_simulation,
        ) {
            PoolTradingSimulationConfig {
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

        let simulation_result = match trading_simulation {
            V2TradingSimulation::Historical(pool_simulator) => pool
                .evaluate_trading_status_v3_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            V2TradingSimulation::Live(pool_simulator) => {
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v3(pool_simulator, &tx_context, pool_config),
                )
                .await
                {
                    Ok(result) => result.map(|result| result.failure_reason.clone()),
                    Err(_) => Err(eyre!(
                        "live v3 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} prior_tx_count={} force_simulation={}",
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
            V2TradingSimulation::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                pool.base
                    .set_trading_failure_context(failure_reason.clone(), None);
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
                    reason = ?failure_reason,
                    action = "evaluate_v3_trading",
                    result = "ok",
                    "completed v3 pool trading simulation"
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
                    action = "evaluate_v3_trading",
                    result = "error",
                    reason = %error,
                    "failed v3 pool trading simulation"
                );
                return Err(error);
            }
        }
        simulated.push(pool_address.clone());
    }

    Ok(simulated)
}

async fn simulate_updated_v4_pools(
    token: &mut ERC20Token,
    tx: &ProcessedTransaction,
    pool_keys: &[String],
    current_block_pool_keys: &[String],
    prior_txs: &[ProcessedTransaction],
    trading_simulation: V2TradingSimulation<'_>,
    force_simulation: bool,
    block_header: Option<&BlockHeader>,
) -> Result<Vec<String>> {
    if pool_keys.is_empty() {
        return Ok(Vec::new());
    }

    let token_address = token.contract_address.clone();
    let tx_context = UniswapV2TxContext {
        block_number: tx.block_number,
        block_timestamp: tx.block_timestamp,
        tx_hash: hash_string(&tx.hash),
        from_address: Some(address_string(&tx.from_address)),
    };
    let config = PoolTradingSimulationConfig {
        prior_txs: prior_txs.to_vec(),
        ..Default::default()
    };

    let mut simulated = Vec::new();
    for pool_key in pool_keys {
        let should_simulate = token
            .uniswap_v4_pool(pool_key)
            .map(|pool| {
                !pool.base.is_scam() && (force_simulation || should_simulate_v4_trading(pool, tx))
            })
            .unwrap_or(false);
        if !should_simulate {
            continue;
        }

        let Some(pool) = token.uniswap_v4_pool_mut(pool_key) else {
            continue;
        };

        let pool_config = if should_simulate_at_current_block(
            pool_key,
            current_block_pool_keys,
            trading_simulation,
        ) {
            PoolTradingSimulationConfig {
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

        let simulation_result = match trading_simulation {
            V2TradingSimulation::Historical(pool_simulator) => pool
                .evaluate_trading_status_v4_with_pool_simulator(
                    pool_simulator,
                    &tx_context,
                    pool_config,
                )
                .await
                .map(|result| result.failure_reason.clone()),
            V2TradingSimulation::Live(pool_simulator) => {
                let timeout = Duration::from_millis(LIVE_POOL_SIMULATION_TIMEOUT_MS);
                match tokio::time::timeout(
                    timeout,
                    pool.evaluate_live_trading_status_v4(pool_simulator, &tx_context, pool_config),
                )
                .await
                {
                    Ok(result) => result.map(|result| result.failure_reason.clone()),
                    Err(_) => Err(eyre!(
                        "live v4 pool trading simulation timed out after {} ms block={} tx_index={} tx_hash={} token={} pool={} prior_tx_count={} force_simulation={}",
                        timeout.as_millis(),
                        tx.block_number,
                        tx.tx_index,
                        tx_hash,
                        token_address,
                        pool_key,
                        prior_tx_count,
                        force_simulation
                    )),
                }
            }
            #[cfg(test)]
            V2TradingSimulation::Noop => Ok(None),
        };

        match simulation_result {
            Ok(failure_reason) => {
                pool.base
                    .set_trading_failure_context(failure_reason.clone(), None);
                tracing::info!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_key = %pool_key,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    can_buy = pool.base.state.can_buy,
                    can_sell = pool.base.state.can_sell,
                    buy_tax = ?pool.base.buy_tax,
                    sell_tax = ?pool.base.sell_tax,
                    reason = ?failure_reason,
                    action = "evaluate_v4_trading",
                    result = "ok",
                    "completed v4 pool trading simulation"
                );
                simulated.push(pool_key.clone());
            }
            Err(error) => {
                let reason = error.to_string();
                pool.base.set_trading_failure_context(
                    Some(reason.clone()),
                    Some("simulator_error".to_string()),
                );
                tracing::warn!(
                    target: "pool_buy_sell_sim",
                    block_number = tx.block_number,
                    token_address = %token_address,
                    pool_key = %pool_key,
                    tx_hash = %tx_hash,
                    prior_tx_count,
                    force_simulation,
                    action = "evaluate_v4_trading",
                    result = "skipped",
                    reason = %reason,
                    "skipped v4 pool trading simulation and preserved pool state"
                );
            }
        }
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
    all_pool_addresses: Vec<String>,
    updated: &[String],
    include_all_token_pools: bool,
) -> Vec<String> {
    let mut addresses = updated.to_vec();
    if include_all_token_pools {
        addresses.extend(all_pool_addresses);
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

fn should_simulate_v3_trading(pool: &UniswapV3Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.is_scam() {
        return false;
    }

    let pool_address = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v3_trading_simulation_trigger(tx, pool_address))
}

fn should_simulate_v4_trading(pool: &UniswapV4Pool, tx: &ProcessedTransaction) -> bool {
    if pool.base.is_scam() {
        return false;
    }

    let pool_key = &pool.base.identity.pool_address;
    pool.base.has_control_address(tx_control_addresses(tx))
        || (!pool.base.can_buy_and_sell() && has_v4_trading_simulation_trigger(tx, pool_key))
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

fn has_v3_trading_simulation_trigger(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v3_initializations
        .iter()
        .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_swaps
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_mints
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_burns
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
}

fn has_v4_trading_simulation_trigger(tx: &ProcessedTransaction, pool_key: &str) -> bool {
    tx.uniswap_v4_initializes
        .iter()
        .any(|event| v4_event_display_key(event.pool_manager_address, event.event_id) == pool_key)
        || tx.uniswap_v4_modifies.iter().any(|event| {
            v4_event_display_key(event.pool_manager_address, event.event_id) == pool_key
        })
        || tx.uniswap_v4_swaps.iter().any(|event| {
            v4_event_display_key(event.pool_manager_address, event.event_id) == pool_key
        })
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

fn touches_v3_pool(tx: &ProcessedTransaction, pool_address: &str) -> bool {
    tx.uniswap_v3_initializations
        .iter()
        .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_swaps
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_mints
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_burns
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_positions
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_increases
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
        || tx
            .uniswap_v3_decreases
            .iter()
            .any(|event| same_address_str(event.pool_address, pool_address))
}

fn touches_v4_pool(tx: &ProcessedTransaction, pool_key: &str) -> bool {
    v4_pool_event_keys(tx).iter().any(|key| key == pool_key)
}

fn known_decimals_for_address(address: Address) -> Option<u8> {
    get_token_symbol(address).and_then(get_token_decimals)
}

fn known_decimals_for_address_or_native(address: Address) -> Option<u8> {
    if address.is_zero() {
        Some(18)
    } else {
        known_decimals_for_address(address)
    }
}

fn has_protocol_pool_changes(changes: &[&Vec<String>]) -> bool {
    changes.iter().any(|change| !change.is_empty())
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

    #[test]
    fn token_decimals_failures_are_optional_pool_metadata_misses() {
        assert!(is_not_uniswap_v2_pool_metadata_miss(
            "Failed to get token decimals for 0x38c6a68304cdefb9bec48bbfaaba5c5b47818bb2"
        ));
        assert!(is_not_uniswap_v2_pool_metadata_miss(
            "Token decimals call for 0xe0b7927c4af23765cb51314a0e0521a9645f0e2a returned 0 bytes (expected >= 32)"
        ));
    }
}
