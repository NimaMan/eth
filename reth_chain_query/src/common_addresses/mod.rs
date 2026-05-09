//! Common addresses module
//!
//! This module contains all common Ethereum addresses used throughout the system.
//! All files are auto-generated from Python address files.

pub mod address_book;
pub mod cex;
pub mod denom_tokens;
pub mod dex_token_denom_pairs;
pub mod etf;
pub mod pool_addresses;
pub mod stablecoins;
pub mod validators;
pub mod wallets;

// Re-export commonly used items
pub use crate::dex::pool_types::{DEFAULT_POOL_TYPE, DEX_POOL_TYPES};
pub use address_book::{get_address_by_name, ADDRESSES_BY_NAME};
pub use cex::{ADDRESSES_BY_EXCHANGE, CEX_ADDRESSES, CEX_ADDRESS_SET};
pub use denom_tokens::{get_token_decimals, get_token_symbol, is_denom_token};
pub use denom_tokens::{DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS};
pub use dex_token_denom_pairs::{
    balancer_pools, curve_pools, dai_usdc_pairs, eth_dai_pairs, eth_usdc_pairs, eth_usdt_pairs,
    sushiswap_tokens, uniswap_v2_tokens, uniswap_v3_tokens, uniswap_v4_pools, usdc_usdt_pairs,
    BalancerPoolInfo, BalancerTokenInfo, CurvePoolInfo, CurvePoolTokenInfo, StablecoinPairSpec,
    SushiSwapTokenInfo, UniswapV2TokenInfo, UniswapV3TokenInfo, UniswapV4PoolInfo,
};
pub use etf::{ADDRESSES_BY_PROVIDER, ETF_ADDRESSES, ETF_ADDRESS_SET};
pub use pool_addresses::{
    get_pool_protocol, is_known_factory, is_v4_pool_manager, KnownV2Protocol,
    KnownV2ProtocolDescriptor, POOL_FACTORIES, ROUTERS,
};
pub use stablecoins::{STABLECOINS, STABLECOIN_BY_ADDRESS, STABLECOIN_BY_SYMBOL};
pub use validators::{is_bribe, is_fee_recipient, FEE_RECIPIENTS, FEE_RECIPIENT_LIST};
pub use wallets::WALLET_ADDRESSES;
