//! Shared helpers for loading block headers and state from local Reth context.
//!
//! The goal of this module is to centralize the logic required to hydrate block
//! context from the local Reth database. Callers that need headers or state at a
//! specific block should rely on [`BlockContextLoader`] instead of open-coding
//! provider access.

pub mod header_json;

use crate::{
    config::view_call::{STATE_RETRY_DELAY_MS, STATE_RETRY_MAX_ATTEMPTS},
    tx_chain::sequential::{ForkedState, SharedStateProvider},
    TxSimulator,
};
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types_trace::geth::{AccountState as PreStateAccountState, DiffMode, PreStateFrame};
use eyre::{eyre, Result};
use reth_primitives_traits::SealedHeader;
use reth_provider::{HeaderProvider, StateProviderBox};
use reth_revm::{
    database::StateProviderDatabase,
    db::{AccountState as RevmAccountState, CacheDB, DbAccount},
};
use revm::bytecode::Bytecode;
use revm::state::AccountInfo;
use tokio::time::{sleep, Duration};
use tracing::debug;

/// Complete block context (header + state source).
pub(crate) struct BlockContext {
    pub header: SealedHeader,
    pub state: StateProviderBox,
}

/// Convenience wrapper around the shared header/state loader logic.
pub struct BlockContextLoader<'a> {
    simulator: &'a TxSimulator,
}

impl<'a> BlockContextLoader<'a> {
    pub fn new(simulator: &'a TxSimulator) -> Self {
        Self { simulator }
    }

    /// Load block header + state from local historical context.
    pub(crate) async fn load_block_context(
        &self,
        block_number: u64,
        header_hint: Option<SealedHeader>,
    ) -> Result<BlockContext> {
        let header = self.load_block_header(block_number, header_hint).await?;

        if let Some(state) = self.try_load_historical_state(block_number).await? {
            return Ok(BlockContext { header, state });
        }

        Err(eyre!(
            "state for block {} is unavailable from local Reth historical state",
            block_number
        ))
    }

    /// Resolve a sealed header for a specific block from local Reth.
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

        Err(eyre!(
            "missing local Reth block header for {}",
            block_number
        ))
    }

    fn fetch_header_from_mdbx(&self, block_number: u64) -> Result<Option<SealedHeader>> {
        self.simulator.refresh_static_file_provider()?;
        let provider = self.simulator.provider_factory.provider()?;
        let maybe_header = provider.header_by_number(block_number)?;
        Ok(maybe_header.map(SealedHeader::new_unhashed))
    }

    async fn try_load_historical_state(
        &self,
        block_number: u64,
    ) -> Result<Option<StateProviderBox>> {
        let latest_reth_finished = self.simulator.get_latest_block()?;
        if !historical_state_available_from_reth(block_number, latest_reth_finished) {
            debug!(
                block_number,
                latest_reth_finished, "Reth historical state is behind requested block"
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
            "Reth historical state unavailable"
        );
        Ok(None)
    }

    pub(crate) async fn direct_forked_state_from_prestate_diffs(
        &self,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        header: SealedHeader,
        state_diffs: &[PreStateFrame],
    ) -> Result<ForkedState> {
        if header.number != block_number {
            return Err(eyre!(
                "direct live state header block mismatch: header={}, expected={}",
                header.number,
                block_number
            ));
        }
        if header.hash() != block_hash {
            return Err(eyre!(
                "direct live state header hash mismatch for block {}: header={}, expected={}",
                block_number,
                header.hash(),
                block_hash
            ));
        }
        if header.parent_hash != parent_hash {
            return Err(eyre!(
                "direct live state parent hash mismatch for block {}: header={}, expected={}",
                block_number,
                header.parent_hash,
                parent_hash
            ));
        }

        let parent_block = block_number
            .checked_sub(1)
            .ok_or_else(|| eyre!("cannot build direct live state for genesis block"))?;
        self.try_load_historical_state(parent_block)
            .await?
            .ok_or_else(|| {
                eyre!(
                    "cannot build direct live state for block {}: exact parent state {} is unavailable from Reth historical state",
                    block_number,
                    parent_block
                )
            })?;
        let parent_header = self.fetch_header_from_mdbx(parent_block)?.ok_or_else(|| {
            eyre!(
                "cannot build direct live state for block {}: exact parent header {} is unavailable from Reth historical headers",
                block_number,
                parent_block
            )
        })?;
        if parent_header.hash() != parent_hash {
            return Err(eyre!(
                "direct live state parent hash mismatch for block {}: parent header={}, expected={}",
                block_number,
                parent_header.hash(),
                parent_hash
            ));
        }

        let mut fork_state = forked_state_from_refreshing_historical_state(
            parent_block,
            parent_header,
            self.simulator.provider_factory.clone(),
        );

        fork_state.block_number = header.number;
        fork_state.block_header = header;
        fork_state.nonces.clear();
        fork_state
            .db
            .cache
            .block_hashes
            .insert(U256::from(parent_block), parent_hash);
        fork_state
            .db
            .cache
            .block_hashes
            .insert(U256::from(block_number), block_hash);
        for (tx_index, frame) in state_diffs.iter().enumerate() {
            let diff = frame.as_diff().ok_or_else(|| {
                eyre!(
                    "direct live state diff for block {} tx index {} was not diffMode",
                    block_number,
                    tx_index
                )
            })?;
            apply_prestate_diff(&mut fork_state, diff)?;
        }
        Ok(fork_state)
    }
}

fn historical_state_available_from_reth(block_number: u64, latest_reth_finished: u64) -> bool {
    block_number <= latest_reth_finished
}

fn forked_state_from_refreshing_historical_state(
    block_number: u64,
    block_header: SealedHeader,
    provider_factory: crate::simulator::EthereumProviderFactory,
) -> ForkedState {
    let db = CacheDB::new(StateProviderDatabase::new(
        SharedStateProvider::refreshing_history(provider_factory, block_number),
    ));
    ForkedState {
        db,
        block_number,
        block_header,
        nonces: Default::default(),
    }
}

pub(crate) fn apply_prestate_diff(fork_state: &mut ForkedState, diff: &DiffMode) -> Result<()> {
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
    let mut info = account_info_from_prestate(pre_state)
        .or_else(|| {
            fork_state
                .db
                .cache
                .accounts
                .get(&address)
                .and_then(DbAccount::info)
        })
        .unwrap_or_default();

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

fn account_info_from_prestate(pre_state: Option<&PreStateAccountState>) -> Option<AccountInfo> {
    let pre_state = pre_state?;
    let mut info = AccountInfo::default();
    if let Some(balance) = pre_state.balance {
        info.balance = balance;
    }
    if let Some(nonce) = pre_state.nonce {
        info.nonce = nonce;
    }
    if let Some(code) = pre_state.code.as_ref() {
        let bytecode = Bytecode::new_raw(code.clone());
        info.code_hash = bytecode.hash_slow();
        info.code = Some(bytecode);
    }
    Some(info)
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
