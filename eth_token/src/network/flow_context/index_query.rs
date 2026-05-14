//! Address block participation query abstraction.

use std::collections::{BTreeMap, BTreeSet};

use eyre::Result;

use super::model::BlockRange;

/// Query interface over an address-block participation index.
pub trait AddressParticipationQuery {
    fn blocks_for_address(&self, address: &str, range: BlockRange) -> Result<Vec<u64>>;
}

/// Test/dry-run implementation backed by memory.
#[derive(Clone, Debug, Default)]
pub struct InMemoryAddressParticipation {
    blocks_by_address: BTreeMap<String, BTreeSet<u64>>,
}

impl InMemoryAddressParticipation {
    pub fn insert_blocks(
        &mut self,
        address: impl AsRef<str>,
        blocks: impl IntoIterator<Item = u64>,
    ) {
        let entry = self
            .blocks_by_address
            .entry(address.as_ref().trim().to_ascii_lowercase())
            .or_default();
        entry.extend(blocks);
    }
}

impl AddressParticipationQuery for InMemoryAddressParticipation {
    fn blocks_for_address(&self, address: &str, range: BlockRange) -> Result<Vec<u64>> {
        let address = address.trim().to_ascii_lowercase();
        let blocks = self
            .blocks_by_address
            .get(&address)
            .map(|blocks| {
                blocks
                    .iter()
                    .copied()
                    .filter(|block| range.contains(*block))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(blocks)
    }
}
