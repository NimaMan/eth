/// Database management for RethIndex
/// 
/// This module handles the MDBX environment setup and table management
/// for the RethIndex complementary database.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use eyre::Result;
use reth_libmdbx::{Environment, Geometry, DatabaseFlags, WriteFlags};
use crate::reth_index::tables::mempool_tx_arrivals::MempoolTxArrivalTable;

/// RethIndex database manager
pub struct RethIndexDB {
    path: PathBuf,
    env: Arc<Environment>,
    tx_arrival_dbi: reth_libmdbx::Database,
}

impl RethIndexDB {
    /// Open or create the RethIndex database
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;
        // Place the index in a single file arrival_index.mdbx within the provided directory
        let env = Environment::builder()
            .set_max_dbs(16)
            .set_geometry(Geometry::<std::ops::RangeInclusive<usize>>::default())
            .open(path)?;

        // Create tables if not exist
        let mut rwtx = env.begin_rw_txn()?;
        let tx_arrival_dbi = rwtx.create_db(Some(MempoolTxArrivalTable::TABLE_NAME), DatabaseFlags::INTEGER_KEY)?;
        rwtx.commit()?;

        Ok(Self {
            path: path.to_path_buf(),
            env: Arc::new(env),
            tx_arrival_dbi,
        })
    }

    /// Create all tables if they don't exist
    pub fn create_tables(&self) -> Result<()> { Ok(()) }

    /// Get database statistics
    pub fn stats(&self) -> Result<DatabaseStats> {
        Ok(DatabaseStats::default())
    }
    
    /// Get all transactions for an address
    pub fn get_transactions(&self, _address: alloy_primitives::Address) -> Result<Vec<u64>> {
        // TODO: Implement address_to_txs table lookup
        Ok(Vec::new())
    }

    /// Write arrival timestamp (milliseconds) for a transaction number
    pub fn put_tx_arrival_ms(&self, tx_number: u64, first_seen_ms: u64) -> Result<()> {
        let mut tx = self.env.begin_rw_txn()?;
        let key = MempoolTxArrivalTable::encode_key(tx_number);
        let val = MempoolTxArrivalTable::encode_value(first_seen_ms);
        tx.put(self.tx_arrival_dbi.dbi(), &key, &val, WriteFlags::UPSERT)?;
        tx.commit()?;
        Ok(())
    }

    /// Read arrival timestamp (milliseconds) for a transaction number
    pub fn get_tx_arrival_ms(&self, tx_number: u64) -> Result<Option<u64>> {
        let ro = self.env.begin_ro_txn()?;
        let key = MempoolTxArrivalTable::encode_key(tx_number);
        match ro.get::<Vec<u8>>(self.tx_arrival_dbi.dbi(), &key) {
            Ok(Some(bytes)) => Ok(Some(MempoolTxArrivalTable::decode_value(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Batch write arrivals in a single RW transaction.
    pub fn put_tx_arrivals_ms(&self, entries: &[(u64, u64)]) -> Result<usize> {
        if entries.is_empty() { return Ok(0); }
        let mut tx = self.env.begin_rw_txn()?;
        for (tx_number, first_seen_ns) in entries.iter().copied() {
            let key = MempoolTxArrivalTable::encode_key(tx_number);
            let val = MempoolTxArrivalTable::encode_value(first_seen_ns);
            tx.put(self.tx_arrival_dbi.dbi(), &key, &val, WriteFlags::UPSERT)?;
        }
        tx.commit()?;
        Ok(entries.len())
    }

    /// Delete arrival timestamp for a transaction number. Returns true if deleted.
    pub fn delete_tx_arrival(&self, tx_number: u64) -> Result<bool> {
        let mut tx = self.env.begin_rw_txn()?;
        let key = MempoolTxArrivalTable::encode_key(tx_number);
        if let Err(e) = tx.del(self.tx_arrival_dbi.dbi(), &key, Option::<&[u8]>::None) {
            if let reth_libmdbx::Error::NotFound = e { return Ok(false); }
            return Err(e.into());
        }
        tx.commit()?;
        Ok(true)
    }

    /// Count all tx arrival entries
    pub fn count_tx_arrivals(&self) -> Result<u64> {
        let ro = self.env.begin_ro_txn()?;
        let cursor = ro.cursor(&self.tx_arrival_dbi)?;
        let mut count: u64 = 0;
        for res in cursor.into_iter::<[u8;8], [u8;8]>() {
            let _ = res?;
            count += 1;
        }
        Ok(count)
    }
}

/// Database statistics
#[derive(Debug, Default)]
pub struct DatabaseStats {
    pub total_size: u64,
    pub address_count: u64,
    pub transaction_count: u64,
    pub last_processed_block: u64,
}
