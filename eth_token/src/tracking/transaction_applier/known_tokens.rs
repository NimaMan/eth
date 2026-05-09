use alloy_primitives::Address;
use reth_chain_query::common_addresses::{get_token_decimals, get_token_symbol};

pub(super) fn known_decimals_for_address(address: Address) -> Option<u8> {
    get_token_symbol(address).and_then(get_token_decimals)
}

pub(super) fn known_decimals_for_address_or_native(address: Address) -> Option<u8> {
    if address.is_zero() {
        Some(18)
    } else {
        known_decimals_for_address(address)
    }
}
