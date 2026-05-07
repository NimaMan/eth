use alloy_primitives::{Address, B256, U256};
use eyre::Result;
use reth_provider::StateProviderBox;
use reth_revm::database::StateProviderDatabase;
use reth_revm::State;
use revm::{bytecode::Bytecode, state::AccountInfo, Database, DatabaseRef};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::block_trace::types::{StateAccessKeys, StateReadProfile, StorageAccessKey};

use super::metrics::ms;

pub(super) type ProfiledCacheDb = State<InstrumentedStateProviderDatabase>;

#[derive(Debug)]
pub(super) struct InstrumentedStateProviderDatabase {
    inner: StateProviderDatabase<StateProviderBox>,
    reads: Arc<Mutex<StateReadProfile>>,
    keys: Option<Arc<Mutex<StateAccessKeys>>>,
}

impl InstrumentedStateProviderDatabase {
    pub(super) fn new(
        inner: StateProviderDatabase<StateProviderBox>,
        reads: Arc<Mutex<StateReadProfile>>,
        keys: Option<Arc<Mutex<StateAccessKeys>>>,
    ) -> Self {
        Self { inner, reads, keys }
    }

    fn record(&self, kind: StateReadKind, elapsed: Duration) {
        if let Ok(mut reads) = self.reads.lock() {
            match kind {
                StateReadKind::Account(_) => reads.account_reads += 1,
                StateReadKind::Storage { .. } => reads.storage_reads += 1,
                StateReadKind::Code(_) => reads.code_reads += 1,
                StateReadKind::BlockHash(_) => reads.block_hash_reads += 1,
            }
            reads.provider_read_ms += ms(elapsed);
        }

        if let Some(keys) = &self.keys {
            if let Ok(mut keys) = keys.lock() {
                match kind {
                    StateReadKind::Account(address) => {
                        keys.accounts.insert(address);
                    }
                    StateReadKind::Storage { address, key } => {
                        keys.storage.insert(StorageAccessKey { address, key });
                    }
                    StateReadKind::Code(code_hash) => {
                        keys.code_hashes.insert(code_hash);
                    }
                    StateReadKind::BlockHash(number) => {
                        keys.block_hashes.insert(number);
                    }
                }
            }
        }
    }

    pub(super) fn snapshot_reads(&self) -> StateReadProfile {
        self.reads
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

enum StateReadKind {
    Account(Address),
    Storage { address: Address, key: U256 },
    Code(B256),
    BlockHash(u64),
}

impl DatabaseRef for InstrumentedStateProviderDatabase {
    type Error = <StateProviderDatabase<StateProviderBox> as DatabaseRef>::Error;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let started = Instant::now();
        let result = self.inner.basic_ref(address);
        self.record(StateReadKind::Account(address), started.elapsed());
        result
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        let started = Instant::now();
        let result = self.inner.code_by_hash_ref(code_hash);
        self.record(StateReadKind::Code(code_hash), started.elapsed());
        result
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let started = Instant::now();
        let result = self.inner.storage_ref(address, index);
        self.record(
            StateReadKind::Storage {
                address,
                key: index,
            },
            started.elapsed(),
        );
        result
    }

    fn storage_by_account_id_ref(
        &self,
        address: Address,
        account_id: usize,
        storage_key: U256,
    ) -> Result<U256, Self::Error> {
        let started = Instant::now();
        let result = self
            .inner
            .storage_by_account_id_ref(address, account_id, storage_key);
        self.record(
            StateReadKind::Storage {
                address,
                key: storage_key,
            },
            started.elapsed(),
        );
        result
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        let started = Instant::now();
        let result = self.inner.block_hash_ref(number);
        self.record(StateReadKind::BlockHash(number), started.elapsed());
        result
    }
}

impl Database for InstrumentedStateProviderDatabase {
    type Error = <StateProviderDatabase<StateProviderBox> as Database>::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        self.basic_ref(address)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        self.code_by_hash_ref(code_hash)
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        self.storage_ref(address, index)
    }

    fn storage_by_account_id(
        &mut self,
        address: Address,
        account_id: usize,
        storage_key: U256,
    ) -> Result<U256, Self::Error> {
        self.storage_by_account_id_ref(address, account_id, storage_key)
    }

    fn block_hash(&mut self, number: u64) -> Result<B256, Self::Error> {
        self.block_hash_ref(number)
    }
}

pub(super) fn prewarm_profiled_cache_db(
    db: &mut ProfiledCacheDb,
    keys: &StateAccessKeys,
) -> Result<()> {
    for address in &keys.accounts {
        let _ = db.basic(*address)?;
    }

    for access in &keys.storage {
        let _ = db.storage(access.address, access.key)?;
    }

    for code_hash in &keys.code_hashes {
        let _ = db.code_by_hash(*code_hash)?;
    }

    for number in &keys.block_hashes {
        let _ = db.block_hash(*number)?;
    }

    Ok(())
}
