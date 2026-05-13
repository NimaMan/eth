//! Canonical known-address classification across the generated catalogs.

use alloy_primitives::Address;
use std::collections::HashMap;

use super::{
    address_book::ADDRESSES_BY_NAME,
    cex::get_cex_by_address,
    denom_tokens::DENOM_ADDRESSES,
    etf::get_etf_by_address,
    pool_addresses::{POOL_FACTORIES, ROUTERS},
    stablecoins::get_stablecoin_by_address,
    validators::get_fee_recipient_name,
    wallets::WALLET_ADDRESSES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownAddressKind {
    Stablecoin,
    DenomToken,
    Cex,
    Etf,
    FeeRecipient,
    DexFactory,
    DexRouter,
    Wallet,
    Named,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownAddress {
    pub address: Address,
    pub kind: KnownAddressKind,
    pub name: &'static str,
    pub group: Option<&'static str>,
}

pub fn identify_known_address(address: Address) -> Option<KnownAddress> {
    if let Some(info) = get_stablecoin_by_address(address) {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::Stablecoin,
            name: info.symbol,
            group: Some(info.unit),
        });
    }

    if let Some(symbol) = DENOM_ADDRESSES.get(&address).copied() {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::DenomToken,
            name: symbol,
            group: None,
        });
    }

    if let Some(entry) = get_cex_by_address(address) {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::Cex,
            name: entry.name,
            group: Some(entry.exchange),
        });
    }

    if let Some(entry) = get_etf_by_address(address) {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::Etf,
            name: entry.name,
            group: Some(entry.provider),
        });
    }

    if let Some(name) = get_fee_recipient_name(address) {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::FeeRecipient,
            name,
            group: None,
        });
    }

    if let Some(name) = find_name_for_address(&POOL_FACTORIES, address) {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::DexFactory,
            name,
            group: None,
        });
    }

    if let Some(name) = find_name_for_address(&ROUTERS, address) {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::DexRouter,
            name,
            group: None,
        });
    }

    if let Some(name) = find_name_for_address(&WALLET_ADDRESSES, address) {
        return Some(KnownAddress {
            address,
            kind: KnownAddressKind::Wallet,
            name,
            group: None,
        });
    }

    find_name_for_address(&ADDRESSES_BY_NAME, address).map(|name| KnownAddress {
        address,
        kind: KnownAddressKind::Named,
        name,
        group: None,
    })
}

pub fn is_known_address(address: Address) -> bool {
    identify_known_address(address).is_some()
}

fn find_name_for_address(
    map: &HashMap<&'static str, Address>,
    address: Address,
) -> Option<&'static str> {
    map.iter()
        .find_map(|(name, candidate)| (*candidate == address).then_some(*name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;

    #[test]
    fn identifies_stablecoin_before_generic_denom_token() {
        let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        let known = identify_known_address(usdc).unwrap();

        assert_eq!(known.kind, KnownAddressKind::Stablecoin);
        assert_eq!(known.name, "USDC");
        assert_eq!(known.group, Some("US Dollar"));
    }

    #[test]
    fn identifies_dex_router() {
        let router = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
        let known = identify_known_address(router).unwrap();

        assert_eq!(known.kind, KnownAddressKind::DexRouter);
        assert_eq!(known.name, "univ2_router");
    }

    #[test]
    fn identifies_cex_and_etf_catalog_entries() {
        let cex = address!("5c89724967D76a4f966b013140355789093F6c7D");
        let cex_known = identify_known_address(cex).unwrap();
        assert_eq!(cex_known.kind, KnownAddressKind::Cex);
        assert_eq!(cex_known.group, Some("1xBet"));

        let etf = address!("1ae3adaB1c43f97D53Ee3619CD8220C294059Dec");
        let etf_known = identify_known_address(etf).unwrap();
        assert_eq!(etf_known.kind, KnownAddressKind::Etf);
        assert_eq!(etf_known.group, Some("21Shares"));
    }
}
