//! Common addresses module
//!
//! This module contains all common Ethereum addresses used throughout the system.
//! All files are auto-generated from Python address files.

pub mod cex;
pub mod denom_tokens;
pub mod dex_pools;
pub mod dex_token_sets;
pub mod etf;
pub mod pool_types;
pub mod stablecoins;
pub mod validators;

// Re-export commonly used items
pub use cex::{ADDRESSES_BY_EXCHANGE, CEX_ADDRESSES, CEX_ADDRESS_SET};
pub use denom_tokens::{get_address_by_name, get_token_decimals, get_token_symbol, is_denom_token};
pub use denom_tokens::{ADDRESSES_BY_NAME, DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS};
pub use dex_pools::{
    compute_sushiswap_pool, compute_uniswap_v2_pool, compute_uniswap_v3_pool,
    find_uniswap_v4_pools_for_pair, get_all_v3_pools, V4PoolInfo, BALANCER_VAULT,
    SUSHISWAP_FACTORY, UNISWAP_V2_FACTORY, UNISWAP_V3_FACTORY, V3_FEE_TIERS,
};
pub use dex_token_sets::{uniswap_v2_tokens, uniswap_v3_tokens, UniswapV2TokenInfo, UniswapV3TokenInfo};
pub use etf::{ADDRESSES_BY_PROVIDER, ETF_ADDRESSES, ETF_ADDRESS_SET};
pub use pool_types::{DEFAULT_POOL_TYPE, DEX_POOL_TYPES};
pub use stablecoins::{STABLECOINS, STABLECOIN_BY_ADDRESS, STABLECOIN_BY_SYMBOL};
pub use validators::{is_bribe, is_fee_recipient, FEE_RECIPIENTS, FEE_RECIPIENT_LIST};
