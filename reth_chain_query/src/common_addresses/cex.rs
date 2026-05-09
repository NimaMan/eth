//! CEX (Centralized Exchange) addresses
//!
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::Address;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// CEX address entry
#[derive(Debug, Clone)]
pub struct CexAddress {
    pub address: Address,
    pub name: &'static str,
    pub exchange: &'static str,
}

/// All CEX addresses
mod part_000;
mod part_001;
mod part_002;
mod part_003;
mod part_004;
mod part_005;
mod part_006;
mod part_007;
mod part_008;
mod part_009;
mod part_010;
mod part_011;
mod part_012;
mod part_013;
mod part_014;
mod part_015;

/// All CEX addresses
pub static CEX_ADDRESSES: Lazy<Vec<CexAddress>> = Lazy::new(|| {
    let mut addresses = Vec::new();
    addresses.extend_from_slice(part_000::ADDRESSES);
    addresses.extend_from_slice(part_001::ADDRESSES);
    addresses.extend_from_slice(part_002::ADDRESSES);
    addresses.extend_from_slice(part_003::ADDRESSES);
    addresses.extend_from_slice(part_004::ADDRESSES);
    addresses.extend_from_slice(part_005::ADDRESSES);
    addresses.extend_from_slice(part_006::ADDRESSES);
    addresses.extend_from_slice(part_007::ADDRESSES);
    addresses.extend_from_slice(part_008::ADDRESSES);
    addresses.extend_from_slice(part_009::ADDRESSES);
    addresses.extend_from_slice(part_010::ADDRESSES);
    addresses.extend_from_slice(part_011::ADDRESSES);
    addresses.extend_from_slice(part_012::ADDRESSES);
    addresses.extend_from_slice(part_013::ADDRESSES);
    addresses.extend_from_slice(part_014::ADDRESSES);
    addresses.extend_from_slice(part_015::ADDRESSES);
    addresses
});

/// Total number of CEX addresses
pub const CEX_ADDRESS_COUNT: usize = 2776;

/// Lazy static HashSet for quick lookups
pub static CEX_ADDRESS_SET: Lazy<HashSet<Address>> =
    Lazy::new(|| CEX_ADDRESSES.iter().map(|entry| entry.address).collect());

/// Lazy static HashMap for address to exchange mapping
pub static CEX_BY_ADDRESS: Lazy<HashMap<Address, &'static CexAddress>> = Lazy::new(|| {
    CEX_ADDRESSES
        .iter()
        .map(|entry| (entry.address, entry))
        .collect()
});

/// Group addresses by exchange
pub static ADDRESSES_BY_EXCHANGE: Lazy<HashMap<&'static str, Vec<Address>>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, Vec<Address>> = HashMap::new();
    for entry in CEX_ADDRESSES.iter() {
        map.entry(entry.exchange)
            .or_insert_with(Vec::new)
            .push(entry.address);
    }
    map
});

/// Check if an address belongs to a CEX
pub fn is_cex_address(address: Address) -> bool {
    CEX_ADDRESS_SET.contains(&address)
}

/// Get CEX info by address
pub fn get_cex_by_address(address: Address) -> Option<&'static CexAddress> {
    CEX_BY_ADDRESS.get(&address).copied()
}

/// Get all addresses for a specific exchange
pub fn get_exchange_addresses(exchange: &str) -> Option<&'static Vec<Address>> {
    ADDRESSES_BY_EXCHANGE.get(exchange)
}

/// Exchange counts:
/// - OKX: 165 addresses
/// - Coinbase: 159 addresses
/// - HTX: 155 addresses
/// - Binance: 115 addresses
/// - Kraken: 76 addresses
/// - HitBTC: 72 addresses
/// - KuCoin: 58 addresses
/// - CoinDCX: 52 addresses
/// - ShapeShift: 50 addresses
/// - Bidesk: 48 addresses
/// Exchange names with counts
pub fn exchange_stats() -> Vec<(&'static str, usize)> {
    let mut stats: Vec<_> = ADDRESSES_BY_EXCHANGE
        .iter()
        .map(|(name, addrs)| (*name, addrs.len()))
        .collect();
    stats.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    stats
}
