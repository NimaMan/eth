use serde::{Deserialize, Serialize};

use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};

pub const CURVE_V1_PROTOCOL: &str = "CURVE-V1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoolToken {
    pub symbol: Option<String>,
    pub address: String,
    pub decimals: u8,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePool {
    pub base: BasePool,
    pub name: Option<String>,
    pub lp_token_address: Option<String>,
    pub base_token_index: usize,
    pub quote_token_index: usize,
    pub tokens: Vec<CurvePoolToken>,
}

impl CurvePool {
    pub fn new(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        config: BasePoolConfig,
        name: Option<String>,
        lp_token_address: Option<String>,
        base_token_index: usize,
        quote_token_index: usize,
        tokens: Vec<CurvePoolToken>,
    ) -> Self {
        let identity = PoolIdentity::new(
            pool_address,
            token_address,
            denom_address,
            CURVE_V1_PROTOCOL,
        );
        Self {
            base: BasePool::new(identity, config),
            name,
            lp_token_address: lp_token_address.map(normalize_address_string),
            base_token_index,
            quote_token_index,
            tokens: tokens
                .into_iter()
                .map(|mut token| {
                    token.address = normalize_address_string(token.address);
                    token
                })
                .collect(),
        }
    }
}

fn normalize_address_string(value: impl Into<String>) -> String {
    value.into().trim().to_ascii_lowercase()
}
