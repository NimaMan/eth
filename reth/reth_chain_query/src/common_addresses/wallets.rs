//! Named wallet addresses used across simulations and examples.
//!
//! This file centralises externally owned account (EOA) addresses so examples such as the
//! buy/sell simulator can look them up via `get_address_by_name`.

use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Wallet addresses keyed by human-friendly label.
pub static WALLET_ADDRESSES: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "Nima_eth_1",
        address!("0C96c602b1b332B8AB2093E5d72D804a24bd5689"),
    );
    m
});
