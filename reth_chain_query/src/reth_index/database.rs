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
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

use crate::reth_index::tables::{
    address_block_participation::{AddressBlockParticipationIndex, ParticipationBlockNumber},
    mempool_tx_arrivals::MempoolTxArrivalTable,
};

const PYRETH_INDEX_DB_MAP_SIZE_BYTES_ENV: &str = "PYRETH_INDEX_DB_MAP_SIZE_BYTES";
const PYRETH_INDEX_DB_GROWTH_STEP_BYTES_ENV: &str = "PYRETH_INDEX_DB_GROWTH_STEP_BYTES";
const MIB: usize = 1024 * 1024;
const GIB: usize = 1024 * MIB;
const MIN_RETH_INDEX_DB_MAP_SIZE_BYTES: usize = 64 * MIB;
const DEFAULT_NEW_RETH_INDEX_DB_MAP_SIZE_BYTES: usize = 64 * GIB;
const DEFAULT_EXISTING_RETH_INDEX_DB_MAP_HEADROOM_BYTES: usize = 32 * GIB;
const MIN_RETH_INDEX_DB_GROWTH_STEP_BYTES: usize = 16 * MIB;
const DEFAULT_RETH_INDEX_DB_GROWTH_STEP_BYTES: usize = GIB;

/// RethIndex database manager.
pub struct RethIndexDB {
    path: PathBuf,
    env: Arc<Environment>,
    tx_arrival_dbi: Database,
    address_block_participation_dbi: Database,
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
            builder.set_geometry(reth_index_geometry_from_env(path));
            builder.open(path)?
        } else {
            Self::open_rw_environment(path)?
        };

        if read_only {
            let tx: Transaction<RO> = env.begin_ro_txn()?;
            let tx_arrival_dbi = tx.open_db(Some(MempoolTxArrivalTable::TABLE_NAME))?;
            let address_block_participation_dbi =
                tx.open_db(Some(AddressBlockParticipationIndex::TABLE_NAME))?;

            Ok(Self {
                path: path.to_path_buf(),
                env: Arc::new(env),
                tx_arrival_dbi,
                address_block_participation_dbi,
            })
        } else {
            let rwtx = env.begin_rw_txn()?;
            let tx_arrival_dbi = rwtx.create_db(
                Some(MempoolTxArrivalTable::TABLE_NAME),
                DatabaseFlags::INTEGER_KEY,
            )?;
            let address_block_participation_dbi = match rwtx.create_db(
                Some(AddressBlockParticipationIndex::TABLE_NAME),
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
                address_block_participation_dbi,
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
            .set_geometry(reth_index_geometry_from_env(path));

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
                    .set_geometry(reth_index_geometry_from_env(path));
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
    pub fn get_address_participation_blocks(
        &self,
        address: Address,
    ) -> Result<Vec<ParticipationBlockNumber>> {
        let tx: Transaction<RO> = self.env.begin_ro_txn()?;
        let mut cursor = tx.cursor(self.address_block_participation_dbi.dbi())?;

        let mut blocks = Vec::new();
        let key = AddressBlockParticipationIndex::encode_key(address);

        if let Some((_, value)) = cursor.set_key::<Vec<u8>, Vec<u8>>(key.as_slice())? {
            blocks.push(AddressBlockParticipationIndex::decode_value(&value)?);
            while let Some((_, value)) = cursor.next_dup::<Vec<u8>, Vec<u8>>()? {
                blocks.push(AddressBlockParticipationIndex::decode_value(&value)?);
            }
        }

        Ok(blocks)
    }

    /// Append block numbers for multiple addresses in a single write transaction.
    ///
    /// Duplicate `(address, block_number)` values are skipped so block replay is
    /// idempotent.
    pub fn append_address_participation_blocks_batch(
        &self,
        entries: &[(Address, Vec<ParticipationBlockNumber>)],
    ) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let tx: Transaction<RW> = self.env.begin_rw_txn()?;
        let mut cursor = tx.cursor(self.address_block_participation_dbi.dbi())?;
        let total = self.append_entries_with_cursor(&mut cursor, entries)?;
        tx.commit()?;
        Ok(total)
    }

    /// Append address block entries for multiple processed blocks inside a single MDBX transaction.
    pub fn append_address_participation_blocks_by_block(
        &self,
        blocks: &[&[(Address, Vec<ParticipationBlockNumber>)]],
    ) -> Result<Vec<usize>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let tx: Transaction<RW> = self.env.begin_rw_txn()?;
        let mut cursor = tx.cursor(self.address_block_participation_dbi.dbi())?;
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
        entries: &[(Address, Vec<ParticipationBlockNumber>)],
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
        block_numbers: &[ParticipationBlockNumber],
    ) -> Result<usize> {
        if block_numbers.is_empty() {
            return Ok(0);
        }

        let key = AddressBlockParticipationIndex::encode_key(address);
        let mut inserted = 0usize;

        for block_number in block_numbers.iter().copied() {
            let value = AddressBlockParticipationIndex::encode_value(block_number);
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

    /// Return all `(tx_number, first_seen_ms)` arrival entries ordered by `tx_number`.
    pub fn tx_arrival_entries(&self) -> Result<Vec<(u64, u64)>> {
        let tx: Transaction<RO> = self.env.begin_ro_txn()?;
        let cursor = tx.cursor(self.tx_arrival_dbi.dbi())?;
        let mut entries = Vec::new();
        for entry in cursor.into_iter::<[u8; 8], [u8; 8]>() {
            let (key, value) = entry?;
            entries.push((
                MempoolTxArrivalTable::decode_key(&key)?,
                MempoolTxArrivalTable::decode_value(&value)?,
            ));
        }
        entries.sort_by_key(|(tx_number, _)| *tx_number);
        Ok(entries)
    }

    /// Return the min/max transaction numbers that have recorded mempool arrivals.
    pub fn tx_arrival_tx_number_bounds(&self) -> Result<Option<(u64, u64)>> {
        let entries = self.tx_arrival_entries()?;
        if entries.is_empty() {
            return Ok(None);
        }
        let first = entries[0].0;
        let last = entries[entries.len() - 1].0;
        Ok(Some((first, last)))
    }
}

fn reth_index_geometry_from_env(path: &Path) -> Geometry<RangeInclusive<usize>> {
    let map_size = configured_map_size(path);
    let growth_step = configured_usize_env(
        PYRETH_INDEX_DB_GROWTH_STEP_BYTES_ENV,
        DEFAULT_RETH_INDEX_DB_GROWTH_STEP_BYTES,
        MIN_RETH_INDEX_DB_GROWTH_STEP_BYTES,
    );

    reth_index_geometry(map_size, growth_step)
}

fn configured_map_size(path: &Path) -> usize {
    configured_usize_env_with_default(
        PYRETH_INDEX_DB_MAP_SIZE_BYTES_ENV,
        default_map_size_for_path(path),
        MIN_RETH_INDEX_DB_MAP_SIZE_BYTES,
    )
}

fn default_map_size_for_path(path: &Path) -> usize {
    let base = DEFAULT_NEW_RETH_INDEX_DB_MAP_SIZE_BYTES;
    let target = reth_index_data_file_size(path)
        .map(|size| size.saturating_add(DEFAULT_EXISTING_RETH_INDEX_DB_MAP_HEADROOM_BYTES))
        .unwrap_or(base)
        .max(base);
    round_up_to_multiple(target, GIB)
}

fn reth_index_data_file_size(path: &Path) -> Option<usize> {
    ["mdbx.dat", "data.mdb"]
        .into_iter()
        .filter_map(|name| {
            let len = std::fs::metadata(path.join(name)).ok()?.len();
            usize::try_from(len).ok()
        })
        .max()
}

fn reth_index_geometry(map_size: usize, growth_step: usize) -> Geometry<RangeInclusive<usize>> {
    Geometry {
        size: Some(MIN_RETH_INDEX_DB_MAP_SIZE_BYTES..=map_size),
        growth_step: Some(usize_to_isize_saturating(growth_step)),
        shrink_threshold: None,
        page_size: None,
    }
}

fn configured_usize_env(key: &str, default: usize, min: usize) -> usize {
    configured_usize_env_with_default(key, default, min)
}

fn configured_usize_env_with_default(key: &str, default: usize, min: usize) -> usize {
    let Ok(raw) = std::env::var(key) else {
        return default;
    };
    match parse_usize_config(&raw) {
        Some(value) if value >= min => value,
        Some(value) => {
            warn!(
                key,
                value,
                min,
                default,
                "Configured RethIndex MDBX size is below minimum; using default"
            );
            default
        }
        None => {
            warn!(
                key,
                value = %raw,
                default,
                "Invalid RethIndex MDBX size config; using default"
            );
            default
        }
    }
}

fn parse_usize_config(value: &str) -> Option<usize> {
    let normalized = value.trim().replace('_', "");
    if normalized.is_empty() {
        return None;
    }
    normalized.parse::<usize>().ok()
}

fn usize_to_isize_saturating(value: usize) -> isize {
    value.min(isize::MAX as usize) as isize
}

fn round_up_to_multiple(value: usize, multiple: usize) -> usize {
    if multiple == 0 {
        return value;
    }
    let remainder = value % multiple;
    if remainder == 0 {
        value
    } else {
        value.saturating_add(multiple - remainder)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_size_config_with_underscores() {
        assert_eq!(parse_usize_config("1_073_741_824"), Some(1_073_741_824));
        assert_eq!(parse_usize_config(" 4096 "), Some(4096));
        assert_eq!(parse_usize_config(""), None);
        assert_eq!(parse_usize_config("1GiB"), None);
    }

    #[test]
    fn new_index_default_map_size_is_bounded() {
        let temp = test_temp_dir("new_index_default_map_size_is_bounded");
        std::fs::create_dir_all(&temp).expect("create tempdir");
        assert_eq!(
            default_map_size_for_path(&temp),
            DEFAULT_NEW_RETH_INDEX_DB_MAP_SIZE_BYTES
        );
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn existing_index_default_keeps_bounded_headroom() {
        let temp = test_temp_dir("existing_index_default_keeps_bounded_headroom");
        std::fs::create_dir_all(&temp).expect("create tempdir");
        std::fs::write(temp.join("mdbx.dat"), []).expect("create mdbx file");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(temp.join("mdbx.dat"))
            .expect("open mdbx file");
        file.set_len((160 * GIB) as u64).expect("size mdbx file");

        assert_eq!(default_map_size_for_path(&temp), 192 * GIB);
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn geometry_uses_explicit_large_upper_bound() {
        let geometry = reth_index_geometry(
            DEFAULT_NEW_RETH_INDEX_DB_MAP_SIZE_BYTES,
            DEFAULT_RETH_INDEX_DB_GROWTH_STEP_BYTES,
        );
        let range = geometry.size.expect("geometry size range");
        assert_eq!(*range.start(), MIN_RETH_INDEX_DB_MAP_SIZE_BYTES);
        assert_eq!(*range.end(), DEFAULT_NEW_RETH_INDEX_DB_MAP_SIZE_BYTES);
        assert_eq!(
            geometry.growth_step,
            Some(DEFAULT_RETH_INDEX_DB_GROWTH_STEP_BYTES as isize)
        );
    }

    fn test_temp_dir(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("reth_index_{name}_{unique}"))
    }
}
