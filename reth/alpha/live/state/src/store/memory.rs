use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use alloy_primitives::{Address, B256};
use async_trait::async_trait;

use crate::{
    keys, BlockNumber, BlockReadyNotification, EncodedChainStateSnapshot, LiveStateError, Result,
    SnapshotWriteOptions, TokenSnapshot,
};

use super::{LiveStateReader, LiveStateWriter};

#[derive(Clone, Debug, Default)]
pub struct InMemoryLiveStateStore {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Debug, Default)]
struct Inner {
    chain_states: BTreeMap<BlockNumber, EncodedChainStateSnapshot>,
    tokens: BTreeMap<String, TokenSnapshot>,
    latest_block_number: Option<BlockNumber>,
    latest_block_hash: Option<B256>,
    latest_chain_state_block_number: Option<BlockNumber>,
    block_ready_notifications: Vec<BlockReadyNotification>,
}

impl InMemoryLiveStateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block_ready_notifications(&self) -> Result<Vec<BlockReadyNotification>> {
        Ok(self.read_inner()?.block_ready_notifications.clone())
    }

    fn read_inner(&self) -> Result<RwLockReadGuard<'_, Inner>> {
        self.inner
            .read()
            .map_err(|_| LiveStateError::Store("in-memory live-state read lock poisoned".into()))
    }

    fn write_inner(&self) -> Result<RwLockWriteGuard<'_, Inner>> {
        self.inner
            .write()
            .map_err(|_| LiveStateError::Store("in-memory live-state write lock poisoned".into()))
    }
}

#[async_trait]
impl LiveStateReader for InMemoryLiveStateStore {
    async fn latest_block_number(&self) -> Result<Option<BlockNumber>> {
        Ok(self.read_inner()?.latest_block_number)
    }

    async fn latest_block_hash(&self) -> Result<Option<B256>> {
        Ok(self.read_inner()?.latest_block_hash)
    }

    async fn latest_chain_state_block_number(&self) -> Result<Option<BlockNumber>> {
        Ok(self.read_inner()?.latest_chain_state_block_number)
    }

    async fn read_chain_state_snapshot(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<EncodedChainStateSnapshot>> {
        Ok(self.read_inner()?.chain_states.get(&block_number).cloned())
    }

    async fn read_token(&self, token_address: Address) -> Result<Option<TokenSnapshot>> {
        Ok(self
            .read_inner()?
            .tokens
            .get(&keys::normalized_address_string(token_address))
            .cloned())
    }

    async fn list_token_addresses(&self) -> Result<Vec<Address>> {
        Ok(self
            .read_inner()?
            .tokens
            .values()
            .map(|token| token.contract_address)
            .collect())
    }
}

#[async_trait]
impl LiveStateWriter for InMemoryLiveStateStore {
    async fn mark_block_ready(&self, notification: BlockReadyNotification) -> Result<()> {
        let mut inner = self.write_inner()?;
        inner.latest_block_number = Some(notification.block_number);
        inner.latest_block_hash = Some(notification.block_hash);
        inner.block_ready_notifications.push(notification);
        Ok(())
    }

    async fn write_chain_state_snapshot(&self, snapshot: EncodedChainStateSnapshot) -> Result<()> {
        let mut inner = self.write_inner()?;
        let block_number = snapshot.block_number;
        inner.latest_chain_state_block_number = Some(match inner.latest_chain_state_block_number {
            Some(current) => current.max(block_number),
            None => block_number,
        });
        inner.chain_states.insert(block_number, snapshot);
        Ok(())
    }

    async fn write_token(
        &self,
        snapshot: TokenSnapshot,
        _options: SnapshotWriteOptions,
    ) -> Result<()> {
        self.write_inner()?.tokens.insert(
            keys::normalized_address_string(snapshot.contract_address),
            snapshot,
        );
        Ok(())
    }

    async fn delete_token(&self, token_address: Address) -> Result<()> {
        self.write_inner()?
            .tokens
            .remove(&keys::normalized_address_string(token_address));
        Ok(())
    }
}
