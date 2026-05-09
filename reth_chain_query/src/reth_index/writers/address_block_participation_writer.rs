use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use alloy_primitives::Address;
use eyre::Result;

use crate::reth_index::tables::address_block_participation::ParticipationBlockNumber;
use crate::reth_index::RethIndexDB;

/// Addresses touched by a transaction within a block.
pub struct AddressParticipation {
    pub tx_index: u64,
    pub addresses: Vec<Address>,
}

/// Writer that persists address -> block participation into the RethIndex DB.
#[derive(Clone)]
pub struct AddressBlockParticipationWriter {
    db: Arc<RethIndexDB>,
}

struct BlockEntries {
    entries: Vec<(Address, Vec<ParticipationBlockNumber>)>,
}

impl AddressBlockParticipationWriter {
    pub fn new(db: Arc<RethIndexDB>) -> Self {
        Self { db }
    }

    /// Persist address participations for a single block.
    pub fn ingest_block_participation<I>(
        &self,
        block_number: u64,
        participations: I,
    ) -> Result<usize>
    where
        I: IntoIterator<Item = AddressParticipation>,
    {
        let entries = self.prepare_block_entries(block_number, participations)?;
        if entries.entries.is_empty() {
            return Ok(0);
        }
        self.db
            .append_address_participation_blocks_batch(&entries.entries)
    }

    /// Persist address participations for multiple blocks in one MDBX transaction.
    pub fn ingest_block_participation_batch(
        &self,
        blocks: Vec<(u64, Vec<AddressParticipation>)>,
    ) -> Result<Vec<usize>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let mut prepared = Vec::with_capacity(blocks.len());
        for (block_number, participations) in blocks {
            prepared.push(self.prepare_block_entries(block_number, participations)?);
        }

        self.write_block_entries_batch(prepared)
    }

    fn prepare_block_entries<I>(&self, block_number: u64, participations: I) -> Result<BlockEntries>
    where
        I: IntoIterator<Item = AddressParticipation>,
    {
        let mut addresses = BTreeSet::new();
        for participation in participations {
            let AddressParticipation {
                tx_index: _,
                addresses: tx_addresses,
            } = participation;
            let mut tx_dedup = HashSet::with_capacity(tx_addresses.len());
            for address in tx_addresses {
                if tx_dedup.insert(address) {
                    addresses.insert(address);
                }
            }
        }

        let entries = addresses
            .into_iter()
            .map(|address| (address, vec![block_number]))
            .collect::<Vec<_>>();

        Ok(BlockEntries { entries })
    }

    fn write_block_entries_batch(&self, blocks: Vec<BlockEntries>) -> Result<Vec<usize>> {
        if blocks.is_empty() {
            return Ok(Vec::new());
        }

        let block_refs = blocks
            .iter()
            .map(|block| block.entries.as_slice())
            .collect::<Vec<_>>();
        self.db
            .append_address_participation_blocks_by_block(&block_refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempIndexDir(PathBuf);

    impl TempIndexDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("reth-index-{name}-{}-{unique}", std::process::id()));
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempIndexDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn single_block_deduplicates_addresses() {
        let temp = TempIndexDir::new("address-block-single");
        let db = Arc::new(RethIndexDB::open(temp.path()).unwrap());
        let writer = AddressBlockParticipationWriter::new(db.clone());

        let a = Address::repeat_byte(0x11);
        let b = Address::repeat_byte(0x22);
        let c = Address::repeat_byte(0x33);

        let participations = vec![
            AddressParticipation {
                tx_index: 0,
                addresses: vec![a, a, b],
            },
            AddressParticipation {
                tx_index: 1,
                addresses: vec![a, c],
            },
        ];

        let inserted = writer
            .ingest_block_participation(42, participations)
            .unwrap();
        assert_eq!(inserted, 3);
        assert_eq!(db.get_address_participation_blocks(a).unwrap(), vec![42]);
        assert_eq!(db.get_address_participation_blocks(b).unwrap(), vec![42]);
        assert_eq!(db.get_address_participation_blocks(c).unwrap(), vec![42]);

        let inserted = writer
            .ingest_block_participation(
                42,
                vec![AddressParticipation {
                    tx_index: 0,
                    addresses: vec![a, b, c],
                }],
            )
            .unwrap();
        assert_eq!(inserted, 0);
    }

    #[test]
    fn batch_write_keeps_blocks_sorted_per_address() {
        let temp = TempIndexDir::new("address-block-batch");
        let db = Arc::new(RethIndexDB::open(temp.path()).unwrap());
        let writer = AddressBlockParticipationWriter::new(db.clone());

        let a = Address::repeat_byte(0xaa);
        let b = Address::repeat_byte(0xbb);

        let inserted = writer
            .ingest_block_participation_batch(vec![
                (
                    10,
                    vec![AddressParticipation {
                        tx_index: 0,
                        addresses: vec![a, b],
                    }],
                ),
                (
                    11,
                    vec![AddressParticipation {
                        tx_index: 0,
                        addresses: vec![a],
                    }],
                ),
                (
                    12,
                    vec![AddressParticipation {
                        tx_index: 0,
                        addresses: vec![b, a],
                    }],
                ),
            ])
            .unwrap();

        assert_eq!(inserted, vec![2, 1, 2]);
        assert_eq!(
            db.get_address_participation_blocks(a).unwrap(),
            vec![10, 11, 12]
        );
        assert_eq!(
            db.get_address_participation_blocks(b).unwrap(),
            vec![10, 12]
        );
    }
}
