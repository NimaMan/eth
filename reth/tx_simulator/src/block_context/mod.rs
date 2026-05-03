//! Shared helpers for loading block headers and state across MDBX/Redis sources.
//!
//! The goal of this module is to centralize the logic required to hydrate block
//! context regardless of whether the local MDBX database has indexed the block
//! yet. Callers that need headers or state at a specific block should rely on
//! [`BlockContextLoader`] instead of open-coding MDBX/Redis fallbacks.

pub mod live_chain_cache;
pub mod live_data_registry;

/// Default Redis URL used for live chain data snapshots when no environment
/// override is provided.
pub const DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL: &str = "redis://localhost:6379/0";

/// Resolve the Redis URL from `LIVE_BLOCKCHAIN_DATA_REDIS_URL`, falling back to
/// [`DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL`] when unset.
pub fn resolve_live_data_redis_url() -> String {
    std::env::var("LIVE_BLOCKCHAIN_DATA_REDIS_URL")
        .unwrap_or_else(|_| DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string())
}

use self::live_chain_cache::LiveChainCache;
use crate::{
    config::view_call::{STATE_RETRY_DELAY_MS, STATE_RETRY_MAX_ATTEMPTS},
    header_utils::parse_sealed_header_from_json,
    single_tx::unsigned::UnsignedTransaction,
    tx_builders::processed_tx_json_unsigned_builder::build_unsigned_transaction_from_processed_tx_json,
    tx_chain::{sequential::ForkedState, unsigned::UnsignedTxChainSimulation},
    TxSimulator,
};
use eyre::{eyre, Result};
use reth_primitives_traits::SealedHeader;
use reth_provider::{HeaderProvider, StateProviderBox};
use serde_json::Value;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
/// Block state returned by [`BlockContextLoader`].
pub(crate) enum BlockStateProvider {
    /// State is available directly from MDBX.
    Historical(StateProviderBox),
    /// State was reconstructed by replaying live data into a forked cache DB.
    LiveFork(ForkedState),
}

/// Complete block context (header + state source).
pub(crate) struct BlockContext {
    pub header: SealedHeader,
    pub state: BlockStateProvider,
}

/// Convenience wrapper around the shared header/state loader logic.
pub struct BlockContextLoader<'a> {
    simulator: &'a TxSimulator,
}

impl<'a> BlockContextLoader<'a> {
    pub fn new(simulator: &'a TxSimulator) -> Self {
        Self { simulator }
    }

    /// Load block header + state, replaying live data when MDBX lags the requested block.
    pub(crate) async fn load_block_context(
        &self,
        block_number: u64,
        header_hint: Option<SealedHeader>,
    ) -> Result<BlockContext> {
        let header = self.load_block_header(block_number, header_hint).await?;
        let latest_persisted = self.simulator.get_latest_block()?;

        if block_number <= latest_persisted {
            let state = self.load_historical_state(block_number).await?;
            return Ok(BlockContext {
                header,
                state: BlockStateProvider::Historical(state),
            });
        }

        let forked = self
            .replay_live_state(block_number, Some(header.clone()))
            .await?
            .ok_or_else(|| eyre!("live data for block {} is unavailable", block_number))?;

        Ok(BlockContext {
            header,
            state: BlockStateProvider::LiveFork(forked),
        })
    }

    /// Attempt to replay the requested block purely from live data.
    pub(crate) async fn replay_live_state(
        &self,
        block_number: u64,
        header_hint: Option<SealedHeader>,
    ) -> Result<Option<ForkedState>> {
        let cache = match self.simulator.live_chain_cache() {
            Some(cache) => cache,
            None => return Ok(None),
        };

        let Some(latest_live) = cache.latest_block_number().await? else {
            return Ok(None);
        };
        if block_number > latest_live {
            return Ok(None);
        }

        let header = self.load_block_header(block_number, header_hint).await?;
        let persisted = self.simulator.get_latest_block()?;
        if block_number <= persisted {
            return Ok(None);
        }

        let fork = self
            .reconstruct_state_from_live_data(cache, persisted, block_number, header.clone())
            .await?;
        Ok(Some(fork))
    }

    /// Resolve a sealed header for a specific block, falling back to live cache.
    pub async fn load_block_header(
        &self,
        block_number: u64,
        header_hint: Option<SealedHeader>,
    ) -> Result<SealedHeader> {
        if let Some(header) = header_hint {
            return Ok(header);
        }

        if let Some(header) = self.fetch_header_from_mdbx(block_number)? {
            return Ok(header);
        }

        self.fetch_header_from_live_cache(block_number).await
    }

    fn fetch_header_from_mdbx(&self, block_number: u64) -> Result<Option<SealedHeader>> {
        let provider = self.simulator.provider_factory.provider()?;
        let maybe_header = provider.header_by_number(block_number)?;
        Ok(maybe_header.map(SealedHeader::new_unhashed))
    }

    async fn fetch_header_from_live_cache(&self, block_number: u64) -> Result<SealedHeader> {
        let cache = self
            .simulator
            .live_chain_cache()
            .ok_or_else(|| eyre!("live chain cache not configured"))?;
        let payload = match cache.fetch_block_header(block_number).await? {
            Some(payload) => payload,
            None => {
                let latest_live = cache.latest_block_number().await.ok().flatten();
                let available_blocks = cache.recent_block_numbers(5).await.unwrap_or_default();
                let latest_persisted = self.simulator.get_latest_block().ok();
                return Err(eyre!(
                    "missing live block header for {} (latest live {:?}, available {:?}, latest persisted {:?})",
                    block_number,
                    latest_live,
                    available_blocks,
                    latest_persisted
                ));
            }
        };
        parse_sealed_header_from_json(&payload)
    }

    async fn load_historical_state(&self, block_number: u64) -> Result<StateProviderBox> {
        let retry_delay = Duration::from_millis(STATE_RETRY_DELAY_MS);
        for attempt in 1..=STATE_RETRY_MAX_ATTEMPTS {
            let simulator = self.simulator.clone();
            match tokio::task::spawn_blocking(move || {
                simulator
                    .provider_factory
                    .history_by_block_number(block_number)
            })
            .await
            {
                Ok(Ok(state)) => return Ok(state),
                Ok(Err(err)) => {
                    if attempt == STATE_RETRY_MAX_ATTEMPTS {
                        return Err(eyre!(
                            "failed to fetch historical state for block {}: {}",
                            block_number,
                            err
                        ));
                    }
                }
                Err(join_err) => {
                    if attempt == STATE_RETRY_MAX_ATTEMPTS {
                        return Err(eyre!(
                            "state fetch task panicked for block {}: {}",
                            block_number,
                            join_err
                        ));
                    }
                }
            }
            sleep(retry_delay).await;
        }
        Err(eyre!(
            "exhausted retries fetching state provider for block {}",
            block_number
        ))
    }

    async fn reconstruct_state_from_live_data(
        &self,
        cache: Arc<LiveChainCache>,
        persisted_block: u64,
        target_block: u64,
        final_header: SealedHeader,
    ) -> Result<ForkedState> {
        if target_block <= persisted_block {
            return Err(eyre!(
                "target block {} already persisted (latest {})",
                target_block,
                persisted_block
            ));
        }

        self.simulator.assert_block_available(persisted_block)?;
        let mut fork_state = self.simulator.create_forked_state(persisted_block)?;
        let simulator = Arc::new(self.simulator.clone());

        let mut current = persisted_block;
        while current < target_block {
            let next_block = current + 1;
            let header = if next_block == final_header.number {
                final_header.clone()
            } else {
                self.fetch_header_from_live_cache(next_block).await?
            };

            fork_state.block_number = header.number;
            fork_state.block_header = header.clone();
            fork_state.nonces.clear();

            let raw = cache
                .fetch_processed_block(next_block)
                .await?
                .ok_or_else(|| {
                    eyre!(
                        "missing processed transactions for block {} while replaying {}->{}; this block must be present in the live Redis cache",
                        next_block,
                        persisted_block,
                        target_block
                    )
                })?;
            let transactions = decode_processed_transactions(&raw)?;
            if transactions.is_empty() {
                current = next_block;
                continue;
            }

            let mut chain = UnsignedTxChainSimulation::new(simulator.clone(), fork_state);
            for tx in transactions {
                chain.step(tx).await?;
            }
            fork_state = chain.into_forked_state();
            current = next_block;
        }

        Ok(fork_state)
    }
}

/// Convert stored processed transaction JSON into [`UnsignedTransaction`] values.
fn decode_processed_transactions(values: &[Value]) -> Result<Vec<UnsignedTransaction>> {
    let mut txs = Vec::with_capacity(values.len());
    for value in values {
        txs.push(build_unsigned_transaction_from_processed_tx_json(value)?);
    }
    Ok(txs)
}
