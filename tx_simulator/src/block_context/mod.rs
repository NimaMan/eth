//! Shared helpers for loading block headers and state across MDBX/Redis sources.
//!
//! The goal of this module is to centralize the logic required to hydrate block
//! context regardless of whether the local MDBX database has indexed the block
//! yet. Callers that need headers or state at a specific block should rely on
//! [`BlockContextLoader`] instead of open-coding MDBX/Redis fallbacks.

pub mod header_json;
pub mod live_chain_cache;
pub mod live_data_registry;
pub mod processed_tx_json;

/// Default Redis URL used for live chain data snapshots when no environment
/// override is provided.
pub const DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL: &str = "redis://localhost:6379/0";

/// Resolve the Redis URL from env/config, falling back to
/// [`DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL`] when unset.
pub fn resolve_live_data_redis_url() -> String {
    crate::config::repo::live_data_redis_url()
        .unwrap_or_else(|_| DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL.to_string())
}

use crate::{
    config::view_call::{STATE_RETRY_DELAY_MS, STATE_RETRY_MAX_ATTEMPTS},
    single_tx::unsigned::UnsignedTransaction,
    tx_chain::{
        sequential::{ForkedState, SharedStateProvider},
        unsigned::UnsignedTxChainSimulation,
    },
    TxSimulator,
};
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types_trace::geth::{AccountState as PreStateAccountState, DiffMode, PreStateFrame};
use eyre::{eyre, Result};
use reth_primitives_traits::SealedHeader;
use reth_provider::{HeaderProvider, StateProviderBox};
use reth_revm::{
    database::StateProviderDatabase,
    db::{AccountState as RevmAccountState, Cache, CacheDB, DbAccount},
    Database,
};
use revm::bytecode::Bytecode;
use serde_json::Value;
use std::{collections::HashSet, sync::Arc};
use tokio::time::{sleep, Duration};
use tracing::{debug, warn};

use self::header_json::parse_sealed_header_from_json;
use self::live_data_registry::ChainStateSnapshot;
use self::processed_tx_json::build_unsigned_transaction_from_processed_tx_json;
/// Block state returned by [`BlockContextLoader`].
pub(crate) enum BlockStateProvider {
    /// State is available directly from MDBX.
    Historical(StateProviderBox),
    /// State was restored from the live block processor's tracked state.
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

    /// Load block header + state, replaying live data when local historical context lags.
    pub(crate) async fn load_block_context(
        &self,
        block_number: u64,
        header_hint: Option<SealedHeader>,
    ) -> Result<BlockContext> {
        let header = self.load_block_header(block_number, header_hint).await?;

        if let Some(state) = self.try_load_historical_state(block_number).await? {
            return Ok(BlockContext {
                header,
                state: BlockStateProvider::Historical(state),
            });
        }

        let forked = self
            .replay_live_state(block_number, Some(header.clone()))
            .await?
            .ok_or_else(|| {
                eyre!(
                    "state for block {} is unavailable from both Reth historical state and Redis live state",
                    block_number
                )
            })?;

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
        let Some(cache) = self.simulator.live_chain_cache() else {
            return Ok(None);
        };

        let header = self.load_block_header(block_number, header_hint).await?;
        match cache.fetch_chain_state_snapshot(block_number).await? {
            Some(snapshot) if snapshot.block_hash == header.hash() => {
                debug!(
                    block_number,
                    base_block_number = snapshot.base_block_number,
                    accounts = snapshot.account_count(),
                    contracts = snapshot.contract_count(),
                    "restoring tracked live state"
                );
                Ok(Some(
                    self.forked_state_from_snapshot(&snapshot, header).await?,
                ))
            }
            Some(snapshot) => {
                warn!(
                    block_number,
                    expected = %header.hash(),
                    found = %snapshot.block_hash,
                    "ignoring tracked live state with mismatched block hash"
                );
                Ok(None)
            }
            None => Ok(None),
        }
    }

    async fn fetch_required_live_state_snapshot(
        &self,
        block_number: u64,
    ) -> Result<ChainStateSnapshot> {
        let cache = match self.simulator.live_chain_cache() {
            Some(cache) => cache,
            None => {
                return Err(eyre!(
                    "state for block {} is unavailable from Reth DB and Redis live state is not configured",
                    block_number
                ))
            }
        };

        match cache.fetch_chain_state_snapshot(block_number).await? {
            Some(snapshot) if snapshot.block_number == block_number => {
                validate_live_state_snapshot_schema(&snapshot)?;
                Ok(snapshot)
            }
            Some(snapshot) => Err(eyre!(
                "Redis live state snapshot key for block {} contained snapshot for block {}",
                block_number,
                snapshot.block_number
            )),
            None => {
                let latest_live = cache.latest_block_number().await.ok().flatten();
                let latest_chain_state =
                    cache.latest_chain_state_block_number().await.ok().flatten();
                let available_blocks = cache.recent_block_numbers(10).await.unwrap_or_default();
                Err(eyre!(
                    "state for block {} is unavailable from Reth DB and Redis live state snapshot is missing (latest live {:?}, latest chain state {:?}, recent Redis blocks {:?})",
                    block_number,
                    latest_live,
                    latest_chain_state,
                    available_blocks
                ))
            }
        }
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
        self.simulator.refresh_static_file_provider()?;
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
                if let Some(header) = self.fetch_header_from_mdbx(block_number)? {
                    return Ok(header);
                }
                let latest_live = cache.latest_block_number().await.ok().flatten();
                let available_blocks = cache.recent_block_numbers(5).await.unwrap_or_default();
                let latest_reth_finished = self.simulator.get_latest_block().ok();
                let latest_historical_context =
                    self.simulator.latest_historical_context_block_number().ok();
                return Err(eyre!(
                    "missing live block header for {} (latest live {:?}, available {:?}, latest reth finished {:?}, latest historical context {:?})",
                    block_number,
                    latest_live,
                    available_blocks,
                    latest_reth_finished,
                    latest_historical_context
                ));
            }
        };
        parse_sealed_header_from_json(&payload)
    }

    async fn try_load_historical_state(
        &self,
        block_number: u64,
    ) -> Result<Option<StateProviderBox>> {
        let latest_reth_finished = self.simulator.get_latest_block()?;
        if !historical_state_available_from_reth(block_number, latest_reth_finished) {
            debug!(
                block_number,
                latest_reth_finished,
                "Reth historical state is behind requested block; using Redis live state"
            );
            return Ok(None);
        }

        let retry_delay = Duration::from_millis(STATE_RETRY_DELAY_MS);
        let mut last_error = None;
        for attempt in 1..=STATE_RETRY_MAX_ATTEMPTS {
            let simulator = self.simulator.clone();
            match tokio::task::spawn_blocking(move || {
                simulator
                    .provider_factory
                    .caught_up_static_file_provider()?;
                simulator
                    .provider_factory
                    .history_by_block_number(block_number)
            })
            .await
            {
                Ok(Ok(state)) => return Ok(Some(state)),
                Ok(Err(err)) => {
                    last_error = Some(err.to_string());
                }
                Err(join_err) => {
                    last_error = Some(format!("state fetch task panicked: {join_err}"));
                }
            }
            if attempt < STATE_RETRY_MAX_ATTEMPTS {
                sleep(retry_delay).await;
            }
        }
        debug!(
            block_number,
            last_error = ?last_error,
            "Reth historical state unavailable; trying Redis live state"
        );
        Ok(None)
    }

    async fn build_live_state_snapshot_from_processed_payloads(
        &self,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        header_payload: &str,
        tx_payloads: &[&str],
    ) -> Result<ChainStateSnapshot> {
        let header = parse_sealed_header_from_json(header_payload)?;
        if header.number != block_number {
            return Err(eyre!(
                "live state snapshot header block mismatch: header={}, expected={}",
                header.number,
                block_number
            ));
        }
        if header.hash() != block_hash {
            return Err(eyre!(
                "live state snapshot header hash mismatch for block {}: header={}, expected={}",
                block_number,
                header.hash(),
                block_hash
            ));
        }
        if header.parent_hash != parent_hash {
            return Err(eyre!(
                "live state snapshot parent hash mismatch for block {}: header={}, expected={}",
                block_number,
                header.parent_hash,
                parent_hash
            ));
        }

        let (mut fork_state, base_block_number) = self
            .load_parent_state_for_live_snapshot(block_number, parent_hash)
            .await?;
        fork_state.block_number = header.number;
        fork_state.block_header = header;
        fork_state.nonces.clear();

        let transactions = decode_processed_transaction_payloads(tx_payloads)?;
        if !transactions.is_empty() {
            let simulator = Arc::new(self.simulator.clone());
            let mut chain = UnsignedTxChainSimulation::new(simulator, fork_state);
            for tx in transactions {
                chain.step(tx).await?;
            }
            fork_state = chain.into_forked_state();
        }

        Ok(ChainStateSnapshot::new(
            base_block_number,
            block_number,
            block_hash,
            parent_hash,
            fork_state.db.cache.clone(),
        ))
    }

    async fn build_live_state_snapshot_from_prestate_diffs(
        &self,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        header_payload: &str,
        state_diffs: &[PreStateFrame],
    ) -> Result<ChainStateSnapshot> {
        let header = parse_sealed_header_from_json(header_payload)?;
        if header.number != block_number {
            return Err(eyre!(
                "live state snapshot header block mismatch: header={}, expected={}",
                header.number,
                block_number
            ));
        }
        if header.hash() != block_hash {
            return Err(eyre!(
                "live state snapshot header hash mismatch for block {}: header={}, expected={}",
                block_number,
                header.hash(),
                block_hash
            ));
        }
        if header.parent_hash != parent_hash {
            return Err(eyre!(
                "live state snapshot parent hash mismatch for block {}: header={}, expected={}",
                block_number,
                header.parent_hash,
                parent_hash
            ));
        }

        let (mut fork_state, base_block_number) = self
            .load_parent_state_for_live_snapshot(block_number, parent_hash)
            .await?;
        fork_state.block_number = header.number;
        fork_state.block_header = header;
        fork_state.nonces.clear();

        for (tx_index, frame) in state_diffs.iter().enumerate() {
            let diff = frame.as_diff().ok_or_else(|| {
                eyre!(
                    "live state snapshot diff for block {} tx index {} was not diffMode",
                    block_number,
                    tx_index
                )
            })?;
            apply_prestate_diff(&mut fork_state, diff)?;
        }

        Ok(ChainStateSnapshot::new(
            base_block_number,
            block_number,
            block_hash,
            parent_hash,
            fork_state.db.cache.clone(),
        ))
    }

    async fn load_parent_state_for_live_snapshot(
        &self,
        block_number: u64,
        parent_hash: B256,
    ) -> Result<(ForkedState, u64)> {
        let parent_block = block_number
            .checked_sub(1)
            .ok_or_else(|| eyre!("cannot build live state snapshot for genesis block"))?;
        let live_cache = self.simulator.live_chain_cache();
        if let Some(cache) = live_cache.as_ref() {
            if let Some(snapshot) = cache.fetch_chain_state_snapshot(parent_block).await? {
                if snapshot.block_hash == parent_hash {
                    debug!(
                        block_number,
                        parent_block,
                        snapshot_base_block_number = snapshot.base_block_number,
                        "using parent tracked live state as base for next live state snapshot"
                    );

                    let header = self.load_block_header(parent_block, None).await?;
                    let base_block_number = snapshot.base_block_number;
                    let fork_state = self.forked_state_from_snapshot(&snapshot, header).await?;
                    return Ok((fork_state, base_block_number));
                } else {
                    warn!(
                        block_number,
                        parent_block,
                        expected = %parent_hash,
                        found = %snapshot.block_hash,
                        "ignoring parent tracked live state with mismatched block hash"
                    );
                }
            }
        }

        if let Some(fork) = self
            .forked_state_from_reth_historical_state(parent_block, None)
            .await?
        {
            return Ok((fork, parent_block));
        }

        if live_cache.is_some() {
            return Err(eyre!(
                "cannot build live state snapshot for block {}: parent {} is unavailable from both Reth historical state and Redis live state",
                block_number,
                parent_block
            ));
        }

        Err(eyre!(
            "cannot build live state snapshot for block {}: parent {} is unavailable from Reth historical state and Redis live state is not configured",
            block_number,
            parent_block
        ))
    }

    async fn forked_state_from_snapshot(
        &self,
        snapshot: &ChainStateSnapshot,
        header: SealedHeader,
    ) -> Result<ForkedState> {
        validate_live_state_snapshot_schema(snapshot)?;
        let mut fork_state = self
            .restore_snapshot_base_from_reth_or_redis(snapshot, header.clone())
            .await?;
        fork_state.block_number = snapshot.block_number;
        fork_state.block_header = header;
        fork_state.nonces.clear();
        Ok(fork_state)
    }

    async fn restore_snapshot_base_from_reth_or_redis(
        &self,
        snapshot: &ChainStateSnapshot,
        final_header: SealedHeader,
    ) -> Result<ForkedState> {
        let mut snapshots = vec![snapshot.clone()];
        let mut base_block_number = snapshot.base_block_number;
        let mut visited = HashSet::new();

        loop {
            if let Some(mut fork_state) = self
                .forked_state_from_reth_historical_state(
                    base_block_number,
                    Some(final_header.clone()),
                )
                .await?
            {
                for snapshot in snapshots.iter().rev() {
                    merge_live_snapshot_cache(&mut fork_state.db.cache, &snapshot.cache);
                    fork_state.block_number = snapshot.block_number;
                }
                return Ok(fork_state);
            }

            if !visited.insert(base_block_number) {
                return Err(eyre!(
                    "cannot restore live state snapshot for block {}: Redis live state snapshot chain loops at base block {}",
                    snapshot.block_number,
                    base_block_number
                ));
            }

            let base_snapshot = self
                .fetch_required_live_state_snapshot(base_block_number)
                .await?;
            if base_snapshot.base_block_number == base_snapshot.block_number {
                return Err(eyre!(
                    "cannot restore live state snapshot for block {}: Redis live state snapshot for block {} uses itself as base",
                    snapshot.block_number,
                    base_snapshot.block_number
                ));
            }
            debug!(
                snapshot_block = snapshot.block_number,
                redis_base_block = base_snapshot.block_number,
                redis_base_of_base = base_snapshot.base_block_number,
                "using Redis live state snapshot as base"
            );
            base_block_number = base_snapshot.base_block_number;
            snapshots.push(base_snapshot);
        }
    }

    async fn forked_state_from_reth_historical_state(
        &self,
        block_number: u64,
        header_hint: Option<SealedHeader>,
    ) -> Result<Option<ForkedState>> {
        let Some(state) = self.try_load_historical_state(block_number).await? else {
            return Ok(None);
        };
        let header = match header_hint {
            Some(header) => header,
            None => self.load_block_header(block_number, None).await?,
        };
        Ok(Some(forked_state_from_historical_state(
            block_number,
            header,
            state,
        )))
    }
}

fn validate_live_state_snapshot_schema(snapshot: &ChainStateSnapshot) -> Result<()> {
    if snapshot.schema_version != ChainStateSnapshot::SCHEMA_VERSION {
        return Err(eyre!(
            "unsupported live state snapshot schema {} for block {}",
            snapshot.schema_version,
            snapshot.block_number
        ));
    }
    Ok(())
}

fn historical_state_available_from_reth(block_number: u64, latest_reth_finished: u64) -> bool {
    block_number <= latest_reth_finished
}

fn forked_state_from_historical_state(
    block_number: u64,
    block_header: SealedHeader,
    state: StateProviderBox,
) -> ForkedState {
    let db = CacheDB::new(StateProviderDatabase::new(SharedStateProvider::new(state)));
    ForkedState {
        db,
        block_number,
        block_header,
        nonces: Default::default(),
    }
}

fn merge_live_snapshot_cache(base: &mut Cache, overlay: &Cache) {
    base.accounts.extend(overlay.accounts.clone());
    base.contracts.extend(overlay.contracts.clone());
    base.logs.extend(overlay.logs.clone());
    base.block_hashes.extend(overlay.block_hashes.clone());
}

/// Convert stored processed transaction JSON strings into [`UnsignedTransaction`] values.
fn decode_processed_transaction_payloads(payloads: &[&str]) -> Result<Vec<UnsignedTransaction>> {
    let mut txs = Vec::with_capacity(payloads.len());
    for payload in payloads {
        let value: Value = serde_json::from_str(payload)
            .map_err(|err| eyre!("failed to decode processed tx payload: {}", err))?;
        txs.push(build_unsigned_transaction_from_processed_tx_json(&value)?);
    }
    Ok(txs)
}

fn apply_prestate_diff(fork_state: &mut ForkedState, diff: &DiffMode) -> Result<()> {
    for address in diff
        .pre
        .keys()
        .filter(|address| !diff.post.contains_key(*address))
    {
        mark_account_not_existing(fork_state, *address);
    }

    for (address, post_state) in &diff.post {
        let created_in_tx = !diff.pre.contains_key(address);
        apply_post_state_to_account(
            fork_state,
            *address,
            diff.pre.get(address),
            post_state,
            created_in_tx,
        )?;
    }

    Ok(())
}

fn mark_account_not_existing(fork_state: &mut ForkedState, address: Address) {
    fork_state
        .db
        .cache
        .accounts
        .insert(address, DbAccount::new_not_existing());
    fork_state.nonces.remove(&address);
}

fn apply_post_state_to_account(
    fork_state: &mut ForkedState,
    address: Address,
    pre_state: Option<&PreStateAccountState>,
    post_state: &PreStateAccountState,
    created_in_tx: bool,
) -> Result<()> {
    let mut info = fork_state.db.basic(address)?.unwrap_or_default();

    if let Some(balance) = post_state.balance {
        info.balance = balance;
    }
    if let Some(nonce) = post_state.nonce {
        info.nonce = nonce;
    }
    if let Some(code) = post_state.code.as_ref() {
        let bytecode = Bytecode::new_raw(code.clone());
        info.code_hash = bytecode.hash_slow();
        info.code = Some(bytecode);
    }

    fork_state.db.insert_account_info(address, info);
    if created_in_tx {
        if let Some(account) = fork_state.db.cache.accounts.get_mut(&address) {
            account.update_account_state(RevmAccountState::StorageCleared);
        }
    }

    if let Some(pre_state) = pre_state {
        for slot in removed_storage_slots(pre_state, post_state) {
            let slot: U256 = slot.into();
            fork_state
                .db
                .insert_account_storage(address, slot, U256::ZERO)
                .map_err(|err| eyre!("failed to clear storage diff for {address}: {err:?}"))?;
        }
    }

    for (slot, value) in &post_state.storage {
        let slot: U256 = (*slot).into();
        let value: U256 = (*value).into();
        fork_state
            .db
            .insert_account_storage(address, slot, value)
            .map_err(|err| eyre!("failed to apply storage diff for {address}: {err:?}"))?;
    }

    fork_state.nonces.remove(&address);
    Ok(())
}

fn removed_storage_slots(
    pre_state: &PreStateAccountState,
    post_state: &PreStateAccountState,
) -> Vec<B256> {
    pre_state
        .storage
        .keys()
        .filter(|slot| !post_state.storage.contains_key(*slot))
        .copied()
        .collect()
}

impl TxSimulator {
    /// Build tracked live state for a processed block.
    ///
    /// The returned snapshot stores only REVM's fork cache on top of a persisted
    /// base block. The live block processor writes this into Redis so later
    /// simulations can restore recent live state without replaying the whole
    /// Redis window on every call.
    pub async fn build_live_state_snapshot_from_processed_payloads(
        &self,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        header_payload: &str,
        tx_payloads: &[&str],
    ) -> Result<ChainStateSnapshot> {
        self.block_context_loader()
            .build_live_state_snapshot_from_processed_payloads(
                block_number,
                block_hash,
                parent_hash,
                header_payload,
                tx_payloads,
            )
            .await
    }

    /// Build tracked live state for a block from exact prestate
    /// diff traces (`prestateTracer` with `diffMode=true`).
    pub async fn build_live_state_snapshot_from_prestate_diffs(
        &self,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        header_payload: &str,
        state_diffs: &[PreStateFrame],
    ) -> Result<ChainStateSnapshot> {
        self.block_context_loader()
            .build_live_state_snapshot_from_prestate_diffs(
                block_number,
                block_hash,
                parent_hash,
                header_payload,
                state_diffs,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::b256;

    use super::*;

    #[test]
    fn removed_storage_slots_detects_zeroed_diff_storage() {
        let slot = b256!("0000000000000000000000000000000000000000000000000000000000000000");
        let previous = b256!("0000000000000000000000007a9bc53fbe126d61f9a6449fe8bb4e2f5ff29f52");
        let mut pre_state = PreStateAccountState::default();
        pre_state.storage.insert(slot, previous);
        let post_state = PreStateAccountState::default();

        assert_eq!(removed_storage_slots(&pre_state, &post_state), vec![slot]);
    }

    #[test]
    fn removed_storage_slots_keeps_nonzero_post_updates() {
        let slot = b256!("0000000000000000000000000000000000000000000000000000000000000000");
        let previous = b256!("0000000000000000000000007a9bc53fbe126d61f9a6449fe8bb4e2f5ff29f52");
        let updated = b256!("000000000000000000000000dadb0d80178819f2319190d340ce9a924f783711");
        let mut pre_state = PreStateAccountState::default();
        pre_state.storage.insert(slot, previous);
        let mut post_state = PreStateAccountState::default();
        post_state.storage.insert(slot, updated);

        assert!(removed_storage_slots(&pre_state, &post_state).is_empty());
    }

    #[test]
    fn historical_state_requires_reth_finish_at_requested_block() {
        assert!(historical_state_available_from_reth(100, 100));
        assert!(historical_state_available_from_reth(99, 100));
        assert!(!historical_state_available_from_reth(101, 100));
    }
}
