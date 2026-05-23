//! Canonical burn/dead addresses shared by ETH analytics.

use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::{collections::HashMap, str::FromStr};

pub const ZERO_ADDRESS: Address = address!("0000000000000000000000000000000000000000");
pub const DEAD_ADDRESS: Address = address!("000000000000000000000000000000000000dEaD");

pub static BURN_ADDRESSES: Lazy<HashMap<Address, &'static str>> = Lazy::new(|| {
    let mut addresses = HashMap::new();
    addresses.insert(ZERO_ADDRESS, "zero_address");
    addresses.insert(DEAD_ADDRESS, "dead_address");
    addresses
});

pub fn get_burn_address_name(address: Address) -> Option<&'static str> {
    BURN_ADDRESSES.get(&address).copied()
}

pub fn is_burn_address(address: Address) -> bool {
    BURN_ADDRESSES.contains_key(&address)
}

pub fn is_burn_address_str(address: impl AsRef<str>) -> bool {
    Address::from_str(address.as_ref()).is_ok_and(is_burn_address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_zero_and_dead_addresses() {
        assert!(is_burn_address(ZERO_ADDRESS));
        assert!(is_burn_address(DEAD_ADDRESS));
        assert!(is_burn_address_str(
            "0x0000000000000000000000000000000000000000"
        ));
        assert!(is_burn_address_str(
            "0x000000000000000000000000000000000000dEaD"
        ));
    }
}
