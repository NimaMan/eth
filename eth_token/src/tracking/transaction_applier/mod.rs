use std::time::Duration;

use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::{LivePoolBuySellSimulator, PoolBuySellSimulator, ProcessedTransaction};

use crate::chain_metadata::UniswapV2PoolMetadataProvider;
use reth_chain_query::provider::BlockHeader;

use super::replay_context::triggers::tx_is_token_control_replay_candidate;
use super::{normalize_address_string, TokenRegistry, TokenStateUpdateReport, TrackedTokenIndex};

mod candidates;
mod discovery;
mod known_tokens;
mod metadata;
mod pool_updates;
mod touches;
mod trading_status;

#[cfg(test)]
mod tests;

use candidates::{candidate_token_addresses, candidate_token_addresses_with_pool_discovery};
use pool_updates::{
    has_protocol_pool_changes, update_touched_v2_pools, update_touched_v3_pools,
    update_touched_v4_pools,
};
use touches::touches_token_state;
use trading_status::{
    current_block_simulation_pool_addresses, simulate_updated_v2_pools, simulate_updated_v3_pools,
    simulate_updated_v4_pools, simulation_pool_addresses, simulation_prior_txs,
};

pub type ProcessedTokenUpdateRouter = TokenTransactionApplier;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenTransactionApplier {
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

pub(super) const LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS: u64 = 2_500;
pub(super) const LIVE_POOL_SIMULATION_TIMEOUT_MS: u64 = 2_500;
pub(super) const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";
pub(super) const UNISWAP_V2_FACTORY: &str = "0x5c69bee701ef814a2b6a3edd4b1652cb9cc5aa6f";

impl TokenTransactionApplier {
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
        if !tx.status {
            return Ok(Vec::new());
        }

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

            let mut discovered_v2 = self.discover_uniswap_v2_pools_for_token(token, tx);
            discovered_v2.extend(self.discover_sushiswap_v2_pools_for_token(token, tx));
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
        if !tx.status {
            return Ok(Vec::new());
        }

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

            let mut discovered_v2 = self.discover_uniswap_v2_pools_for_token(token, tx);
            discovered_v2.extend(self.discover_sushiswap_v2_pools_for_token(token, tx));
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
                self.discover_uniswap_v2_pools_from_events_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                    None,
                )
                .await?,
            );
            discovered_v2.extend(self.discover_sushiswap_v2_pools_for_token(token, tx));
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
        if !tx.status {
            return Ok(Vec::new());
        }

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
                self.discover_uniswap_v2_pools_from_events_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                    pool_metadata_timeout,
                )
                .await?,
            );
            discovered_v2.extend(self.discover_sushiswap_v2_pools_for_token(token, tx));
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
}
