//! External known-address database integration.

use alloy_primitives::Address;
use std::collections::BTreeMap;

/// A known-address entry from an external database.
#[derive(Clone, Debug)]
pub struct KnownAddressEntry {
    pub address: Address,
    pub label: String,
    pub entity: Option<String>,
    pub source: String,
}

/// Local known-address book.
#[derive(Clone, Debug, Default)]
pub struct KnownAddressBook {
    pub entries: BTreeMap<Address, KnownAddressEntry>,
}

impl KnownAddressBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_from_json(_path: &str) -> eyre::Result<Self> {
        // TODO: Load from local JSON file
        Ok(Self::new())
    }

    pub fn label_for(&self, address: Address) -> Option<&KnownAddressEntry> {
        self.entries.get(&address)
    }
}
