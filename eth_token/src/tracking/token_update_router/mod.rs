use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::{
    BlockStateSession, BlockTxStateSession, LivePoolBuySellSimulator, PoolBuySellSimulator,
    ProcessedTransaction,
};

use crate::chain_metadata::UniswapV2PoolMetadataProvider;
use reth_chain_query::provider::BlockHeader;

use super::replay_context::triggers::tx_is_token_control_replay_candidate;
use super::{normalize_address_string, TokenRegistry, TokenStateUpdateReport, TrackedTokenIndex};

mod known_token_metadata;
mod pool_discovery;
mod pool_metadata_lookup;
mod pool_state_update;
mod token_candidates;
mod token_state_update;
mod trading_status_update;

#[cfg(test)]
mod tests;

use pool_state_update::{
    has_protocol_pool_changes, update_touched_v2_pools, update_touched_v3_pools,
    update_touched_v4_pools,
};
use token_candidates::{candidate_token_addresses, candidate_token_addresses_with_pool_discovery};
use token_state_update::touches_token_state;
use trading_status_update::{
    current_block_simulation_pool_addresses, simulate_updated_v2_pools, simulate_updated_v3_pools,
    simulate_updated_v4_pools, simulation_pool_addresses,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct ProcessedTokenUpdateProfile {
    pub candidate_ms: u128,
    pub token_state_ms: u128,
    pub pool_discovery_ms: u128,
    pub pool_update_ms: u128,
    pub simulation_v2_ms: u128,
    pub simulation_v3_ms: u128,
    pub simulation_v4_ms: u128,
    pub report_ms: u128,
    pub candidate_tokens: usize,
    pub visited_tokens: usize,
    pub update_reports: usize,
    pub simulated_v2_pools: usize,
    pub simulated_v3_pools: usize,
    pub simulated_v4_pools: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessedTokenUpdateRouter {
    pub history_limit: usize,
    pub known_routers: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum PoolTradingSimulationMode<'a> {
    Historical(&'a PoolBuySellSimulator),
    HistoricalBlockSession {
        pool_simulator: &'a PoolBuySellSimulator,
        block_session: &'a Mutex<Option<BlockTxStateSession>>,
    },
    LiveBlockSession {
        pool_simulator: &'a LivePoolBuySellSimulator,
        block_sessions: &'a Mutex<BTreeMap<u64, BlockStateSession>>,
    },
    #[cfg(test)]
    Noop,
}

impl PoolTradingSimulationMode<'_> {
    fn is_live(self) -> bool {
        matches!(self, Self::LiveBlockSession { .. })
    }
}

pub(super) const LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS: u64 = 2_500;
pub(super) const LIVE_POOL_SIMULATION_TIMEOUT_MS: u64 = 2_500;
pub(super) const LIVE_TOKEN_TRACKER_LOG_TARGET: &str = "live_token_tracker";

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

            let discovered_v2 = self.discover_known_v2_pools_for_token(token, tx);
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
                    discovered_known_v2_pools: discovered_v2,
                    updated_known_v2_pools: updated_v2,
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
    ) -> Result<Vec<TokenStateUpdateReport>> {
        self.update_registry_from_processed_transaction_with_trading_simulation(
            registry,
            token_index,
            tx,
            PoolTradingSimulationMode::Historical(pool_simulator),
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
        block_sessions: &Mutex<BTreeMap<u64, BlockStateSession>>,
    ) -> Result<Vec<TokenStateUpdateReport>> {
        self.update_registry_from_processed_transaction_with_trading_simulation(
            registry,
            token_index,
            tx,
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions,
            },
            None,
        )
        .await
    }

    pub(crate) async fn update_registry_from_processed_transaction_with_trading_simulation(
        &self,
        registry: &mut TokenRegistry,
        token_index: &TrackedTokenIndex,
        tx: &ProcessedTransaction,
        trading_simulation: PoolTradingSimulationMode<'_>,
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

            let discovered_v2 = self.discover_known_v2_pools_for_token(token, tx);
            let discovered_v3 = self.discover_uniswap_v3_pools_for_token(token, tx);
            let discovered_v4 = self.discover_uniswap_v4_pools_for_token(token, tx);
            let updated_v2 = update_touched_v2_pools(token, tx)?;
            let updated_v3 = update_touched_v3_pools(token, tx)?;
            let updated_v4 = update_touched_v4_pools(token, tx)?;
            let token_control_replay =
                token_state_updated && tx_is_token_control_replay_candidate(token, tx);
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
                    discovered_known_v2_pools: discovered_v2,
                    updated_known_v2_pools: updated_v2,
                    simulated_known_v2_pools: simulated_v2,
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
                .discover_known_v2_pools_for_token_with_metadata(
                    token,
                    tx,
                    pool_metadata_provider,
                    None,
                )
                .await?;
            discovered_v2.extend(
                self.discover_known_v2_pools_from_events_for_token(
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
                    discovered_known_v2_pools: discovered_v2,
                    updated_known_v2_pools: updated_v2,
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
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        self.update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            PoolTradingSimulationMode::Historical(pool_simulator),
            None,
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
        block_sessions: &Mutex<BTreeMap<u64, BlockStateSession>>,
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolMetadataProvider,
    {
        self.update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions,
            },
            None,
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
        trading_simulation: PoolTradingSimulationMode<'_>,
        block_header: Option<&BlockHeader>,
        mut profile: Option<&mut ProcessedTokenUpdateProfile>,
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
        let candidate_started = Instant::now();
        let token_addresses = candidate_token_addresses_with_pool_discovery(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            pool_metadata_timeout,
        )
        .await?;
        if let Some(profile) = profile.as_deref_mut() {
            profile.candidate_ms += elapsed_millis(candidate_started);
            profile.candidate_tokens += token_addresses.len();
        }
        let mut reports = Vec::new();

        for token_address in token_addresses {
            let Some(token) = registry.token_mut(&token_address) else {
                continue;
            };
            if let Some(profile) = profile.as_deref_mut() {
                profile.visited_tokens += 1;
            }

            let token_state_started = Instant::now();
            let token_state_updated = touches_token_state(tx, &token_address);
            if token_state_updated {
                token.update_token_state_from_processed_transaction(tx)?;
            }
            if let Some(profile) = profile.as_deref_mut() {
                profile.token_state_ms += elapsed_millis(token_state_started);
            }

            let pool_discovery_started = Instant::now();
            let mut discovered_v2 = self
                .discover_known_v2_pools_for_token_with_metadata(
                    token,
                    tx,
                    pool_metadata_provider,
                    pool_metadata_timeout,
                )
                .await?;
            discovered_v2.extend(
                self.discover_known_v2_pools_from_events_for_token(
                    token,
                    tx,
                    pool_metadata_provider,
                    pool_metadata_timeout,
                )
                .await?,
            );
            let discovered_v3 = self.discover_uniswap_v3_pools_for_token(token, tx);
            let discovered_v4 = self.discover_uniswap_v4_pools_for_token(token, tx);
            if let Some(profile) = profile.as_deref_mut() {
                profile.pool_discovery_ms += elapsed_millis(pool_discovery_started);
            }

            let pool_update_started = Instant::now();
            let updated_v2 = update_touched_v2_pools(token, tx)?;
            let updated_v3 = update_touched_v3_pools(token, tx)?;
            let updated_v4 = update_touched_v4_pools(token, tx)?;
            if let Some(profile) = profile.as_deref_mut() {
                profile.pool_update_ms += elapsed_millis(pool_update_started);
            }

            let token_control_replay =
                token_state_updated && tx_is_token_control_replay_candidate(token, tx);
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
            let simulation_v2_started = Instant::now();
            let simulated_v2 = simulate_updated_v2_pools(
                token,
                tx,
                &simulation_v2_pool_addresses,
                &current_block_v2_pool_addresses,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;
            if let Some(profile) = profile.as_deref_mut() {
                profile.simulation_v2_ms += elapsed_millis(simulation_v2_started);
                profile.simulated_v2_pools += simulated_v2.len();
            }
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
            let simulation_v3_started = Instant::now();
            let simulated_v3 = simulate_updated_v3_pools(
                token,
                tx,
                &simulation_v3_pool_addresses,
                &current_block_v3_pool_addresses,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;
            if let Some(profile) = profile.as_deref_mut() {
                profile.simulation_v3_ms += elapsed_millis(simulation_v3_started);
                profile.simulated_v3_pools += simulated_v3.len();
            }
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
            let simulation_v4_started = Instant::now();
            let simulated_v4 = simulate_updated_v4_pools(
                token,
                tx,
                &simulation_v4_pool_keys,
                &current_block_v4_pool_keys,
                trading_simulation,
                token_control_replay,
                block_header,
            )
            .await?;
            if let Some(profile) = profile.as_deref_mut() {
                profile.simulation_v4_ms += elapsed_millis(simulation_v4_started);
                profile.simulated_v4_pools += simulated_v4.len();
            }

            let report_started = Instant::now();
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
                    discovered_known_v2_pools: discovered_v2,
                    updated_known_v2_pools: updated_v2,
                    simulated_known_v2_pools: simulated_v2,
                    discovered_uniswap_v3_pools: discovered_v3,
                    updated_uniswap_v3_pools: updated_v3,
                    simulated_uniswap_v3_pools: simulated_v3,
                    discovered_uniswap_v4_pools: discovered_v4,
                    updated_uniswap_v4_pools: updated_v4,
                    simulated_uniswap_v4_pools: simulated_v4,
                });
                if let Some(profile) = profile.as_deref_mut() {
                    profile.update_reports += 1;
                }
            }
            if let Some(profile) = profile.as_deref_mut() {
                profile.report_ms += elapsed_millis(report_started);
            }
        }

        Ok(reports)
    }
}

fn elapsed_millis(started: Instant) -> u128 {
    started.elapsed().as_millis()
}
