use crate::{ExpectedBehavior, TokenConfig};
use reth_chain_query::common_addresses::dex_token_denom_pairs::{
    uniswap_v2_tokens, UniswapV2TokenInfo,
};
use tx_processor::trade_simulation::PoolType;

pub fn token_configs() -> Vec<TokenConfig> {
    uniswap_v2_tokens()
        .iter()
        .map(|info| to_token_config(info))
        .collect()
}

fn to_token_config(info: &UniswapV2TokenInfo) -> TokenConfig {
    TokenConfig {
        symbol: info.symbol,
        token_address: info.token_address,
        denom_address: info.denom_address,
        pool_type: PoolType::UniswapV2,
        expected_behavior: expected_behavior(info.symbol),
        decimals: info.decimals,
    }
}

fn expected_behavior(symbol: &str) -> ExpectedBehavior {
    match symbol {
        "PEPE" | "SHIB" | "DOGE" | "FLOKI" | "FTT" | "BONE" | "ELON" | "AKITA" | "TORN"
        | "BABYDOGE" | "KISHU" => ExpectedBehavior::MayHaveTaxes,
        _ => ExpectedBehavior::ShouldWork,
    }
}
