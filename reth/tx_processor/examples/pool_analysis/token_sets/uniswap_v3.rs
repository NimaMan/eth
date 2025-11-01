use crate::{ExpectedBehavior, TokenConfig};
use reth_chain_query::common_addresses::dex_token_sets::{uniswap_v3_tokens, UniswapV3TokenInfo};

pub fn token_configs() -> Vec<TokenConfig> {
    uniswap_v3_tokens()
        .iter()
        .map(|info| to_token_config(info))
        .collect()
}

fn to_token_config(info: &UniswapV3TokenInfo) -> TokenConfig {
    TokenConfig {
        symbol: info.symbol,
        token_address: info.token_address,
        denom_address: info.denom_address,
        fee_tier: info.fee_tier,
        expected_behavior: expected_behavior(info.symbol),
        decimals: info.decimals,
    }
}

fn expected_behavior(symbol: &str) -> ExpectedBehavior {
    match symbol {
        "FLOKI" => ExpectedBehavior::MightFail("May have taxes"),
        _ => ExpectedBehavior::ShouldWork,
    }
}
