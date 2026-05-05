use alloy_primitives::U256;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub type DecimalAmount = Decimal;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Amount {
    pub raw: U256,
    pub decimals: u8,
}

impl Amount {
    pub const fn zero(decimals: u8) -> Self {
        Self {
            raw: U256::ZERO,
            decimals,
        }
    }
}
