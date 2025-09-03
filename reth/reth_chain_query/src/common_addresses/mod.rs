//! Common addresses module
//! 
//! This module contains all common Ethereum addresses used throughout the system.
//! All files are auto-generated from Python address files.

pub mod denom_tokens;
pub mod stablecoins;
pub mod cex;
pub mod etf;
pub mod validators;

// Re-export commonly used items
pub use denom_tokens::{DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS, ADDRESSES_BY_NAME};
pub use denom_tokens::{get_token_symbol, get_token_decimals, is_denom_token, get_address_by_name};
pub use stablecoins::{STABLECOINS, STABLECOIN_BY_ADDRESS, STABLECOIN_BY_SYMBOL};
pub use cex::{CEX_ADDRESSES, CEX_ADDRESS_SET, ADDRESSES_BY_EXCHANGE};
pub use etf::{ETF_ADDRESSES, ETF_ADDRESS_SET, ADDRESSES_BY_PROVIDER};
pub use validators::{FEE_RECIPIENTS, FEE_RECIPIENT_LIST, is_fee_recipient, is_bribe};
