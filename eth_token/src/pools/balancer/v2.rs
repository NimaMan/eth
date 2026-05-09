use serde::{Deserialize, Serialize};

use crate::pools::base::{BasePool, BasePoolConfig, PoolIdentity};

pub const BALANCER_V2_PROTOCOL: &str = "BALANCER-V2";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BalancerPoolToken {
    pub symbol: Option<String>,
    pub address: String,
    pub decimals: u8,
    pub index: usize,
    pub weight: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BalancerPool {
    pub base: BasePool,
    pub pool_id: String,
    pub vault_address: String,
    pub swap_fee_bps: Option<u32>,
    pub tokens: Vec<BalancerPoolToken>,
}

impl BalancerPool {
    pub fn new(
        pool_address: impl Into<String>,
        token_address: impl Into<String>,
        denom_address: impl Into<String>,
        config: BasePoolConfig,
        pool_id: impl Into<String>,
        vault_address: impl Into<String>,
        swap_fee_bps: Option<u32>,
        tokens: Vec<BalancerPoolToken>,
    ) -> Self {
        let identity = PoolIdentity::new(
            pool_address,
            token_address,
            denom_address,
            BALANCER_V2_PROTOCOL,
        );
        Self {
            base: BasePool::new(identity, config),
            pool_id: normalize_hash_string(pool_id),
            vault_address: normalize_address_string(vault_address),
            swap_fee_bps,
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

fn normalize_hash_string(value: impl Into<String>) -> String {
    let value = value.into().trim().to_ascii_lowercase();
    if value.starts_with("0x") {
        value
    } else {
        format!("0x{value}")
    }
}
