/// Database management for RethIndex.
///
/// Handles MDBX environment creation and provides helpers for the analytics
/// tables that sit alongside the canonical Reth database.
use alloy_primitives::Address;
use eyre::Result;
use reth_libmdbx::{
    Database, DatabaseFlags, Environment, EnvironmentFlags, Geometry, Mode, Transaction,
    WriteFlags, RO, RW,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::reth_index::tables::{
    address_index::{self, txumber, AddressIndex},
    mempool_tx_arrivals::MempoolTxArrivalTable,
};

/// RethIndex database manager.
pub struct RethIndexDB {
    path: PathBuf,
    env: Arc<Environment>,
    tx_arrival_dbi: Database,
    address_index_dbi: Database,
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

        let mut builder = Environment::builder();

        if read_only {
            builder.set_flags(EnvironmentFlags::from(Mode::ReadOnly));
            builder.set_max_dbs(32);
        } else {
            builder
                .set_max_dbs(32)
                .set_geometry(Geometry::<std::ops::RangeInclusive<usize>>::default());
        }

        let env = builder.open(path)?;

        if read_only {
            let tx: Transaction<RO> = env.begin_ro_txn()?;
            let tx_arrival_dbi = tx.open_db(Some(MempoolTxArrivalTable::TABLE_NAME))?;
            let address_index_dbi = tx.open_db(Some(AddressIndex::TABLE_NAME))?;

            Ok(Self {
                path: path.to_path_buf(),
                env: Arc::new(env),
                tx_arrival_dbi,
                address_index_dbi,
            })
        } else {
            let mut rwtx = env.begin_rw_txn()?;
            let tx_arrival_dbi = rwtx.create_db(
                Some(MempoolTxArrivalTable::TABLE_NAME),
                DatabaseFlags::INTEGER_KEY,
            )?;
            let address_index_dbi =
                rwtx.create_db(Some(AddressIndex::TABLE_NAME), DatabaseFlags::empty())?;
            rwtx.commit()?;

            Ok(Self {
                path: path.to_path_buf(),
                env: Arc::new(env),
                tx_arrival_dbi,
                address_index_dbi,
            })
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

    /// Fetch all transaction numbers associated with `address`.
    pub fn get_transactions(&self, address: Address) -> Result<Vec<txumber>> {
        let tx: Transaction<RO> = self.env.begin_ro_txn()?;
        let mut cursor = tx.cursor(&self.address_index_dbi)?;

        let mut txs = Vec::new();
        let mut item =
            cursor.set_range::<Vec<u8>, Vec<u8>>(&AddressIndex::encode_shard_prefix(address))?;

        while let Some((key, value)) = item {
            if AddressIndex::key_address(&key)? != address {
                break;
            }

            let mut shard = AddressIndex::decode_values(&value)?;
            txs.append(&mut shard);
            item = cursor.next::<Vec<u8>, Vec<u8>>()?;
        }

        Ok(txs)
    }

    /// Append transactions for multiple addresses in a single write transaction.
    ///
    /// Each vector must be sorted ascending and contain only new (greater-than-last)
    /// entries for that address.
    pub fn append_address_transactions_batch(
        &self,
        entries: &[(Address, Vec<txumber>)],
    ) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let mut tx: Transaction<RW> = self.env.begin_rw_txn()?;
        let mut total = 0usize;

        for (address, txs) in entries {
            if txs.is_empty() {
                continue;
            }
            total += self.append_for_address(&mut tx, *address, txs)?;
        }

        tx.commit()?;
        Ok(total)
    }

    fn append_for_address(
        &self,
        tx: &mut Transaction<RW>,
        address: Address,
        txs: &Vec<txumber>,
    ) -> Result<usize> {
        if txs.is_empty() {
            return Ok(0);
        }

        let mut cursor = tx.cursor(&self.address_index_dbi)?;
        let prefix = AddressIndex::encode_shard_prefix(address);

        // Gather existing shards for this address (if any).
        let mut shards: Vec<(Vec<u8>, Vec<txumber>, u64)> = Vec::new();
        let mut existing_flat: Vec<txumber> = Vec::new();
        let mut item = cursor.set_range::<Vec<u8>, Vec<u8>>(&prefix)?;
        while let Some((key, value)) = item {
            if AddressIndex::key_address(&key)? != address {
                break;
            }

            let (_, shard_id) = AddressIndex::decode_shard_key(&key)?;
            let shard_values = AddressIndex::decode_values(&value)?;
            existing_flat.extend_from_slice(&shard_values);
            shards.push((key, shard_values, shard_id));
            item = cursor.next::<Vec<u8>, Vec<u8>>()?;
        }

        let mut new_values: Vec<txumber> = txs.iter().copied().collect();
        new_values.sort_unstable();
        new_values.dedup();

        if new_values.is_empty() {
            return Ok(0);
        }

        if shards.is_empty() {
            let inserted = new_values.len();
            let mut to_write = new_values;
            self.insert_additional_shards(&mut cursor, address, 0, &mut to_write)?;
            return Ok(inserted);
        }

        // Existing data present – determine if we can append or need a rewrite.
        let existing_len = existing_flat.len();
        let max_existing = existing_flat
            .last()
            .copied()
            .expect("at least one value present with shards");

        let mut greater_than_max: Vec<txumber> = Vec::new();
        let mut missing_earlier: Vec<txumber> = Vec::new();

        for value in new_values.iter().copied() {
            if value > max_existing {
                greater_than_max.push(value);
            } else if existing_flat.binary_search(&value).is_err() {
                missing_earlier.push(value);
            }
        }

        if missing_earlier.is_empty() {
            // All new entries are strictly greater than the current max (or duplicates that already
            // exist). Append efficiently without rewriting earlier shards.
            if greater_than_max.is_empty() {
                return Ok(0);
            }

            let (last_key, last_values, last_shard_id) =
                shards.last_mut().expect("non-empty shards");

            let mut inserted = 0usize;
            let available = address_index::SHARD_TX_CAPACITY.saturating_sub(last_values.len());
            if available > 0 {
                let take = available.min(greater_than_max.len());
                if take > 0 {
                    last_values.extend_from_slice(&greater_than_max[..take]);
                    let encoded = AddressIndex::encode_values(last_values);
                    cursor.put(last_key.as_slice(), encoded.as_slice(), WriteFlags::UPSERT)?;
                    inserted += take;
                    greater_than_max.drain(..take);
                }
            }

            if !greater_than_max.is_empty() {
                inserted += self.insert_additional_shards(
                    &mut cursor,
                    address,
                    *last_shard_id + 1,
                    &mut greater_than_max,
                )?;
            }

            return Ok(inserted);
        }

        // We have new entries that belong before the current max. Merge everything and rewrite.
        let mut combined = existing_flat;
        combined.extend(missing_earlier.iter().copied());
        combined.extend(greater_than_max.iter().copied());
        combined.sort_unstable();
        combined.dedup();

        let combined_len = combined.len();
        let newly_inserted = combined_len.saturating_sub(existing_len);
        if newly_inserted == 0 {
            return Ok(0);
        }

        // Remove existing shards for the address so we can rewrite them in order.
        let mut item = cursor.set_range::<Vec<u8>, Vec<u8>>(&prefix)?;
        while let Some((key, _)) = item {
            if AddressIndex::key_address(&key)? != address {
                break;
            }
            cursor.del(WriteFlags::CURRENT)?;
            item = cursor.next::<Vec<u8>, Vec<u8>>()?;
        }

        let mut to_write = combined;
        self.insert_additional_shards(&mut cursor, address, 0, &mut to_write)?;
        Ok(newly_inserted)
    }

    fn insert_additional_shards(
        &self,
        cursor: &mut reth_libmdbx::Cursor<RW>,
        address: Address,
        mut shard_id: u64,
        txs: &mut Vec<txumber>,
    ) -> Result<usize> {
        let mut inserted = 0usize;

        while !txs.is_empty() {
            let take = txs.len().min(address_index::SHARD_TX_CAPACITY);
            let chunk: Vec<txumber> = txs.drain(..take).collect();
            let key = AddressIndex::encode_shard_key(address, shard_id);
            let encoded = AddressIndex::encode_values(&chunk);
            cursor.put(key.as_slice(), encoded.as_slice(), WriteFlags::UPSERT)?;
            shard_id += 1;
            inserted += chunk.len();
        }

        Ok(inserted)
    }

    /// Write arrival timestamp (milliseconds) for a transaction number.
    pub fn put_tx_arrival_ms(&self, tx_number: u64, first_seen_ms: u64) -> Result<()> {
        let mut tx: Transaction<RW> = self.env.begin_rw_txn()?;
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

        let mut tx: Transaction<RW> = self.env.begin_rw_txn()?;
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
        let mut tx: Transaction<RW> = self.env.begin_rw_txn()?;
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
        let cursor = tx.cursor(&self.tx_arrival_dbi)?;
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
