use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use eyre::Result;
use serde::{Deserialize, Serialize};
use tx_processor::{
    BlockStateSession, BlockTxStateSession, LivePoolBuySellSimulator, PoolBuySellSimulator,
    ProcessedTransaction,
};

use crate::chain_metadata::{UniswapV2PoolIdentityProvider, UniswapV2PoolMetadataProvider};
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
mod v2_pool_candidate_router;

#[cfg(test)]
mod tests;

use pool_state_update::{
    has_protocol_pool_changes, update_touched_v2_pools, update_touched_v3_pools,
    update_touched_v4_pools,
};
use token_candidates::{
    candidate_token_addresses, candidate_token_addresses_with_pool_discovery,
    candidate_token_addresses_with_pool_discovery_and_profile, CandidateTokenAddressProfile,
};
use token_state_update::touches_token_state;
use trading_status_update::{
    current_block_simulation_pool_addresses, should_simulate_v2_trading,
    should_simulate_v3_trading, should_simulate_v4_trading, simulate_updated_v2_pools,
    simulate_updated_v3_pools, simulate_updated_v4_pools, simulation_pool_addresses,
};
pub use v2_pool_candidate_router::V2PoolCandidateCache;

#[derive(Clone, Debug, Default)]
pub(crate) struct ProcessedTokenUpdateProfile {
    pub candidate_us: u128,
    pub candidate_routing_addresses_us: u128,
    pub candidate_resolve_addresses_us: u128,
    pub candidate_v4_pool_keys_us: u128,
    pub candidate_v3_position_transfer_us: u128,
    pub candidate_v4_position_transfer_us: u128,
    pub candidate_v4_position_approval_us: u128,
    pub candidate_v2_pair_created_us: u128,
    pub candidate_v2_pool_event_scan_us: u128,
    pub candidate_v2_transfer_route_us: u128,
    pub candidate_v2_cache_route_us: u128,
    pub candidate_v2_identity_lookup_us: u128,
    pub candidate_finalize_us: u128,
    pub token_state_us: u128,
    pub pool_discovery_us: u128,
    pub pool_update_us: u128,
    pub simulation_v2_us: u128,
    pub simulation_v3_us: u128,
    pub simulation_v4_us: u128,
    pub report_us: u128,
    pub candidate_tx_count: usize,
    pub candidate_tokens: usize,
    pub candidate_routing_addresses: usize,
    pub candidate_v4_pool_keys: usize,
    pub candidate_v2_pool_events: usize,
    pub candidate_v2_transfer_route_hits: usize,
    pub candidate_v2_identity_cache_hits: usize,
    pub candidate_v2_irrelevant_cache_hits: usize,
    pub candidate_v2_irrelevant_cache_inserts: usize,
    pub candidate_v2_identity_lookups: usize,
    pub candidate_v2_identity_hits: usize,
    pub candidate_v2_identity_skipped_by_transfer: usize,
    pub candidate_position_scan_tokens: usize,
    pub visited_tokens: usize,
    pub token_state_updates: usize,
    pub token_control_replays: usize,
    pub update_reports: usize,
    pub simulation_v2_candidate_pools: usize,
    pub simulation_v3_candidate_pools: usize,
    pub simulation_v4_candidate_pools: usize,
    pub simulation_v2_current_block_pools: usize,
    pub simulation_v3_current_block_pools: usize,
    pub simulation_v4_current_block_pools: usize,
    pub simulated_v2_pools: usize,
    pub simulated_v3_pools: usize,
    pub simulated_v4_pools: usize,
}

impl ProcessedTokenUpdateProfile {
    fn record_candidate_address_profile(&mut self, profile: CandidateTokenAddressProfile) {
        self.candidate_routing_addresses_us += profile.routing_addresses_us;
        self.candidate_resolve_addresses_us += profile.resolve_addresses_us;
        self.candidate_v4_pool_keys_us += profile.v4_pool_keys_us;
        self.candidate_v3_position_transfer_us += profile.v3_position_transfer_us;
        self.candidate_v4_position_transfer_us += profile.v4_position_transfer_us;
        self.candidate_v4_position_approval_us += profile.v4_position_approval_us;
        self.candidate_v2_pair_created_us += profile.v2_pair_created_us;
        self.candidate_v2_pool_event_scan_us += profile.v2_pool_event_scan_us;
        self.candidate_v2_transfer_route_us += profile.v2_transfer_route_us;
        self.candidate_v2_cache_route_us += profile.v2_cache_route_us;
        self.candidate_v2_identity_lookup_us += profile.v2_identity_lookup_us;
        self.candidate_finalize_us += profile.finalize_us;
        self.candidate_routing_addresses += profile.routing_address_count;
        self.candidate_v4_pool_keys += profile.v4_pool_key_count;
        self.candidate_v2_pool_events += profile.v2_pool_event_count;
        self.candidate_v2_transfer_route_hits += profile.v2_transfer_route_hits;
        self.candidate_v2_identity_cache_hits += profile.v2_identity_cache_hits;
        self.candidate_v2_irrelevant_cache_hits += profile.v2_irrelevant_cache_hits;
        self.candidate_v2_irrelevant_cache_inserts += profile.v2_irrelevant_cache_inserts;
        self.candidate_v2_identity_lookups += profile.v2_identity_lookups;
        self.candidate_v2_identity_hits += profile.v2_identity_hits;
        self.candidate_v2_identity_skipped_by_transfer += profile.v2_identity_skipped_by_transfer;
        self.candidate_position_scan_tokens += profile.position_scan_tokens;
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum PendingPoolSimulationKind {
    V2,
    V3,
    V4,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct PendingPoolSimulationKey {
    pub token_address: String,
    pub pool_kind: PendingPoolSimulationKind,
    pub pool_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingPoolSimulation {
    pub key: PendingPoolSimulationKey,
    pub trigger_tx: ProcessedTransaction,
}

pub(crate) type PendingPoolSimulationMap =
    BTreeMap<PendingPoolSimulationKey, PendingPoolSimulation>;

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
        profile_run_id: Option<&'a str>,
    },
    HistoricalPostBlockSession {
        pool_simulator: &'a PoolBuySellSimulator,
        block_sessions: &'a Mutex<BTreeMap<u64, BlockStateSession>>,
        profile_run_id: Option<&'a str>,
    },
    LiveBlockSession {
        pool_simulator: &'a LivePoolBuySellSimulator,
        block_sessions: &'a Mutex<BTreeMap<u64, BlockStateSession>>,
        direct_state_only: bool,
        profile_run_id: Option<&'a str>,
    },
    Noop,
}

impl<'a> PoolTradingSimulationMode<'a> {
    fn uses_direct_live_state_only(self) -> bool {
        matches!(
            self,
            Self::LiveBlockSession {
                direct_state_only: true,
                ..
            }
        )
    }

    pub(crate) fn profile_run_id(self) -> Option<&'a str> {
        match self {
            Self::HistoricalBlockSession { profile_run_id, .. }
            | Self::HistoricalPostBlockSession { profile_run_id, .. }
            | Self::LiveBlockSession { profile_run_id, .. } => profile_run_id,
            Self::Historical(_) => None,
            Self::Noop => None,
        }
    }
}

pub(super) const LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS: u64 = 250;
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
                direct_state_only: true,
                profile_run_id: Some("live"),
            },
            None,
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
        mut pending_simulations: Option<&mut PendingPoolSimulationMap>,
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
            let simulated_v2 = if let Some(pending_simulations) = pending_simulations.as_deref_mut()
            {
                collect_pending_v2_pool_simulations(
                    pending_simulations,
                    &token_address,
                    token,
                    tx,
                    &simulation_v2_pool_addresses,
                    token_control_replay,
                );
                Vec::new()
            } else {
                simulate_updated_v2_pools(
                    token,
                    tx,
                    &simulation_v2_pool_addresses,
                    &current_block_v2_pool_addresses,
                    trading_simulation,
                    token_control_replay,
                    block_header,
                )
                .await?
            };
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
            let simulated_v3 = if let Some(pending_simulations) = pending_simulations.as_deref_mut()
            {
                collect_pending_v3_pool_simulations(
                    pending_simulations,
                    &token_address,
                    token,
                    tx,
                    &simulation_v3_pool_addresses,
                    token_control_replay,
                );
                Vec::new()
            } else {
                simulate_updated_v3_pools(
                    token,
                    tx,
                    &simulation_v3_pool_addresses,
                    &current_block_v3_pool_addresses,
                    trading_simulation,
                    token_control_replay,
                    block_header,
                )
                .await?
            };
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
            let simulated_v4 = if let Some(pending_simulations) = pending_simulations.as_deref_mut()
            {
                collect_pending_v4_pool_simulations(
                    pending_simulations,
                    &token_address,
                    token,
                    tx,
                    &simulation_v4_pool_keys,
                    token_control_replay,
                );
                Vec::new()
            } else {
                simulate_updated_v4_pools(
                    token,
                    tx,
                    &simulation_v4_pool_keys,
                    &current_block_v4_pool_keys,
                    trading_simulation,
                    token_control_replay,
                    block_header,
                )
                .await?
            };

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
        P: UniswapV2PoolIdentityProvider + UniswapV2PoolMetadataProvider,
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
        P: UniswapV2PoolIdentityProvider + UniswapV2PoolMetadataProvider,
    {
        self.update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            PoolTradingSimulationMode::Historical(pool_simulator),
            None,
            None,
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
        P: UniswapV2PoolIdentityProvider + UniswapV2PoolMetadataProvider,
    {
        self.update_registry_from_processed_transaction_with_discovery_and_trading_simulation(
            registry,
            token_index,
            tx,
            pool_metadata_provider,
            PoolTradingSimulationMode::LiveBlockSession {
                pool_simulator,
                block_sessions,
                direct_state_only: true,
                profile_run_id: Some("live"),
            },
            None,
            None,
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
        mut v2_candidate_cache: Option<&mut V2PoolCandidateCache>,
        mut pending_simulations: Option<&mut PendingPoolSimulationMap>,
    ) -> Result<Vec<TokenStateUpdateReport>>
    where
        P: UniswapV2PoolIdentityProvider + UniswapV2PoolMetadataProvider,
    {
        if !tx.status {
            return Ok(Vec::new());
        }

        let pool_metadata_timeout =
            pool_metadata_timeout_for_trading_simulation(trading_simulation);
        let candidate_started = Instant::now();
        let (token_addresses, candidate_address_profile) =
            candidate_token_addresses_with_pool_discovery_and_profile(
                registry,
                token_index,
                tx,
                pool_metadata_provider,
                pool_metadata_timeout,
                v2_candidate_cache.as_deref_mut(),
            )
            .await?;
        if let Some(profile) = profile.as_deref_mut() {
            profile.candidate_us += elapsed_micros(candidate_started);
            profile.record_candidate_address_profile(candidate_address_profile);
            profile.candidate_tx_count += 1;
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
                profile.token_state_us += elapsed_micros(token_state_started);
                if token_state_updated {
                    profile.token_state_updates += 1;
                }
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
                profile.pool_discovery_us += elapsed_micros(pool_discovery_started);
            }

            let pool_update_started = Instant::now();
            let updated_v2 = update_touched_v2_pools(token, tx)?;
            let updated_v3 = update_touched_v3_pools(token, tx)?;
            let updated_v4 = update_touched_v4_pools(token, tx)?;
            if let Some(profile) = profile.as_deref_mut() {
                profile.pool_update_us += elapsed_micros(pool_update_started);
            }

            let token_control_replay =
                token_state_updated && tx_is_token_control_replay_candidate(token, tx);
            if let Some(profile) = profile.as_deref_mut() {
                if token_control_replay {
                    profile.token_control_replays += 1;
                }
            }
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
            if let Some(profile) = profile.as_deref_mut() {
                profile.simulation_v2_candidate_pools += simulation_v2_pool_addresses.len();
                profile.simulation_v2_current_block_pools += current_block_v2_pool_addresses.len();
            }
            let simulated_v2 = if let Some(pending_simulations) = pending_simulations.as_deref_mut()
            {
                collect_pending_v2_pool_simulations(
                    pending_simulations,
                    &token_address,
                    token,
                    tx,
                    &simulation_v2_pool_addresses,
                    token_control_replay,
                );
                Vec::new()
            } else {
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
                    profile.simulation_v2_us += elapsed_micros(simulation_v2_started);
                    profile.simulated_v2_pools += simulated_v2.len();
                }
                simulated_v2
            };
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
            if let Some(profile) = profile.as_deref_mut() {
                profile.simulation_v3_candidate_pools += simulation_v3_pool_addresses.len();
                profile.simulation_v3_current_block_pools += current_block_v3_pool_addresses.len();
            }
            let simulated_v3 = if let Some(pending_simulations) = pending_simulations.as_deref_mut()
            {
                collect_pending_v3_pool_simulations(
                    pending_simulations,
                    &token_address,
                    token,
                    tx,
                    &simulation_v3_pool_addresses,
                    token_control_replay,
                );
                Vec::new()
            } else {
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
                    profile.simulation_v3_us += elapsed_micros(simulation_v3_started);
                    profile.simulated_v3_pools += simulated_v3.len();
                }
                simulated_v3
            };
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
            if let Some(profile) = profile.as_deref_mut() {
                profile.simulation_v4_candidate_pools += simulation_v4_pool_keys.len();
                profile.simulation_v4_current_block_pools += current_block_v4_pool_keys.len();
            }
            let simulated_v4 = if let Some(pending_simulations) = pending_simulations.as_deref_mut()
            {
                collect_pending_v4_pool_simulations(
                    pending_simulations,
                    &token_address,
                    token,
                    tx,
                    &simulation_v4_pool_keys,
                    token_control_replay,
                );
                Vec::new()
            } else {
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
                    profile.simulation_v4_us += elapsed_micros(simulation_v4_started);
                    profile.simulated_v4_pools += simulated_v4.len();
                }
                simulated_v4
            };

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
                profile.report_us += elapsed_micros(report_started);
            }
        }

        Ok(reports)
    }

    pub(crate) async fn simulate_pending_pools_after_block(
        &self,
        registry: &mut TokenRegistry,
        pending_simulations: &PendingPoolSimulationMap,
        pool_simulator: &PoolBuySellSimulator,
        block_header: &BlockHeader,
        mut profile: Option<&mut ProcessedTokenUpdateProfile>,
        profile_run_id: Option<&str>,
    ) -> Result<Vec<TokenStateUpdateReport>> {
        if pending_simulations.is_empty() {
            return Ok(Vec::new());
        }

        let block_sessions: Mutex<BTreeMap<u64, BlockStateSession>> = Mutex::new(BTreeMap::new());
        let trading_simulation = PoolTradingSimulationMode::HistoricalPostBlockSession {
            pool_simulator,
            block_sessions: &block_sessions,
            profile_run_id,
        };
        let mut reports: BTreeMap<String, TokenStateUpdateReport> = BTreeMap::new();

        for pending in pending_simulations.values() {
            let token_address = pending.key.token_address.clone();
            let pool_id = pending.key.pool_id.clone();
            let Some(token) = registry.token_mut(&token_address) else {
                continue;
            };

            match pending.key.pool_kind {
                PendingPoolSimulationKind::V2 => {
                    let simulation_started = Instant::now();
                    let simulated = simulate_updated_v2_pools(
                        token,
                        &pending.trigger_tx,
                        std::slice::from_ref(&pool_id),
                        std::slice::from_ref(&pool_id),
                        trading_simulation,
                        true,
                        Some(block_header),
                    )
                    .await?;
                    if let Some(profile) = profile.as_deref_mut() {
                        profile.simulation_v2_us += elapsed_micros(simulation_started);
                        profile.simulated_v2_pools += simulated.len();
                    }
                    if !simulated.is_empty() {
                        reports
                            .entry(token_address.clone())
                            .or_insert_with(|| TokenStateUpdateReport {
                                token_address: token_address.clone(),
                                ..Default::default()
                            })
                            .simulated_known_v2_pools
                            .extend(simulated);
                    }
                }
                PendingPoolSimulationKind::V3 => {
                    let simulation_started = Instant::now();
                    let simulated = simulate_updated_v3_pools(
                        token,
                        &pending.trigger_tx,
                        std::slice::from_ref(&pool_id),
                        std::slice::from_ref(&pool_id),
                        trading_simulation,
                        true,
                        Some(block_header),
                    )
                    .await?;
                    if let Some(profile) = profile.as_deref_mut() {
                        profile.simulation_v3_us += elapsed_micros(simulation_started);
                        profile.simulated_v3_pools += simulated.len();
                    }
                    if !simulated.is_empty() {
                        reports
                            .entry(token_address.clone())
                            .or_insert_with(|| TokenStateUpdateReport {
                                token_address: token_address.clone(),
                                ..Default::default()
                            })
                            .simulated_uniswap_v3_pools
                            .extend(simulated);
                    }
                }
                PendingPoolSimulationKind::V4 => {
                    let simulation_started = Instant::now();
                    let simulated = simulate_updated_v4_pools(
                        token,
                        &pending.trigger_tx,
                        std::slice::from_ref(&pool_id),
                        std::slice::from_ref(&pool_id),
                        trading_simulation,
                        true,
                        Some(block_header),
                    )
                    .await?;
                    if let Some(profile) = profile.as_deref_mut() {
                        profile.simulation_v4_us += elapsed_micros(simulation_started);
                        profile.simulated_v4_pools += simulated.len();
                    }
                    if !simulated.is_empty() {
                        reports
                            .entry(token_address.clone())
                            .or_insert_with(|| TokenStateUpdateReport {
                                token_address: token_address.clone(),
                                ..Default::default()
                            })
                            .simulated_uniswap_v4_pools
                            .extend(simulated);
                    }
                }
            }
        }

        let mut reports: Vec<_> = reports.into_values().collect();
        for report in &mut reports {
            report.simulated_known_v2_pools.sort();
            report.simulated_known_v2_pools.dedup();
            report.simulated_uniswap_v3_pools.sort();
            report.simulated_uniswap_v3_pools.dedup();
            report.simulated_uniswap_v4_pools.sort();
            report.simulated_uniswap_v4_pools.dedup();
        }
        if let Some(profile) = profile.as_deref_mut() {
            profile.update_reports += reports.len();
        }
        Ok(reports)
    }
}

fn collect_pending_v2_pool_simulations(
    pending_simulations: &mut PendingPoolSimulationMap,
    token_address: &str,
    token: &crate::erc20::ERC20Token,
    tx: &ProcessedTransaction,
    pool_addresses: &[String],
    force_simulation: bool,
) {
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v2_pool(pool_address)
            .map(|pool| {
                !pool.base.has_liquidity_removal()
                    && (force_simulation || should_simulate_v2_trading(pool, tx))
            })
            .unwrap_or(false);
        if should_simulate {
            collect_pending_pool_simulation(
                pending_simulations,
                token_address,
                PendingPoolSimulationKind::V2,
                pool_address,
                tx,
            );
        }
    }
}

fn collect_pending_v3_pool_simulations(
    pending_simulations: &mut PendingPoolSimulationMap,
    token_address: &str,
    token: &crate::erc20::ERC20Token,
    tx: &ProcessedTransaction,
    pool_addresses: &[String],
    force_simulation: bool,
) {
    for pool_address in pool_addresses {
        let should_simulate = token
            .uniswap_v3_pool(pool_address)
            .map(|pool| {
                !pool.base.has_liquidity_removal()
                    && pool.supports_trading_simulation()
                    && (force_simulation || should_simulate_v3_trading(pool, tx))
            })
            .unwrap_or(false);
        if should_simulate {
            collect_pending_pool_simulation(
                pending_simulations,
                token_address,
                PendingPoolSimulationKind::V3,
                pool_address,
                tx,
            );
        }
    }
}

fn collect_pending_v4_pool_simulations(
    pending_simulations: &mut PendingPoolSimulationMap,
    token_address: &str,
    token: &crate::erc20::ERC20Token,
    tx: &ProcessedTransaction,
    pool_keys: &[String],
    force_simulation: bool,
) {
    for pool_key in pool_keys {
        let should_simulate = token
            .uniswap_v4_pool(pool_key)
            .map(|pool| {
                !pool.base.has_liquidity_removal()
                    && (force_simulation || should_simulate_v4_trading(pool, tx))
            })
            .unwrap_or(false);
        if should_simulate {
            collect_pending_pool_simulation(
                pending_simulations,
                token_address,
                PendingPoolSimulationKind::V4,
                pool_key,
                tx,
            );
        }
    }
}

fn collect_pending_pool_simulation(
    pending_simulations: &mut PendingPoolSimulationMap,
    token_address: &str,
    pool_kind: PendingPoolSimulationKind,
    pool_id: &str,
    tx: &ProcessedTransaction,
) {
    let key = PendingPoolSimulationKey {
        token_address: normalize_address_string(token_address),
        pool_kind,
        pool_id: normalize_address_string(pool_id),
    };
    pending_simulations.insert(
        key.clone(),
        PendingPoolSimulation {
            key,
            trigger_tx: tx.clone(),
        },
    );
}

fn elapsed_micros(started: Instant) -> u128 {
    started.elapsed().as_micros()
}

fn pool_metadata_timeout_for_trading_simulation(
    trading_simulation: PoolTradingSimulationMode<'_>,
) -> Option<Duration> {
    if trading_simulation.uses_direct_live_state_only() {
        Some(Duration::from_millis(LIVE_POOL_METADATA_LOOKUP_TIMEOUT_MS))
    } else {
        None
    }
}
