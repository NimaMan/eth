//! ETF (Exchange-Traded Fund) addresses
//!
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::Address;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// ETF address entry
#[derive(Debug, Clone)]
pub struct EtfAddress {
    pub address: Address,
    pub name: &'static str,
    pub provider: &'static str,
}

/// All ETF addresses
mod part_000;
mod part_001;
mod part_002;
mod part_003;
mod part_004;
mod part_005;
mod part_006;

/// All ETF addresses
pub static ETF_ADDRESSES: Lazy<Vec<EtfAddress>> = Lazy::new(|| {
    let mut addresses = Vec::new();
    addresses.extend_from_slice(part_000::ADDRESSES);
    addresses.extend_from_slice(part_001::ADDRESSES);
    addresses.extend_from_slice(part_002::ADDRESSES);
    addresses.extend_from_slice(part_003::ADDRESSES);
    addresses.extend_from_slice(part_004::ADDRESSES);
    addresses.extend_from_slice(part_005::ADDRESSES);
    addresses.extend_from_slice(part_006::ADDRESSES);
    addresses
});

/// Total number of ETF addresses
pub const ETF_ADDRESS_COUNT: usize = 1151;

/// Lazy static HashSet for quick lookups
pub static ETF_ADDRESS_SET: Lazy<HashSet<Address>> =
    Lazy::new(|| ETF_ADDRESSES.iter().map(|entry| entry.address).collect());

/// Lazy static HashMap for address to ETF mapping
pub static ETF_BY_ADDRESS: Lazy<HashMap<Address, &'static EtfAddress>> = Lazy::new(|| {
    ETF_ADDRESSES
        .iter()
        .map(|entry| (entry.address, entry))
        .collect()
});

/// Group addresses by provider
pub static ADDRESSES_BY_PROVIDER: Lazy<HashMap<&'static str, Vec<Address>>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, Vec<Address>> = HashMap::new();
    for entry in ETF_ADDRESSES.iter() {
        map.entry(entry.provider)
            .or_insert_with(Vec::new)
            .push(entry.address);
    }
    map
});

/// Check if an address belongs to an ETF
pub fn is_etf_address(address: Address) -> bool {
    ETF_ADDRESS_SET.contains(&address)
}

/// Get ETF info by address
pub fn get_etf_by_address(address: Address) -> Option<&'static EtfAddress> {
    ETF_BY_ADDRESS.get(&address).copied()
}

/// Get all addresses for a specific provider
pub fn get_provider_addresses(provider: &str) -> Option<&'static Vec<Address>> {
    ADDRESSES_BY_PROVIDER.get(provider)
}

/// Provider counts:
/// - Grayscale: 978 addresses
/// - BlackRock: 141 addresses
/// - Fidelity: 10 addresses
/// - 21Shares: 8 addresses
/// - Bitwise: 6 addresses
/// - Invesco: 4 addresses
/// - Franklin: 2 addresses
/// - VanEck: 2 addresses
/// Provider names with counts
pub fn provider_stats() -> Vec<(&'static str, usize)> {
    let mut stats: Vec<_> = ADDRESSES_BY_PROVIDER
        .iter()
        .map(|(name, addrs)| (*name, addrs.len()))
        .collect();
    stats.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    stats
}
