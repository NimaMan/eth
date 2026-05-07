/// Database management for RethIndex.
///
/// Handles MDBX environment creation and provides helpers for the analytics
/// tables that sit alongside the canonical Reth database.
use alloy_primitives::Address;
use eyre::{eyre, Result};
use reth_libmdbx::Error as MdbxError;
use reth_libmdbx::{
    Database, DatabaseFlags, Environment, EnvironmentFlags, Geometry, Mode, SyncMode, Transaction,
    WriteFlags, RO, RW,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

use crate::reth_index::tables::{
    address_blocks::{AddressBlockIndex, IndexedBlockNumber},
    mempool_tx_arrivals::MempoolTxArrivalTable,
};

/// RethIndex database manager.
pub struct RethIndexDB {
    path: PathBuf,
    env: Arc<Environment>,
    tx_arrival_dbi: Database,
    address_blocks_dbi: Database,
}

impl RethIndexDB {
    /// Open (or create) the MDBX environment for the analytics index with read-write access.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_internal(path, false)
    }

    /// Open the MDBX environment for the analytics index in read-only mode (no lock contention).
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_internal(path, true)
    }

    fn open_internal(path: impl AsRef<Path>, read_only: bool) -> Result<Self> {
        let path = path.as_ref();

        if read_only {
            eyre::ensure!(
                path.exists(),
                "RethIndex directory {} does not exist",
                path.display()
            );
        } else {
            std::fs::create_dir_all(path)?;
        }

        let env = if read_only {
            let mut builder = Environment::builder();
            builder.set_flags(EnvironmentFlags::from(Mode::ReadOnly));
            builder.set_max_dbs(32);
            builder.set_geometry(Geometry::<std::ops::RangeInclusive<usize>>::default());
            builder.open(path)?
        } else {
            Self::open_rw_environment(path)?
        };

        if read_only {
            let tx: Transaction<RO> = env.begin_ro_txn()?;
            let tx_arrival_dbi = tx.open_db(Some(MempoolTxArrivalTable::TABLE_NAME))?;
            let address_blocks_dbi = tx.open_db(Some(AddressBlockIndex::TABLE_NAME))?;

            Ok(Self {
                path: path.to_path_buf(),
                env: Arc::new(env),
                tx_arrival_dbi,
                address_blocks_dbi,
            })
        } else {
            let rwtx = env.begin_rw_txn()?;
            let tx_arrival_dbi = rwtx.create_db(
                Some(MempoolTxArrivalTable::TABLE_NAME),
                DatabaseFlags::INTEGER_KEY,
            )?;
            let address_blocks_dbi = match rwtx.create_db(
                Some(AddressBlockIndex::TABLE_NAME),
                DatabaseFlags::DUP_SORT | DatabaseFlags::DUP_FIXED,
            ) {
                Ok(dbi) => dbi,
                Err(MdbxError::Incompatible) => {
                    return Err(eyre!(
                        "Address block index table exists with a legacy layout. Delete {} and rebuild the index.",
                        path.display()
                    ));
                }
                Err(err) => return Err(err.into()),
            };
            rwtx.commit()?;

            Ok(Self {
                path: path.to_path_buf(),
                env: Arc::new(env),
                tx_arrival_dbi,
                address_blocks_dbi,
            })
        }
    }

    fn sync_flags_from_env() -> EnvironmentFlags {
        let env_value =
            std::env::var("PYRETH_INDEX_DB_SYNC_MODE").unwrap_or_else(|_| "safe-no-sync".into());
        let normalized = env_value.trim().to_ascii_lowercase();
        let sync_mode = match normalized.as_str() {
            "durable" => SyncMode::Durable,
            "no-metasync" | "nometasync" => SyncMode::NoMetaSync,
            "safe-no-sync" | "safenosync" | "safe_no_sync" => SyncMode::SafeNoSync,
            "utterly-no-sync" | "utterlynosync" => SyncMode::UtterlyNoSync,
            _ => {
                warn!(
                    sync_mode = %env_value,
                    "Invalid PYRETH_INDEX_DB_SYNC_MODE value; falling back to safe-no-sync"
                );
                SyncMode::SafeNoSync
            }
        };
        EnvironmentFlags::from(Mode::ReadWrite { sync_mode })
    }

    fn open_rw_environment(path: &Path) -> Result<Environment> {
        let mut tuned = Environment::builder();
        tuned.write_map();
        tuned.set_flags(Self::sync_flags_from_env());
        tuned
            .set_max_dbs(32)
            .set_geometry(Geometry::<std::ops::RangeInclusive<usize>>::default());

        match tuned.open(path) {
            Ok(env) => Ok(env),
            Err(err) => {
                warn!(
                    error = %err,
                    "Failed to open MDBX environment with tuned settings, falling back to defaults"
                );
                let mut fallback = Environment::builder();
                fallback
                    .set_max_dbs(32)
                    .set_geometry(Geometry::<std::ops::RangeInclusive<usize>>::default());
                fallback.open(path).map_err(|fallback_err| {
                    eyre!(
                        "Failed to open MDBX env with tuned settings ({err}) and fallback also failed ({fallback_err})"
                    )
                })
            }
        }
    }

    /// Absolute path to the MDBX directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Underlying MDBX environment handle.
    pub fn env(&self) -> &Arc<Environment> {
        &self.env
    }

    /// Fetch all block numbers associated with `address`.
    pub fn get_blocks(&self, address: Address) -> Result<Vec<IndexedBlockNumber>> {
        let tx: Transaction<RO> = self.env.begin_ro_txn()?;
        let mut cursor = tx.cursor(self.address_blocks_dbi.dbi())?;

        let mut blocks = Vec::new();
        let key = AddressBlockIndex::encode_key(address);

        if let Some((_, value)) = cursor.set_key::<Vec<u8>, Vec<u8>>(key.as_slice())? {
            blocks.push(AddressBlockIndex::decode_value(&value)?);
            while let Some((_, value)) = cursor.next_dup::<Vec<u8>, Vec<u8>>()? {
                blocks.push(AddressBlockIndex::decode_value(&value)?);
            }
        }

        Ok(blocks)
    }

    /// Append block numbers for multiple addresses in a single write transaction.
    ///
    /// Duplicate `(address, block_number)` values are skipped so block replay is
    /// idempotent.
    pub fn append_address_blocks_batch(
        &self,
        entries: &[(Address, Vec<IndexedBlockNumber>)],
    ) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let tx: Transaction<RW> = self.env.begin_rw_txn()?;
        let mut cursor = tx.cursor(self.address_blocks_dbi.dbi())?;
        let total = self.append_entries_with_cursor(&mut cursor, entries)?;
        tx.commit()?;
        Ok(total)
    }

    /// Append address block entries for multiple processed blocks inside a single MDBX transaction.
    pub fn append_block_entry_slices(
        &self,
        blocks: &[&[(Address, Vec<IndexedBlockNumber>)]],
    ) -> Result<Vec<usize>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let tx: Transaction<RW> = self.env.begin_rw_txn()?;
        let mut cursor = tx.cursor(self.address_blocks_dbi.dbi())?;
        let mut per_block = Vec::with_capacity(blocks.len());

        for entries in blocks {
            per_block.push(self.append_entries_with_cursor(&mut cursor, entries)?);
        }

        tx.commit()?;
        Ok(per_block)
    }

    fn append_entries_with_cursor(
        &self,
        cursor: &mut reth_libmdbx::Cursor<RW>,
        entries: &[(Address, Vec<IndexedBlockNumber>)],
    ) -> Result<usize> {
        let mut inserted = 0usize;
        for (address, blocks) in entries {
            if blocks.is_empty() {
                continue;
            }
            inserted += self.append_for_address(cursor, *address, blocks)?;
        }
        Ok(inserted)
    }

    fn append_for_address(
        &self,
        cursor: &mut reth_libmdbx::Cursor<RW>,
        address: Address,
        block_numbers: &[IndexedBlockNumber],
    ) -> Result<usize> {
        if block_numbers.is_empty() {
            return Ok(0);
        }

        let key = AddressBlockIndex::encode_key(address);
        let mut inserted = 0usize;

        for block_number in block_numbers.iter().copied() {
            let value = AddressBlockIndex::encode_value(block_number);
            match cursor.put(key.as_slice(), &value, WriteFlags::NO_DUP_DATA) {
                Ok(_) => {
                    inserted += 1;
                }
                Err(MdbxError::KeyExist) => {
                    // Duplicate entry, skip.
                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        }

        Ok(inserted)
    }

    /// Write arrival timestamp (milliseconds) for a transaction number.
    pub fn put_tx_arrival_ms(&self, tx_number: u64, first_seen_ms: u64) -> Result<()> {
        let tx: Transaction<RW> = self.env.begin_rw_txn()?;
        let key = MempoolTxArrivalTable::encode_key(tx_number);
        let val = MempoolTxArrivalTable::encode_value(first_seen_ms);
        tx.put(self.tx_arrival_dbi.dbi(), &key, &val, WriteFlags::UPSERT)?;
        tx.commit()?;
        Ok(())
    }

    /// Fetch arrival timestamp (milliseconds) for a transaction number if present.
    pub fn get_tx_arrival_ms(&self, tx_number: u64) -> Result<Option<u64>> {
        let tx: Transaction<RO> = self.env.begin_ro_txn()?;
        let key = MempoolTxArrivalTable::encode_key(tx_number);
        let value = tx.get::<[u8; 8]>(self.tx_arrival_dbi.dbi(), &key)?;
        match value {
            Some(bytes) => Ok(Some(MempoolTxArrivalTable::decode_value(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Batch write arrivals in a single RW transaction.
    pub fn put_tx_arrivals_ms(&self, entries: &[(u64, u64)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let tx: Transaction<RW> = self.env.begin_rw_txn()?;
        for (tx_number, first_seen_ms) in entries.iter().copied() {
            let key = MempoolTxArrivalTable::encode_key(tx_number);
            let val = MempoolTxArrivalTable::encode_value(first_seen_ms);
            tx.put(self.tx_arrival_dbi.dbi(), &key, &val, WriteFlags::UPSERT)?;
        }
        tx.commit()?;
        Ok(entries.len())
    }

    /// Delete arrival timestamp for `tx_number`. Returns true if an entry existed.
    pub fn delete_tx_arrival(&self, tx_number: u64) -> Result<bool> {
        let tx: Transaction<RW> = self.env.begin_rw_txn()?;
        let key = MempoolTxArrivalTable::encode_key(tx_number);
        match tx.del(self.tx_arrival_dbi.dbi(), &key, Option::<&[u8]>::None) {
            Ok(true) => {
                tx.commit()?;
                Ok(true)
            }
            Ok(false) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    /// Count entries in the arrival table (for diagnostics).
    pub fn count_tx_arrivals(&self) -> Result<u64> {
        let tx: Transaction<RO> = self.env.begin_ro_txn()?;
        let cursor = tx.cursor(self.tx_arrival_dbi.dbi())?;
        let mut count = 0u64;
        for entry in cursor.into_iter::<[u8; 8], [u8; 8]>() {
            entry?;
            count += 1;
        }
        Ok(count)
    }
}

/// Lightweight database stats placeholder.
#[derive(Debug, Default)]
pub struct DatabaseStats {
    pub total_size: u64,
    pub address_count: u64,
    pub transaction_count: u64,
    pub last_processed_block: u64,
}
